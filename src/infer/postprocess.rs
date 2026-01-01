use std::cmp::Ordering;

use anyhow::{Context, Result};
use image::{GrayImage, Luma};

use crate::infer::preprocess::LetterboxInfo;
use crate::infer::{BBox, Keypoint, RawOutput, SegRawOutput};

#[derive(Copy, Clone)]
enum OutputLayout {
    ChannelsByDet,
    DetectionsByChannel,
}

pub(crate) struct PoseDetection {
    pub(crate) bbox: BBox,
    pub(crate) score: f32,
    pub(crate) keypoints: Vec<Keypoint>,
}

pub(crate) struct SegDetection {
    pub(crate) bbox: BBox,
    pub(crate) bbox_input: BBox,
    pub(crate) score: f32,
    pub(crate) coeffs: Vec<f32>,
}

pub(crate) fn decode_pose(
    output: &RawOutput,
    letterbox: &LetterboxInfo,
    score_threshold: f32,
    min_kpt_conf: f32,
    min_kpt_count: usize,
) -> Result<Vec<PoseDetection>> {
    let (channels, detections, layout) =
        parse_layout(&output.dims).context("unsupported pose output shape")?;
    if channels <= 5 {
        return Ok(Vec::new());
    }

    let kpt_count = (channels - 5) / 3;
    let mut detections_out = Vec::new();

    for det_index in 0..detections {
        let score = get_value(&output.data, layout, det_index, 4, channels, detections);
        if score < score_threshold {
            continue;
        }

        let (_, bbox) = decode_bbox(
            &output.data,
            layout,
            det_index,
            channels,
            detections,
            letterbox,
        );

        let mut keypoints = Vec::with_capacity(kpt_count);
        for k in 0..kpt_count {
            let base = 5 + k * 3;
            let x = get_value(&output.data, layout, det_index, base, channels, detections);
            let y = get_value(
                &output.data,
                layout,
                det_index,
                base + 1,
                channels,
                detections,
            );
            let kp_score = get_value(
                &output.data,
                layout,
                det_index,
                base + 2,
                channels,
                detections,
            );
            let (orig_x, orig_y) = letterbox.to_original(x, y);
            keypoints.push(Keypoint {
                x: orig_x,
                y: orig_y,
                score: kp_score,
            });
        }

        let valid_kpts = keypoints
            .iter()
            .filter(|kp| kp.score >= min_kpt_conf)
            .count();
        if valid_kpts < min_kpt_count {
            continue;
        }

        detections_out.push(PoseDetection {
            bbox,
            score,
            keypoints,
        });
    }

    Ok(detections_out)
}

pub(crate) fn decode_seg(
    output: &SegRawOutput,
    letterbox: &LetterboxInfo,
    person_class: usize,
    score_threshold: f32,
) -> Result<Vec<SegDetection>> {
    let (channels, detections, layout) =
        parse_layout(&output.dets.dims).context("unsupported seg output shape")?;

    let proto_dims = &output.proto.dims;
    if proto_dims.len() != 4 {
        return Err(anyhow::anyhow!("unexpected proto dims: {:?}", proto_dims));
    }
    let mask_dim = proto_dims[1];
    if channels <= 4 + mask_dim {
        return Err(anyhow::anyhow!(
            "seg output channels too small: channels={channels} mask_dim={mask_dim}"
        ));
    }
    let class_count = channels - 4 - mask_dim;
    if person_class >= class_count {
        return Err(anyhow::anyhow!(
            "person class id out of range: class_count={class_count} person_class={person_class}"
        ));
    }

    let mut detections_out = Vec::new();
    let class_offset = 4 + person_class;
    let coeff_offset = 4 + class_count;

    for det_index in 0..detections {
        let score = get_value(
            &output.dets.data,
            layout,
            det_index,
            class_offset,
            channels,
            detections,
        );
        if score < score_threshold {
            continue;
        }

        let (bbox_input, bbox) = decode_bbox(
            &output.dets.data,
            layout,
            det_index,
            channels,
            detections,
            letterbox,
        );

        let mut coeffs = Vec::with_capacity(mask_dim);
        for k in 0..mask_dim {
            let coeff = get_value(
                &output.dets.data,
                layout,
                det_index,
                coeff_offset + k,
                channels,
                detections,
            );
            coeffs.push(coeff);
        }

        detections_out.push(SegDetection {
            bbox,
            bbox_input,
            score,
            coeffs,
        });
    }

    Ok(detections_out)
}

