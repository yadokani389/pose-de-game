use std::sync::mpsc;

use anyhow::Result;
use bevy::prelude::*;

use crate::args::Args;
mod camera;
mod infer;
mod map;
mod profile;
mod worker;

use super::{LatestFrameRes, PeopleDataRes, PoseRuntimeSettings};
pub(crate) use map::PoseTrackState;
use worker::InferWorker;

pub(super) fn infer_from_camera(
    receiver: NonSend<camera::FrameReceiver>,
    worker: NonSend<InferWorker>,
    args: Res<Args>,
    settings: Res<PoseRuntimeSettings>,
    people_data: ResMut<PeopleDataRes>,
    latest_frame: ResMut<LatestFrameRes>,
    tracks: ResMut<PoseTrackState>,
    control: Res<crate::pose::PoseRuntimeControl>,
) {
    infer::infer_from_camera(
        receiver,
        worker,
        args,
        settings,
        people_data,
        latest_frame,
        tracks,
        control,
    );
}

pub fn setup_runtime(app: &mut App, args: &Args) -> Result<()> {
    camera::initialize();

    let (tx, rx) = mpsc::sync_channel(1);
    camera::spawn_capture_thread(args.camera, tx);

    app.insert_non_send_resource(camera::FrameReceiver::new(rx));
    app.insert_non_send_resource(InferWorker::spawn(args)?);

    Ok(())
}

pub(super) fn list_cameras() {
    camera::list_cameras();
}
