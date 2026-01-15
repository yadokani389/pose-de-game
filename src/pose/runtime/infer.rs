use bevy::prelude::*;

use crate::args::Args;

use super::super::{FrameImage, LatestFrameRes, PeopleDataRes, PoseRuntimeSettings};
use super::camera::FrameReceiver;
use super::map::{PoseTrackState, build_people_data};
use super::worker::{InferRequest, InferWorker};

pub(super) fn infer_from_camera(
    receiver: NonSend<FrameReceiver>,
    worker: NonSend<InferWorker>,
    args: Res<Args>,
    settings: Res<PoseRuntimeSettings>,
    mut people_data: ResMut<PeopleDataRes>,
    mut latest_frame: ResMut<LatestFrameRes>,
    mut tracks: ResMut<PoseTrackState>,
    control: Res<crate::pose::PoseRuntimeControl>,
) {
    let capture_frame = settings.capture_frame;
    if let Some(frame) = receiver.drain_latest() {
        let mut frame = frame;
        if args.mirror_camera {
            mirror_rgba_in_place(frame.width, frame.height, &mut frame.data);
        }
        worker.try_send(InferRequest {
            frame,
            capture_frame,
            generation: control.generation,
        });
    }

    if let Some(response) = worker.drain_latest() {
        if response.generation != control.generation {
            return;
        }

        let mut frame_rgba = response.frame_rgba;
        **people_data = build_people_data(
            &response.output,
            frame_rgba.as_deref(),
            args.show_person,
            &mut tracks,
        );

        if settings.capture_frame
            && let Some(rgba) = frame_rgba.take()
        {
            latest_frame.frame = Some(FrameImage {
                width: response.output.frame_w,
                height: response.output.frame_h,
                data: rgba,
            });
        }
    }
}

fn mirror_rgba_in_place(width: u32, height: u32, data: &mut [u8]) {
    let width = width as usize;
    let height = height as usize;
    let row_stride = width.saturating_mul(4);
    if row_stride == 0 || height == 0 {
        return;
    }
    let expected_len = row_stride.saturating_mul(height);
    if data.len() != expected_len {
        return;
    }

    for y in 0..height {
        let row_start = y * row_stride;
        let row = &mut data[row_start..row_start + row_stride];
        for x in 0..width / 2 {
            let left = x * 4;
            let right = (width - 1 - x) * 4;
            row.swap(left, right);
            row.swap(left + 1, right + 1);
            row.swap(left + 2, right + 2);
            row.swap(left + 3, right + 3);
        }
    }
}
