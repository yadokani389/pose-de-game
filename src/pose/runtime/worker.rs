use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use crate::args::Args;
use crate::infer::{InferenceTimings, PoseSegPipeline};

use super::camera::FrameData;
use super::profile::{ProfileStats, log_profile, update_profile};

const INFER_INTERVAL: Duration = Duration::from_millis(30);
const PROFILE_LOG_INTERVAL: Duration = Duration::from_secs(1);

pub(super) struct InferRequest {
    pub(super) frame: FrameData,
    pub(super) capture_frame: bool,
    pub(super) generation: u64,
}

pub(super) struct InferResponse {
    pub(super) output: crate::infer::InferenceOutput,
    pub(super) frame_rgba: Option<Vec<u8>>,
    pub(super) generation: u64,
}

pub(in crate::pose) struct InferWorker {
    request_tx: mpsc::SyncSender<InferRequest>,
    response_rx: mpsc::Receiver<InferResponse>,
}

impl InferWorker {
    pub(super) fn spawn(args: &Args) -> Result<Self> {
        let (request_tx, request_rx) = mpsc::sync_channel(1);
        let (response_tx, response_rx) = mpsc::sync_channel(1);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let args = args.clone();

        thread::spawn(move || {
            if let Err(err) = run_worker(request_rx, response_tx, ready_tx, args) {
                eprintln!("inference worker stopped: {err}");
            }
        });

        let ready = ready_rx
            .recv()
            .context("failed to receive inference worker ready signal")?;
        if let Err(message) = ready {
            return Err(anyhow::anyhow!(message));
        }

        Ok(Self {
            request_tx,
            response_rx,
        })
    }

    pub(super) fn try_send(&self, request: InferRequest) {
        let _ = self.request_tx.try_send(request);
    }

    pub(super) fn drain_latest(&self) -> Option<InferResponse> {
        let mut latest = None;
        while let Ok(response) = self.response_rx.try_recv() {
            latest = Some(response);
        }
        latest
    }
}

struct WorkerState {
    last_infer: Option<Instant>,
    last_log: Instant,
    profile: ProfileStats,
}

impl WorkerState {
    fn new() -> Self {
        Self {
            last_infer: None,
            last_log: Instant::now(),
            profile: ProfileStats::default(),
        }
    }

    fn should_infer(&mut self) -> bool {
        let now = Instant::now();
        if let Some(last) = self.last_infer
            && now.duration_since(last) < INFER_INTERVAL
        {
            return false;
        }
        self.last_infer = Some(now);
        true
    }
}

fn run_worker(
    request_rx: mpsc::Receiver<InferRequest>,
    response_tx: mpsc::SyncSender<InferResponse>,
    ready_tx: mpsc::SyncSender<Result<(), String>>,
    args: Args,
) -> Result<()> {
    let enable_seg = args.show_person;
    let mut pipeline = match PoseSegPipeline::new(
        args.backend.into(),
        args.pose_model.as_deref(),
        args.seg_model.as_deref(),
        args.require_cuda,
        enable_seg,
    ) {
        Ok(pipeline) => {
            let _ = ready_tx.send(Ok(()));
            pipeline
        }
        Err(err) => {
            let _ = ready_tx.send(Err(err.to_string()));
            return Err(err);
        }
    };

    let mut state = WorkerState::new();

    while let Ok(request) = request_rx.recv() {
        if !state.should_infer() {
            continue;
        }

        let needs_rgba = args.show_person || request.capture_frame;
        let mut frame_rgba = None;
        let frame_for_infer = if needs_rgba {
            let data = request.frame.data;
            frame_rgba = Some(data);
            frame_rgba
                .as_ref()
                .expect("frame_rgba should be set when needed")
                .clone()
        } else {
            request.frame.data
        };

        let total_timer = if args.profile {
            Some(Instant::now())
        } else {
            None
        };

        let mut timings = InferenceTimings::default();
        let output_result = if args.profile {
            pipeline
                .infer_profiled(request.frame.width, request.frame.height, frame_for_infer)
                .map(|(output, timing)| {
                    timings = timing;
                    output
                })
        } else {
            pipeline.infer(request.frame.width, request.frame.height, frame_for_infer)
        };

        let output_ok = output_result.is_ok();
        match output_result {
            Ok(output) => {
                let response = InferResponse {
                    output,
                    frame_rgba: if needs_rgba { frame_rgba } else { None },
                    generation: request.generation,
                };
                let _ = response_tx.try_send(response);
            }
            Err(err) => {
                eprintln!("inference error: {err}");
            }
        }

        if args.profile && output_ok {
            let total_ms = total_timer
                .expect("profile timer should be set")
                .elapsed()
                .as_secs_f64()
                * 1000.0;
            update_profile(&mut state.profile, timings, total_ms);
            if state.last_log.elapsed() >= PROFILE_LOG_INTERVAL {
                state.last_log = Instant::now();
                log_profile(&mut state.profile);
            }
        }
    }

    Ok(())
}
