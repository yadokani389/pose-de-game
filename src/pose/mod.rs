use anyhow::Result;
use bevy::prelude::*;

use runtime::infer_from_camera;

mod render;
mod runtime;

use crate::args::Args;
pub use render::{PoseRenderSettings, disable_pose_render, enable_pose_render};

pub struct PosePlugin;

impl Plugin for PosePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PoseRuntimeControl>()
            .init_resource::<PoseRuntimeSettings>()
            .init_resource::<PeopleDataRes>()
            .init_resource::<LatestFrameRes>()
            .init_resource::<runtime::PoseTrackState>()
            .add_systems(Update, infer_from_camera.run_if(pose_runtime_enabled));
        render::configure(app);
    }
}

pub fn setup_runtime(app: &mut App, args: &Args) -> Result<()> {
    runtime::setup_runtime(app, args)
}

pub fn list_cameras() {
    runtime::list_cameras();
}

#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct PoseRuntimeControl {
    pub enabled: bool,
    pub generation: u64,
}

pub fn pose_runtime_enabled(control: Res<PoseRuntimeControl>) -> bool {
    control.enabled
}

pub fn enable_pose_runtime(
    mut control: ResMut<PoseRuntimeControl>,
    mut tracks: ResMut<runtime::PoseTrackState>,
) {
    control.enabled = true;
    control.generation = control.generation.wrapping_add(1);
    tracks.clear();
}

pub fn disable_pose_runtime(
    mut control: ResMut<PoseRuntimeControl>,
    mut people: ResMut<PeopleDataRes>,
    mut latest_frame: ResMut<LatestFrameRes>,
    mut tracks: ResMut<runtime::PoseTrackState>,
) {
    control.enabled = false;
    people.clear();
    latest_frame.frame = None;
    tracks.clear();
}

#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct PoseRuntimeSettings {
    pub capture_frame: bool,
}

pub fn enable_pose_frame_capture(mut settings: ResMut<PoseRuntimeSettings>) {
    settings.capture_frame = true;
}

pub fn disable_pose_frame_capture(mut settings: ResMut<PoseRuntimeSettings>) {
    settings.capture_frame = false;
}

pub fn estimate_center_x(keypoints: &[Option<[f64; 2]>]) -> Option<f64> {
    if let (Some(left), Some(right)) = (
        keypoints.get(5).and_then(|kp| *kp),
        keypoints.get(6).and_then(|kp| *kp),
    ) {
        return Some((left[0] + right[0]) * 0.5);
    }

    if let (Some(left), Some(right)) = (
        keypoints.get(11).and_then(|kp| *kp),
        keypoints.get(12).and_then(|kp| *kp),
    ) {
        return Some((left[0] + right[0]) * 0.5);
    }

    let mut total = 0.0;
    let mut count = 0.0;
    for keypoint in keypoints.iter().flatten() {
        total += keypoint[0];
        count += 1.0;
    }
    if count > 0.0 {
        return Some(total / count);
    }
    None
}

#[derive(Debug, Clone)]
pub struct PersonData {
    pub id: u64,
    pub keypoints: Vec<Option<[f64; 2]>>,
    pub person_image: Option<Image>,
}

type PeopleData = Vec<PersonData>;

#[derive(Debug, Clone)]
pub struct FrameImage {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

#[derive(Resource, Default)]
pub struct LatestFrameRes {
    pub frame: Option<FrameImage>,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct PeopleDataRes(PeopleData);
