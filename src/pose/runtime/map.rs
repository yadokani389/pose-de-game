use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use crate::infer::{InferenceOutput, Keypoint};

use super::super::{PeopleData, PersonData};

const KEYPOINT_SCORE_THRESHOLD: f32 = 0.8;
const KEYPOINT_BORDER_PX: f32 = 1.0;
const MASK_THRESHOLD: f32 = 0.5;

pub(super) fn build_people_data(
    output: &InferenceOutput,
    frame_rgba: Option<&[u8]>,
    show_person: bool,
) -> PeopleData {
    let frame_w = output.frame_w;
    let frame_h = output.frame_h;

    output
        .people
        .iter()
        .map(|person| {
            let keypoints = person
                .keypoints
                .iter()
                .map(|kp| normalize_keypoint(kp, frame_w, frame_h))
                .collect();

            let person_image = if show_person {
                match (frame_rgba, person.mask.as_deref()) {
                    (Some(rgba), Some(mask)) => build_person_image(rgba, frame_w, frame_h, mask),
                    _ => None,
                }
            } else {
                None
            };

            PersonData {
                keypoints,
                person_image,
            }
        })
        .collect()
}

fn normalize_keypoint(kp: &Keypoint, frame_w: u32, frame_h: u32) -> Option<[f64; 2]> {
    if kp.score < KEYPOINT_SCORE_THRESHOLD {
        return None;
    }

    let max_x = frame_w.saturating_sub(1) as f32;
    let max_y = frame_h.saturating_sub(1) as f32;
    if kp.x <= KEYPOINT_BORDER_PX || kp.y <= KEYPOINT_BORDER_PX || kp.x >= max_x || kp.y >= max_y {
        return None;
    }

    let x = (kp.x / frame_w as f32) as f64;
    let y = (kp.y / frame_h as f32) as f64;
    Some([x, y])
}

fn build_person_image(
    frame_rgba: &[u8],
    frame_w: u32,
    frame_h: u32,
    mask: &[f32],
) -> Option<Image> {
    let pixel_count = (frame_w * frame_h) as usize;
    if frame_rgba.len() != pixel_count * 4 || mask.len() != pixel_count {
        return None;
    }

    let mut rgba = vec![0u8; frame_rgba.len()];
    for i in 0..pixel_count {
        let alpha = if mask[i] >= MASK_THRESHOLD { 255 } else { 0 };
        let offset = i * 4;
        if alpha == 0 {
            rgba[offset] = 0;
            rgba[offset + 1] = 0;
            rgba[offset + 2] = 0;
            rgba[offset + 3] = 0;
        } else {
            rgba[offset] = frame_rgba[offset];
            rgba[offset + 1] = frame_rgba[offset + 1];
            rgba[offset + 2] = frame_rgba[offset + 2];
            rgba[offset + 3] = alpha;
        }
    }

    Some(Image::new(
        Extent3d {
            width: frame_w,
            height: frame_h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))
}
