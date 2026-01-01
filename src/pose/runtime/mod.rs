use std::sync::mpsc;

use anyhow::Result;
use bevy::prelude::*;

use crate::args::Args;
use crate::infer::PoseSegPipeline;

mod camera;
mod infer;
mod map;
mod profile;

use super::{LatestFrameRes, PeopleDataRes};

pub(super) fn infer_from_camera(
    receiver: NonSend<camera::FrameReceiver>,
    pipeline: NonSendMut<PoseSegPipeline>,
    args: Res<Args>,
    people_data: ResMut<PeopleDataRes>,
    latest_frame: ResMut<LatestFrameRes>,
    time: Res<Time>,
    app_state: Res<State<crate::AppState>>,
    infer_state: Local<infer::InferState>,
) {
    infer::infer_from_camera(
        receiver,
        pipeline,
        args,
        people_data,
        latest_frame,
        time,
        app_state,
        infer_state,
    );
}

pub fn setup_runtime(app: &mut App, args: &Args) -> Result<()> {
    camera::initialize();

    let enable_seg = args.show_person;

    let pipeline = PoseSegPipeline::new(
        args.backend.into(),
        args.pose_model.as_deref(),
        args.seg_model.as_deref(),
        args.require_cuda,
        enable_seg,
    )?;

    let (tx, rx) = mpsc::sync_channel(1);
    camera::spawn_capture_thread(args.camera, tx);

    app.insert_non_send_resource(camera::FrameReceiver::new(rx));
    app.insert_non_send_resource(pipeline);

    Ok(())
}

pub(super) fn list_cameras() {
    camera::list_cameras();
}