pub(crate) fn build_mask(
    proto: &RawOutput,
    coeffs: &[f32],
    bbox_input: &BBox,
    letterbox: &LetterboxInfo,
) -> Result<Vec<f32>> {
    let dims = &proto.dims;
    if dims.len() != 4 {
        return Err(anyhow::anyhow!("unexpected proto dims: {:?}", dims));
    }
    let mask_dim = dims[1];
    let mask_h = dims[2];
    let mask_w = dims[3];
    if coeffs.len() != mask_dim {
        return Err(anyhow::anyhow!(
            "mask coeff size mismatch: coeffs={} mask_dim={mask_dim}",
            coeffs.len()
        ));
    }

    let mut mask_values = vec![0.0f32; mask_h * mask_w];
    for y in 0..mask_h {
        for x in 0..mask_w {
            let mut sum = 0.0f32;
            for k in 0..mask_dim {
                let idx = (k * mask_h + y) * mask_w + x;
                sum += coeffs[k] * proto.data[idx];
            }
            mask_values[y * mask_w + x] = sigmoid(sum);
        }
    }

    let mut proto_img = GrayImage::new(mask_w as u32, mask_h as u32);
    for y in 0..mask_h {
        for x in 0..mask_w {
            let value = (mask_values[y * mask_w + x] * 255.0).clamp(0.0, 255.0) as u8;
            proto_img.put_pixel(x as u32, y as u32, Luma([value]));
        }
    }

    let input_size = letterbox.input_size;
    let resized = image::imageops::resize(
        &proto_img,
        input_size,
        input_size,
        image::imageops::FilterType::Triangle,
    );

    let mut input_mask = vec![0.0f32; (input_size * input_size) as usize];
    let x1 = bbox_input.x1.floor().clamp(0.0, (input_size - 1) as f32) as u32;
    let y1 = bbox_input.y1.floor().clamp(0.0, (input_size - 1) as f32) as u32;
    let x2 = bbox_input.x2.ceil().clamp(0.0, (input_size - 1) as f32) as u32;
    let y2 = bbox_input.y2.ceil().clamp(0.0, (input_size - 1) as f32) as u32;

    for y in y1..=y2 {
        for x in x1..=x2 {
            let value = resized.get_pixel(x, y)[0] as f32 / 255.0;
            let idx = (y * input_size + x) as usize;
            input_mask[idx] = value;
        }
    }

    let mut input_img = GrayImage::new(input_size, input_size);
    for y in 0..input_size {
        for x in 0..input_size {
            let idx = (y * input_size + x) as usize;
            let value = (input_mask[idx] * 255.0).clamp(0.0, 255.0) as u8;
            input_img.put_pixel(x, y, Luma([value]));
        }
    }

    let cropped = image::imageops::crop_imm(
        &input_img,
        letterbox.pad_x,
        letterbox.pad_y,
        letterbox.new_w,
        letterbox.new_h,
    )
    .to_image();
    let resized_orig = image::imageops::resize(
        &cropped,
        letterbox.orig_w,
        letterbox.orig_h,
        image::imageops::FilterType::Triangle,
    );

    let mut output_mask = vec![0.0f32; (letterbox.orig_w * letterbox.orig_h) as usize];
    for y in 0..letterbox.orig_h {
        for x in 0..letterbox.orig_w {
            let value = resized_orig.get_pixel(x, y)[0] as f32 / 255.0;
            let idx = (y * letterbox.orig_w + x) as usize;
            output_mask[idx] = value;
        }
    }

    Ok(output_mask)
}

