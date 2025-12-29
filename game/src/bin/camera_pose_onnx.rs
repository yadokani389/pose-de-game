use std::{sync::mpsc, thread, time::Duration};

use anyhow::{Context, Result};
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use clap::Parser;
use image::{DynamicImage, RgbImage};
use nokhwa::pixel_format::RgbAFormat;
use nokhwa::utils::{ApiBackend, CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::{Camera, nokhwa_initialize, query};
use tract_ndarray::Array4;
use tract_onnx::prelude::*;

const INPUT_SIZE: u32 = 640;
const INFER_INTERVAL_SECONDS: f64 = 0.03;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(long, default_value_t = 0)]
    camera: u32,
    // TODO: make the default model path configurable when packaging is decided.
    #[arg(long, default_value = "../detect/yolo11n-pose.onnx")]
    model: String,
    #[arg(long)]
    list_cameras: bool,
}

struct FrameReceiver(mpsc::Receiver<FrameData>);

#[derive(Resource)]
struct CameraImage(Handle<Image>);

#[derive(Component)]
struct CameraSprite;

struct FrameData {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

struct PoseModel {
    model: TypedRunnableModel<TypedModel>,
    input_size: u32,
}

#[derive(Default, Resource)]
struct PoseDebug {
    keypoints_world: Vec<Vec2>,
    last_infer: f64,
    last_log: f64,
    last_score: f32,
    last_shape: Option<Vec<usize>>,
}

struct LetterboxInfo {
    scale: f32,
    pad_x: f32,
    pad_y: f32,
    orig_w: u32,
    orig_h: u32,
}

fn main() {
    let args = Args::parse();

    nokhwa_initialize(|_granted| {});

    if args.list_cameras {
        list_cameras();
        return;
    }

    let (tx, rx) = mpsc::sync_channel(1);
    spawn_capture_thread(args.camera, tx);

    let pose_model = load_model(&args.model).expect("failed to load ONNX model");

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_non_send_resource(FrameReceiver(rx))
        .insert_non_send_resource(pose_model)
        .insert_resource(PoseDebug::default())
        .add_systems(Startup, setup)
        .add_systems(Update, apply_frame_and_infer)
        .add_systems(Update, draw_keypoints)
        .run();
}

fn load_model(path: &str) -> Result<PoseModel> {
    let model = tract_onnx::onnx()
        .model_for_path(path)
        .with_context(|| format!("failed to open model at {path}"))?
        .with_input_fact(
            0,
            f32::fact([1, 3, INPUT_SIZE as usize, INPUT_SIZE as usize]).into(),
        )?
        .into_optimized()?
        .into_runnable()?;

    Ok(PoseModel {
        model,
        input_size: INPUT_SIZE,
    })
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.spawn(Camera2d);

    let image = Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let image_handle = images.add(image);

    commands.spawn((Sprite::from_image(image_handle.clone()), CameraSprite));
    commands.insert_resource(CameraImage(image_handle));
}

fn apply_frame_and_infer(
    receiver: NonSend<FrameReceiver>,
    camera_image: Res<CameraImage>,
    mut images: ResMut<Assets<Image>>,
    mut sprite: Query<&mut Sprite, With<CameraSprite>>,
    mut pose_model: NonSendMut<PoseModel>,
    mut debug: ResMut<PoseDebug>,
    time: Res<Time>,
) {
    let mut latest = None;
    while let Ok(frame) = receiver.0.try_recv() {
        latest = Some(frame);
    }

    let Some(frame) = latest else {
        return;
    };

    let frame_for_infer = frame.data.clone();

    let image = images
        .get_mut(&camera_image.0)
        .expect("camera image should exist");

    let extent = Extent3d {
        width: frame.width,
        height: frame.height,
        depth_or_array_layers: 1,
    };

    if image.texture_descriptor.size != extent
        || image.texture_descriptor.format != TextureFormat::Rgba8UnormSrgb
    {
        *image = Image::new(
            extent,
            TextureDimension::D2,
            frame.data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
    } else {
        image.data = Some(frame.data);
    }

    if let Ok(mut sprite) = sprite.single_mut() {
        let size = Vec2::new(frame.width as f32, frame.height as f32);
        if sprite.custom_size != Some(size) {
            sprite.custom_size = Some(size);
        }
    }

    let now = time.elapsed_secs_f64();
    if now - debug.last_infer < INFER_INTERVAL_SECONDS {
        return;
    }
    debug.last_infer = now;

    match run_pose_inference(&mut pose_model, frame.width, frame.height, frame_for_infer) {
        Ok((points, score, shape)) => {
            debug.keypoints_world = points;
            debug.last_score = score;
            debug.last_shape = Some(shape);
        }
        Err(err) => {
            eprintln!("inference error: {err}");
        }
    }

    if now - debug.last_log >= 1.0 {
        debug.last_log = now;
        let shape = debug
            .last_shape
            .as_ref()
            .map(|dims| {
                dims.iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join("x")
            })
            .unwrap_or_else(|| "unknown".to_string());
        println!(
            "pose: keypoints={} score={:.3} output_shape={} ",
            debug.keypoints_world.len(),
            debug.last_score,
            shape
        );
    }
}

fn run_pose_inference(
    pose_model: &mut PoseModel,
    frame_w: u32,
    frame_h: u32,
    rgba: Vec<u8>,
) -> Result<(Vec<Vec2>, f32, Vec<usize>)> {
    let (tensor, letterbox) = preprocess_to_tensor(frame_w, frame_h, rgba, pose_model.input_size)?;

    let outputs = pose_model.model.run(tvec![tensor.into()])?;
    let output = outputs
        .get(0)
        .context("missing model output")?
        .to_array_view::<f32>()?;
    let shape = output.shape().to_vec();

    let Some((best_idx, layout, channels, detections, best_score)) = find_best_detection(&output)
    else {
        return Ok((Vec::new(), 0.0, shape));
    };

    let keypoints = decode_keypoints(&output, best_idx, layout, channels, detections, &letterbox);

    let mut world_points = Vec::new();
    for (x, y, score) in keypoints {
        if score <= 0.1 {
            continue;
        }
        let world = image_to_world(x, y, frame_w, frame_h);
        world_points.push(world);
    }

    Ok((world_points, best_score, shape))
}

fn preprocess_to_tensor(
    frame_w: u32,
    frame_h: u32,
    rgba: Vec<u8>,
    input_size: u32,
) -> Result<(Tensor, LetterboxInfo)> {
    let rgba_image =
        image::RgbaImage::from_raw(frame_w, frame_h, rgba).context("failed to build RGBA image")?;
    let rgb_image = DynamicImage::ImageRgba8(rgba_image).to_rgb8();

    let scale = input_size as f32 / frame_w.max(frame_h) as f32;
    let new_w = (frame_w as f32 * scale).round().max(1.0) as u32;
    let new_h = (frame_h as f32 * scale).round().max(1.0) as u32;
    let resized = image::imageops::resize(
        &rgb_image,
        new_w,
        new_h,
        image::imageops::FilterType::Triangle,
    );

    let mut padded = RgbImage::new(input_size, input_size);
    let pad_x = ((input_size - new_w) / 2) as i64;
    let pad_y = ((input_size - new_h) / 2) as i64;
    image::imageops::replace(&mut padded, &resized, pad_x, pad_y);

    let tensor: Tensor = Array4::from_shape_fn(
        (1, 3, input_size as usize, input_size as usize),
        |(_, c, y, x)| padded.get_pixel(x as u32, y as u32)[c] as f32 / 255.0,
    )
    .into();

    Ok((
        tensor,
        LetterboxInfo {
            scale,
            pad_x: pad_x as f32,
            pad_y: pad_y as f32,
            orig_w: frame_w,
            orig_h: frame_h,
        },
    ))
}

#[derive(Copy, Clone)]
enum OutputLayout {
    ChannelsByDet,
    DetectionsByChannel,
}

fn find_best_detection(
    output: &tract_ndarray::ArrayViewD<'_, f32>,
) -> Option<(usize, OutputLayout, usize, usize, f32)> {
    let dims = output.shape();
    let (channels, detections, layout) = match dims.len() {
        3 => {
            let a = dims[1];
            let b = dims[2];
            if a <= b {
                (a, b, OutputLayout::DetectionsByChannel)
            } else {
                (b, a, OutputLayout::ChannelsByDet)
            }
        }
        2 => {
            let a = dims[0];
            let b = dims[1];
            if a <= b {
                (a, b, OutputLayout::DetectionsByChannel)
            } else {
                (b, a, OutputLayout::ChannelsByDet)
            }
        }
        _ => return None,
    };

    let mut best_score = 0.0;
    let mut best_idx = None;

    for i in 0..detections {
        let score = get_value(output, layout, i, 4);
        if score > best_score {
            best_score = score;
            best_idx = Some(i);
        }
    }

    best_idx.map(|idx| (idx, layout, channels, detections, best_score))
}

fn decode_keypoints(
    output: &tract_ndarray::ArrayViewD<'_, f32>,
    det_index: usize,
    layout: OutputLayout,
    channels: usize,
    _detections: usize,
    letterbox: &LetterboxInfo,
) -> Vec<(f32, f32, f32)> {
    if channels <= 5 {
        return Vec::new();
    }

    let kpt_count = (channels - 5) / 3;
    let mut points = Vec::with_capacity(kpt_count);

    for k in 0..kpt_count {
        let base = 5 + k * 3;
        let x = get_value(output, layout, det_index, base);
        let y = get_value(output, layout, det_index, base + 1);
        let score = get_value(output, layout, det_index, base + 2);
        let (orig_x, orig_y) = letterbox.to_original(x, y);
        points.push((orig_x, orig_y, score));
    }

    points
}

fn get_value(
    output: &tract_ndarray::ArrayViewD<'_, f32>,
    layout: OutputLayout,
    det_index: usize,
    channel: usize,
) -> f32 {
    match (output.shape().len(), layout) {
        (3, OutputLayout::DetectionsByChannel) => output[[0, channel, det_index]],
        (3, OutputLayout::ChannelsByDet) => output[[0, det_index, channel]],
        (2, OutputLayout::DetectionsByChannel) => output[[channel, det_index]],
        (2, OutputLayout::ChannelsByDet) => output[[det_index, channel]],
        _ => 0.0,
    }
}

impl LetterboxInfo {
    fn to_original(&self, x: f32, y: f32) -> (f32, f32) {
        let mut ox = (x - self.pad_x) / self.scale;
        let mut oy = (y - self.pad_y) / self.scale;
        ox = ox.clamp(0.0, self.orig_w.saturating_sub(1) as f32);
        oy = oy.clamp(0.0, self.orig_h.saturating_sub(1) as f32);
        (ox, oy)
    }
}

fn image_to_world(x: f32, y: f32, w: u32, h: u32) -> Vec2 {
    let half_w = w as f32 / 2.0;
    let half_h = h as f32 / 2.0;
    Vec2::new(x - half_w, half_h - y)
}

fn draw_keypoints(debug: Res<PoseDebug>, mut gizmos: Gizmos) {
    for point in &debug.keypoints_world {
        gizmos.circle_2d(*point, 6.0, Color::srgb(0.2, 1.0, 0.2));
    }
}

fn spawn_capture_thread(camera_index: u32, sender: mpsc::SyncSender<FrameData>) {
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

fn list_cameras() {
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
