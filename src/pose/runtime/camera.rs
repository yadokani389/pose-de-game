use std::{sync::mpsc, thread, time::Duration};

use nokhwa::pixel_format::RgbAFormat;
use nokhwa::utils::{ApiBackend, CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::{Camera, nokhwa_initialize, query};

pub(in crate::pose) struct FrameReceiver(mpsc::Receiver<FrameData>);

impl FrameReceiver {
    pub(super) fn new(receiver: mpsc::Receiver<FrameData>) -> Self {
        Self(receiver)
    }

    pub(super) fn drain_latest(&self) -> Option<FrameData> {
        let mut latest = None;
        while let Ok(frame) = self.0.try_recv() {
            latest = Some(frame);
        }
        latest
    }
}

pub(super) struct FrameData {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) data: Vec<u8>,
}

pub(super) fn initialize() {
    nokhwa_initialize(|_granted| {});
}

pub(super) fn list_cameras() {
    initialize();
    match query(ApiBackend::Auto) {
        Ok(cameras) => {
            for camera in cameras {
                println!("{camera:?}");
            }
        }
        Err(err) => {
            eprintln!("Failed to list cameras: {err}");
        }
    }
}

pub(super) fn spawn_capture_thread(camera_index: u32, sender: mpsc::SyncSender<FrameData>) {
    thread::spawn(move || {
        let mut camera = match open_camera(camera_index) {
            Some(camera) => camera,
            None => {
                eprintln!("Failed to open camera {camera_index} with any format");
                return;
            }
        };

        loop {
            let frame = match camera.frame() {
                Ok(frame) => frame,
                Err(err) => {
                    eprintln!("Failed to read frame: {err}");
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
            };

            let decoded = match frame.decode_image::<RgbAFormat>() {
                Ok(decoded) => decoded,
                Err(err) => {
                    eprintln!("Failed to decode frame: {err}");
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
            };

            let (width, height) = decoded.dimensions();
            let data = decoded.into_raw();

            if sender
                .try_send(FrameData {
                    width,
                    height,
                    data,
                })
                .is_err()
            {
                // Drop frame when the renderer is not keeping up.
            }
        }
    });
}

fn open_camera(camera_index: u32) -> Option<Camera> {
    let index = CameraIndex::Index(camera_index);
    let requested =
        RequestedFormat::new::<RgbAFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
    let mut camera = match Camera::new(index, requested) {
        Ok(camera) => camera,
        Err(err) => {
            eprintln!("Failed to open camera {camera_index}: {err}");
            return None;
        }
    };

    if let Err(err) = camera.open_stream() {
        eprintln!("Failed to start stream: {err}");
        return None;
    }

    Some(camera)
}
