use std::time::{Duration, Instant};

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use crate::infer::{BBox, InferenceOutput, Keypoint};

use super::super::{PeopleData, PersonData};

const KEYPOINT_SCORE_THRESHOLD: f32 = 0.5;
const KEYPOINT_BORDER_PX: f32 = 1.0;
const MASK_THRESHOLD: f32 = 0.5;
const TRACK_IOU_THRESHOLD: f32 = 0.3;
const TRACK_TTL: Duration = Duration::from_millis(500);
const KEYPOINT_HOLD: Duration = Duration::from_millis(150);
const KEYPOINT_EMA_ALPHA: f64 = 0.5;

pub(super) fn build_people_data(
    output: &InferenceOutput,
    frame_rgba: Option<&[u8]>,
    show_person: bool,
    tracks: &mut PoseTrackState,
) -> PeopleData {
    let now = Instant::now();
    let frame_w = output.frame_w;
    let frame_h = output.frame_h;
    let mut detections = Vec::with_capacity(output.people.len());

    for person in &output.people {
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

        detections.push(DetectionData {
            bbox: person._bbox,
            score: person._score,
            keypoints,
            person_image,
        });
    }

    tracks.update(detections, now)
}

#[derive(Resource, Default)]
pub(crate) struct PoseTrackState {
    tracks: Vec<TrackState>,
    next_id: u64,
}

impl PoseTrackState {
    pub(crate) fn clear(&mut self) {
        self.tracks.clear();
        self.next_id = 0;
    }

    fn update(&mut self, mut detections: Vec<DetectionData>, now: Instant) -> PeopleData {
        let mut det_used = vec![false; detections.len()];
        let mut track_matches = vec![None; self.tracks.len()];

        for (track_index, track) in self.tracks.iter().enumerate() {
            let mut best_iou = 0.0f32;
            let mut best_score = 0.0f32;
            let mut best_det = None;
            for (det_index, det) in detections.iter().enumerate() {
                if det_used[det_index] {
                    continue;
                }
                let iou = compute_iou(&track.bbox, &det.bbox);
                if iou < TRACK_IOU_THRESHOLD {
                    continue;
                }
                if iou > best_iou + 1e-6
                    || ((iou - best_iou).abs() <= 1e-6 && det.score > best_score)
                {
                    best_iou = iou;
                    best_score = det.score;
                    best_det = Some(det_index);
                }
            }

            if let Some(det_index) = best_det {
                det_used[det_index] = true;
                track_matches[track_index] = Some(det_index);
            }
        }

        for (track_index, det_index) in track_matches.into_iter().enumerate() {
            let track = &mut self.tracks[track_index];
            if let Some(det_index) = det_index {
                let det = &mut detections[det_index];
                track.apply_detection(det, now);
            } else {
                track.expire_keypoints(now);
                track.person_image = None;
            }
        }

        for (det_index, det) in detections.into_iter().enumerate() {
            if det_used[det_index] {
                continue;
            }
            let id = self.next_id;
            self.next_id = self.next_id.wrapping_add(1);
            self.tracks.push(TrackState::from_detection(det, now, id));
        }

        self.tracks
            .retain(|track| now.duration_since(track.last_seen) <= TRACK_TTL);

        self.tracks
            .iter()
            .map(|track| PersonData {
                id: track.id,
                keypoints: track.keypoints.iter().map(|kp| kp.pos).collect(),
                person_image: track.person_image.clone(),
            })
            .collect()
    }
}

struct DetectionData {
    bbox: BBox,
    score: f32,
    keypoints: Vec<Option<[f64; 2]>>,
    person_image: Option<Image>,
}

struct TrackState {
    id: u64,
    bbox: BBox,
    keypoints: Vec<SmoothedKeypoint>,
    last_seen: Instant,
    person_image: Option<Image>,
}

struct SmoothedKeypoint {
    pos: Option<[f64; 2]>,
    last_seen: Option<Instant>,
}

impl TrackState {
    fn from_detection(det: DetectionData, now: Instant, id: u64) -> Self {
        let keypoints = det
            .keypoints
            .into_iter()
            .map(|pos| SmoothedKeypoint {
                pos,
                last_seen: pos.map(|_| now),
            })
            .collect();
        Self {
            id,
            bbox: det.bbox,
            keypoints,
            last_seen: now,
            person_image: det.person_image,
        }
    }

    fn apply_detection(&mut self, det: &mut DetectionData, now: Instant) {
        self.bbox = det.bbox;
        self.last_seen = now;
        self.person_image = det.person_image.take();

        if self.keypoints.len() != det.keypoints.len() {
            self.keypoints = det
                .keypoints
                .iter()
                .map(|pos| SmoothedKeypoint {
                    pos: *pos,
                    last_seen: pos.map(|_| now),
                })
                .collect();
            return;
        }

        for (state, det_pos) in self.keypoints.iter_mut().zip(det.keypoints.iter()) {
            let Some(pos) = det_pos else {
                continue;
            };
            state.pos = Some(match state.pos {
                Some(prev) => smooth_keypoint(prev, *pos),
                None => *pos,
            });
            state.last_seen = Some(now);
        }
        self.expire_keypoints(now);
    }

    fn expire_keypoints(&mut self, now: Instant) {
        for state in &mut self.keypoints {
            let Some(last_seen) = state.last_seen else {
                continue;
            };
            if now.duration_since(last_seen) > KEYPOINT_HOLD {
                state.pos = None;
                state.last_seen = None;
            }
        }
    }
}

fn smooth_keypoint(prev: [f64; 2], next: [f64; 2]) -> [f64; 2] {
    [
        prev[0] + (next[0] - prev[0]) * KEYPOINT_EMA_ALPHA,
        prev[1] + (next[1] - prev[1]) * KEYPOINT_EMA_ALPHA,
    ]
}

fn compute_iou(a: &BBox, b: &BBox) -> f32 {
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
