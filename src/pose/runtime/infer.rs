use std::time::Instant;

use bevy::prelude::*;

use crate::infer::{InferenceTimings, PoseSegPipeline};
use crate::{AppState, args::Args};

use super::super::{FrameImage, LatestFrameRes, PeopleDataRes};
use super::camera::FrameReceiver;
use super::map::build_people_data;
use super::profile::{ProfileStats, log_profile, update_profile};

const INFER_INTERVAL_SECONDS: f64 = 0.03;

#[derive(Default)]
pub(in crate::pose) struct InferState {
    last_infer: f64,
    last_log: f64,
    profile: ProfileStats,
}

pub(super) fn infer_from_camera(
    receiver: NonSend<FrameReceiver>,
    mut pipeline: NonSendMut<PoseSegPipeline>,
    args: Res<Args>,
    mut people_data: ResMut<PeopleDataRes>,
    mut latest_frame: ResMut<LatestFrameRes>,
    time: Res<Time>,
    app_state: Res<State<AppState>>,
    mut state: Local<InferState>,
) {
    let Some(frame) = receiver.drain_latest() else {
        return;
    };

    let debug_active = matches!(*app_state.get(), AppState::PoseDebug);
    let needs_rgba = args.show_person || debug_active;

    let now = time.elapsed_secs_f64();
    if now - state.last_infer < INFER_INTERVAL_SECONDS {
        return;
    }
    state.last_infer = now;

    let mut frame_rgba = None;
    let frame_for_infer = if needs_rgba {
        let data = frame.data;
        frame_rgba = Some(data);
        frame_rgba
            .as_ref()
            .expect("frame_rgba should be set when needed")
            .clone()
    } else {
        frame.data
    };

    let total_timer = if args.profile {
        Some(Instant::now())
    } else {
        None
    };

    let mut timings = InferenceTimings::default();
    let output_result = if args.profile {
        pipeline
            .infer_profiled(frame.width, frame.height, frame_for_infer)
            .map(|(output, timing)| {
                timings = timing;
                output
            })
    } else {
        pipeline.infer(frame.width, frame.height, frame_for_infer)
    };

    let output_ok = output_result.is_ok();
    match output_result {
        Ok(output) => {
            **people_data = build_people_data(&output, frame_rgba.as_deref(), args.show_person);
        }
        Err(err) => {
            eprintln!("inference error: {err}");
        }
    }

    if debug_active {
        if let Some(rgba) = frame_rgba.take() {
            latest_frame.frame = Some(FrameImage {
                width: frame.width,
                height: frame.height,
                data: rgba,
            });
        }
    }

    if args.profile && output_ok {
        let total_ms = total_timer
            .expect("profile timer should be set")
            .elapsed()
            .as_secs_f64()
            * 1000.0;
        update_profile(&mut state.profile, timings, total_ms);
        if now - state.last_log >= 1.0 {
            state.last_log = now;
            log_profile(&mut state.profile);
        }
    }
}