pub(crate) fn compute_iou(a: &BBox, b: &BBox) -> f32 {
    let inter_x1 = a.x1.max(b.x1);
    let inter_y1 = a.y1.max(b.y1);
    let inter_x2 = a.x2.min(b.x2);
    let inter_y2 = a.y2.min(b.y2);

    let inter_w = (inter_x2 - inter_x1).max(0.0);
    let inter_h = (inter_y2 - inter_y1).max(0.0);
    let inter_area = inter_w * inter_h;

    let area_a = (a.x2 - a.x1).max(0.0) * (a.y2 - a.y1).max(0.0);
    let area_b = (b.x2 - b.x1).max(0.0) * (b.y2 - b.y1).max(0.0);
    let denom = area_a + area_b - inter_area;
    if denom <= 0.0 {
        return 0.0;
    }
    inter_area / denom
}

pub(crate) fn nms_pose(
    mut detections: Vec<PoseDetection>,
    iou_threshold: f32,
) -> Vec<PoseDetection> {
    detections.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));

    let mut kept: Vec<PoseDetection> = Vec::new();
    for det in detections {
        let overlaps = kept
            .iter()
            .any(|keep| compute_iou(&det.bbox, &keep.bbox) > iou_threshold);
        if overlaps {
            continue;
        }
        kept.push(det);
    }

    kept
}

pub(crate) fn nms_seg(mut detections: Vec<SegDetection>, iou_threshold: f32) -> Vec<SegDetection> {
    detections.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));

    let mut kept: Vec<SegDetection> = Vec::new();
    for det in detections {
        let overlaps = kept
            .iter()
            .any(|keep| compute_iou(&det.bbox, &keep.bbox) > iou_threshold);
        if overlaps {
            continue;
        }
        kept.push(det);
    }

    kept
}

fn parse_layout(dims: &[usize]) -> Option<(usize, usize, OutputLayout)> {
    match dims.len() {
        3 => {
            let a = dims[1];
            let b = dims[2];
            if a <= b {
                Some((a, b, OutputLayout::DetectionsByChannel))
            } else {
                Some((b, a, OutputLayout::ChannelsByDet))
            }
        }
        2 => {
            let a = dims[0];
            let b = dims[1];
            if a <= b {
                Some((a, b, OutputLayout::DetectionsByChannel))
            } else {
                Some((b, a, OutputLayout::ChannelsByDet))
            }
        }
        _ => None,
    }
}

fn get_value(
    data: &[f32],
    layout: OutputLayout,
    det_index: usize,
    channel: usize,
    channels: usize,
    detections: usize,
) -> f32 {
    let idx = match layout {
        OutputLayout::DetectionsByChannel => channel * detections + det_index,
        OutputLayout::ChannelsByDet => det_index * channels + channel,
    };

    data.get(idx).copied().unwrap_or(0.0)
}

fn decode_bbox(
    data: &[f32],
    layout: OutputLayout,
    det_index: usize,
    channels: usize,
    detections: usize,
    letterbox: &LetterboxInfo,
) -> (BBox, BBox) {
    let cx = get_value(data, layout, det_index, 0, channels, detections);
    let cy = get_value(data, layout, det_index, 1, channels, detections);
    let w = get_value(data, layout, det_index, 2, channels, detections);
    let h = get_value(data, layout, det_index, 3, channels, detections);

    let mut x1 = cx - w * 0.5;
    let mut y1 = cy - h * 0.5;
    let mut x2 = cx + w * 0.5;
    let mut y2 = cy + h * 0.5;

    let max_coord = (letterbox.input_size - 1) as f32;
    x1 = x1.clamp(0.0, max_coord);
    y1 = y1.clamp(0.0, max_coord);
    x2 = x2.clamp(0.0, max_coord);
    y2 = y2.clamp(0.0, max_coord);

    let input_bbox = BBox::new(x1, y1, x2, y2);

    let (ox1, oy1) = letterbox.to_original(x1, y1);
    let (ox2, oy2) = letterbox.to_original(x2, y2);
    let output_bbox = BBox::new(ox1, oy1, ox2, oy2);

    (input_bbox, output_bbox)
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}
