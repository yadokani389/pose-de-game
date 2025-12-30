use anyhow::Result;
use std::time::Instant;

mod backend_onnx;
mod backend_openvino;
mod backend_ort;
pub mod camera_app;
mod postprocess;
mod preprocess;

use postprocess::{
    PoseDetection, SegDetection, build_mask, compute_iou, decode_pose, decode_seg, nms_pose,
    nms_seg,
};
use preprocess::{LetterboxInfo, PreprocessedInput};

const INPUT_SIZE: u32 = 640;
const POSE_SCORE_THRESHOLD: f32 = 0.25;
const SEG_SCORE_THRESHOLD: f32 = 0.1;
const PERSON_CLASS_ID: usize = 0;
const NMS_IOU_THRESHOLD: f32 = 0.45;
const KEYPOINT_MIN_CONF: f32 = 0.5;
const KEYPOINT_MIN_COUNT: usize = 5;
const IOU_THRESHOLD: f32 = 0.1;

#[derive(Debug, Clone, Copy)]
pub enum BackendKind {
    Onnx,
    OpenVino,
    Ort,
}

#[derive(Debug, Clone)]
pub struct Keypoint {
    pub x: f32,
    pub y: f32,
    pub score: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct BBox {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

impl BBox {
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        let (min_x, max_x) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        let (min_y, max_y) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
        Self {
            x1: min_x,
            y1: min_y,
            x2: max_x,
            y2: max_y,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PersonResult {
    pub keypoints: Vec<Keypoint>,
    pub bbox: BBox,
    pub score: f32,
    pub mask: Option<Vec<f32>>,
}

#[derive(Debug, Clone)]
pub struct InferenceOutput {
    pub people: Vec<PersonResult>,
    pub frame_w: u32,
    pub frame_h: u32,
    pub pose_output_shape: Vec<usize>,
    pub seg_output_shape: Vec<usize>,
    pub proto_shape: Vec<usize>,
}

#[derive(Debug, Clone)]
pub(crate) struct RawOutput {
    pub(crate) data: Vec<f32>,
    pub(crate) dims: Vec<usize>,
}

#[derive(Debug, Clone)]
pub(crate) struct SegRawOutput {
    pub(crate) dets: RawOutput,
    pub(crate) proto: RawOutput,
}

pub(crate) trait PoseSegBackend {
    fn input_size(&self) -> u32;
    fn infer_pose(&mut self, input: &PreprocessedInput) -> Result<RawOutput>;
    fn infer_seg(&mut self, input: &PreprocessedInput) -> Result<SegRawOutput>;
}

pub struct PoseSegPipeline {
    backend: Box<dyn PoseSegBackend>,
    enable_seg: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct InferenceTimings {
    pub preprocess_ms: f64,
    pub pose_infer_ms: f64,
    pub seg_infer_ms: f64,
    pub postprocess_ms: f64,
}

impl PoseSegPipeline {
    pub fn new(
        backend: BackendKind,
        pose_model: &str,
        seg_model: &str,
        require_cuda: bool,
        enable_seg: bool,
    ) -> Result<Self> {
        let backend: Box<dyn PoseSegBackend> = match backend {
            BackendKind::Onnx => Box::new(backend_onnx::OnnxBackend::load(
                pose_model, seg_model, INPUT_SIZE,
            )?),
            BackendKind::OpenVino => Box::new(backend_openvino::OpenVinoBackend::load(
                pose_model, seg_model, INPUT_SIZE,
            )?),
            BackendKind::Ort => Box::new(backend_ort::OrtBackend::load(
                pose_model,
                seg_model,
                INPUT_SIZE,
                require_cuda,
            )?),
        };

        Ok(Self {
            backend,
            enable_seg,
        })
    }

    pub fn infer(&mut self, frame_w: u32, frame_h: u32, rgba: Vec<u8>) -> Result<InferenceOutput> {
        let (output, _timings) = self.infer_profiled(frame_w, frame_h, rgba)?;
        Ok(output)
    }

    pub fn infer_profiled(
        &mut self,
        frame_w: u32,
        frame_h: u32,
        rgba: Vec<u8>,
    ) -> Result<(InferenceOutput, InferenceTimings)> {
        let preprocess_start = Instant::now();
        let input = preprocess::preprocess(frame_w, frame_h, rgba, self.backend.input_size())?;
        let preprocess_ms = preprocess_start.elapsed().as_secs_f64() * 1000.0;

        let pose_start = Instant::now();
        let pose_raw = self.backend.infer_pose(&input)?;
        let pose_infer_ms = pose_start.elapsed().as_secs_f64() * 1000.0;

        let mut seg_infer_ms = 0.0;
        let mut seg_raw_opt = None;
        if self.enable_seg {
            let seg_start = Instant::now();
            let seg_raw = self.backend.infer_seg(&input)?;
            seg_infer_ms = seg_start.elapsed().as_secs_f64() * 1000.0;
            seg_raw_opt = Some(seg_raw);
        }

        let post_start = Instant::now();
        let pose_dets = decode_pose(
            &pose_raw,
            &input.letterbox,
            POSE_SCORE_THRESHOLD,
            KEYPOINT_MIN_CONF,
            KEYPOINT_MIN_COUNT,
        )?;
        let pose_dets = nms_pose(pose_dets, NMS_IOU_THRESHOLD);
        let (people, seg_shape, proto_shape) = if let Some(seg_raw) = seg_raw_opt {
            let seg_dets = decode_seg(
                &seg_raw,
                &input.letterbox,
                PERSON_CLASS_ID,
                SEG_SCORE_THRESHOLD,
            )?;
            let seg_dets = nms_seg(seg_dets, NMS_IOU_THRESHOLD);
            let people = match_people(pose_dets, seg_dets, &seg_raw, &input.letterbox)?;
            (
                people,
                seg_raw.dets.dims.clone(),
                seg_raw.proto.dims.clone(),
            )
        } else {
            let people = pose_dets
                .into_iter()
                .map(|pose| PersonResult {
                    keypoints: pose.keypoints,
                    bbox: pose.bbox,
                    score: pose.score,
                    mask: None,
                })
                .collect();
            (people, Vec::new(), Vec::new())
        };
        let postprocess_ms = post_start.elapsed().as_secs_f64() * 1000.0;

        let output = InferenceOutput {
            people,
            frame_w,
            frame_h,
            pose_output_shape: pose_raw.dims.clone(),
            seg_output_shape: seg_shape,
            proto_shape,
        };

        Ok((
            output,
            InferenceTimings {
                preprocess_ms,
                pose_infer_ms,
                seg_infer_ms,
                postprocess_ms,
            },
        ))
    }

    pub fn seg_enabled(&self) -> bool {
        self.enable_seg
    }
}

fn match_people(
    pose_dets: Vec<PoseDetection>,
    seg_dets: Vec<SegDetection>,
    seg_raw: &SegRawOutput,
    letterbox: &LetterboxInfo,
) -> Result<Vec<PersonResult>> {
    let mut used = vec![false; seg_dets.len()];
    let mut results = Vec::with_capacity(pose_dets.len());

    for pose in pose_dets {
        let mut best_iou = 0.0;
        let mut best_idx = None;

        for (idx, seg) in seg_dets.iter().enumerate() {
            if used[idx] {
                continue;
            }
            let iou = compute_iou(&pose.bbox, &seg.bbox);
            if iou > best_iou {
                best_iou = iou;
                best_idx = Some(idx);
            }
        }

        let mask = if let Some(idx) = best_idx {
            if best_iou > IOU_THRESHOLD {
                used[idx] = true;
                Some(build_mask(
                    &seg_raw.proto,
                    &seg_dets[idx].coeffs,
                    &seg_dets[idx].bbox_input,
                    letterbox,
                )?)
            } else {
                None
            }
        } else {
            None
        };

        results.push(PersonResult {
            keypoints: pose.keypoints,
            bbox: pose.bbox,
            score: pose.score,
            mask,
        });
    }

    Ok(results)
}
