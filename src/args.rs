use std::path::PathBuf;

use bevy::ecs::resource::Resource;
use clap::Parser;

use crate::infer::BackendKind;

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum BackendArg {
    Onnx,
    #[cfg(feature = "openvino")]
    Openvino,
    Ort,
}

impl From<BackendArg> for BackendKind {
    fn from(value: BackendArg) -> Self {
        match value {
            BackendArg::Onnx => BackendKind::Onnx,
            #[cfg(feature = "openvino")]
            BackendArg::Openvino => BackendKind::OpenVino,
            BackendArg::Ort => BackendKind::Ort,
        }
    }
}

#[derive(Parser, Resource, Debug, Clone)]
pub struct Args {
    #[clap(short, long)]
    pub synctest: bool,
    #[clap(short, long, default_value = "")]
    pub iroh: String,
    /// Show the person overlay (debug use). Enables segmentation.
    #[clap(long)]
    pub show_person: bool,
    #[arg(long, default_value_t = 0)]
    pub camera: u32,
    /// Pose model path (optional). Uses embedded model when omitted.
    #[arg(long)]
    pub pose_model: Option<PathBuf>,
    /// Segmentation model path (optional). Uses embedded model when omitted.
    #[arg(long)]
    pub seg_model: Option<PathBuf>,
    #[arg(long, value_enum, default_value = "onnx")]
    pub backend: BackendArg,
    #[arg(long)]
    pub list_cameras: bool,
    #[arg(long)]
    pub require_cuda: bool,
    #[arg(long)]
    pub profile: bool,
}
