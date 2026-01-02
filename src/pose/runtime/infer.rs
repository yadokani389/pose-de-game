use bevy::prelude::*;

use crate::{AppState, args::Args};

use super::super::{FrameImage, LatestFrameRes, PeopleDataRes};
use super::camera::FrameReceiver;
use super::map::build_people_data;
use super::worker::{InferRequest, InferWorker};

pub(super) fn infer_from_camera(
    receiver: NonSend<FrameReceiver>,
    worker: NonSend<InferWorker>,
    args: Res<Args>,
    mut people_data: ResMut<PeopleDataRes>,
    mut latest_frame: ResMut<LatestFrameRes>,
    app_state: Res<State<AppState>>,
    control: Res<crate::pose::PoseRuntimeControl>,
) {
    let debug_active = matches!(*app_state.get(), AppState::PoseDebug);
    if let Some(frame) = receiver.drain_latest() {
        worker.try_send(InferRequest {
            frame,
            debug_active,
            generation: control.generation,
        });
    }

    if let Some(response) = worker.drain_latest() {
        if response.generation != control.generation {
            return;
        }

        let mut frame_rgba = response.frame_rgba;
        **people_data =
            build_people_data(&response.output, frame_rgba.as_deref(), args.show_person);

        if debug_active {
            if let Some(rgba) = frame_rgba.take() {
                latest_frame.frame = Some(FrameImage {
                    width: response.output.frame_w,
                    height: response.output.frame_h,
                    data: rgba,
                });
            }
        }
    }
}
