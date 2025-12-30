use clap::Parser;

use pose_de_game::infer::BackendKind;
use pose_de_game::infer::camera_app::{CameraAppConfig, run_camera_pose_app};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(long, default_value_t = 0)]
    camera: u32,
    #[arg(long, default_value = "../detect/yolo11n-pose.onnx")]
    pose_model: String,
    #[arg(long, default_value = "../detect/yolo11n-seg.onnx")]
    seg_model: String,
    #[arg(long, value_enum, default_value = "onnx")]
    backend: BackendArg,
    #[arg(long)]
    list_cameras: bool,
    #[arg(long)]
    require_cuda: bool,
    #[arg(long)]
    seg: bool,
    #[arg(long)]
    profile: bool,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum BackendArg {
    Onnx,
    Openvino,
    Ort,
}

impl From<BackendArg> for BackendKind {
    fn from(value: BackendArg) -> Self {
        match value {
            BackendArg::Onnx => BackendKind::Onnx,
            BackendArg::Openvino => BackendKind::OpenVino,
            BackendArg::Ort => BackendKind::Ort,
        }
    }
}

fn main() {
    let args = Args::parse();

    let config = CameraAppConfig {
        camera: args.camera,
        list_cameras: args.list_cameras,
        backend: args.backend.into(),
        pose_model: args.pose_model,
        seg_model: args.seg_model,
        require_cuda: args.require_cuda,
        enable_seg: args.seg,
        enable_profile: args.profile,
    };

    if let Err(err) = run_camera_pose_app(config) {
        eprintln!("camera_pose failed: {err}");
    }
}
