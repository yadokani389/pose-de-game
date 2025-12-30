use std::{sync::mpsc, thread, time::Duration, time::Instant};

use anyhow::Result;
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use nokhwa::pixel_format::RgbAFormat;
use nokhwa::utils::{ApiBackend, CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::{Camera, nokhwa_initialize, query};

use crate::infer::{BackendKind, InferenceOutput, InferenceTimings, PersonResult, PoseSegPipeline};

const INFER_INTERVAL_SECONDS: f64 = 0.03;
const KEYPOINT_SCORE_THRESHOLD: f32 = 0.1;
const MASK_ALPHA: f32 = 0.4;

pub struct CameraAppConfig {
    pub camera: u32,
    pub list_cameras: bool,
    pub backend: BackendKind,
    pub pose_model: String,
    pub seg_model: String,
    pub require_cuda: bool,
    pub enable_seg: bool,
    pub enable_profile: bool,
}

struct FrameReceiver(mpsc::Receiver<FrameData>);

#[derive(Resource)]
struct CameraImage(Handle<Image>);

#[derive(Resource)]
struct MaskImage(Handle<Image>);

#[derive(Component)]
struct CameraSprite;

#[derive(Component)]
struct MaskSprite;

struct FrameData {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

#[derive(Default, Resource)]
struct PoseDebug {
    keypoints_world: Vec<Vec2>,
    mask_rgba: Vec<u8>,
    mask_size: Option<(u32, u32)>,
    last_infer: f64,
    last_log: f64,
    last_pose_shape: Option<Vec<usize>>,
    last_seg_shape: Option<Vec<usize>>,
    last_proto_shape: Option<Vec<usize>>,
    people_count: usize,
    profile: ProfileStats,
}

#[derive(Default)]
struct ProfileStats {
    enabled: bool,
    sample_count: u32,
    sum_preprocess_ms: f64,
    sum_pose_ms: f64,
    sum_seg_ms: f64,
    sum_postprocess_ms: f64,
    sum_texture_ms: f64,
    sum_total_ms: f64,
}

pub fn run_camera_pose_app(config: CameraAppConfig) -> Result<()> {
    nokhwa_initialize(|_granted| {});

    if config.list_cameras {
        list_cameras();
        return Ok(());
    }

    let (tx, rx) = mpsc::sync_channel(1);
    spawn_capture_thread(config.camera, tx);

    let pipeline = PoseSegPipeline::new(
        config.backend,
        &config.pose_model,
        &config.seg_model,
        config.require_cuda,
        config.enable_seg,
    )?;

    let mut debug = PoseDebug::default();
    debug.profile.enabled = config.enable_profile;

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_non_send_resource(FrameReceiver(rx))
        .insert_non_send_resource(pipeline)
        .insert_resource(debug)
        .add_systems(Startup, setup)
        .add_systems(Update, apply_frame_and_infer)
        .add_systems(Update, draw_keypoints)
        .run();

    Ok(())
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.spawn(Camera2d);

    let camera_image = Image::new_fill(
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
    let camera_handle = images.add(camera_image);

    let mask_image = Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let mask_handle = images.add(mask_image);

    commands.spawn((Sprite::from_image(camera_handle.clone()), CameraSprite));
    commands.spawn((
        Sprite::from_image(mask_handle.clone()),
        MaskSprite,
        Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
    ));

    commands.insert_resource(CameraImage(camera_handle));
    commands.insert_resource(MaskImage(mask_handle));
}

fn apply_frame_and_infer(
    receiver: NonSend<FrameReceiver>,
    camera_image: Res<CameraImage>,
    mask_image: Res<MaskImage>,
    mut images: ResMut<Assets<Image>>,
    mut camera_sprite: Query<&mut Sprite, (With<CameraSprite>, Without<MaskSprite>)>,
    mut mask_sprite: Query<&mut Sprite, (With<MaskSprite>, Without<CameraSprite>)>,
    mut pipeline: NonSendMut<PoseSegPipeline>,
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
    let profile_enabled = debug.profile.enabled;
    let frame_timer = if profile_enabled {
        Some(Instant::now())
    } else {
        None
    };
    let mut texture_ms = 0.0;

    let extent = Extent3d {
        width: frame.width,
        height: frame.height,
        depth_or_array_layers: 1,
    };

    if profile_enabled {
        let tex_start = Instant::now();
        update_texture(
            &camera_image.0,
            &mut images,
            extent,
            TextureFormat::Rgba8UnormSrgb,
            frame.data,
        );
        texture_ms += tex_start.elapsed().as_secs_f64() * 1000.0;
    } else {
        update_texture(
            &camera_image.0,
            &mut images,
            extent,
            TextureFormat::Rgba8UnormSrgb,
            frame.data,
        );
    }

    if let Ok(mut sprite) = camera_sprite.single_mut() {
        let size = Vec2::new(frame.width as f32, frame.height as f32);
        if sprite.custom_size != Some(size) {
            sprite.custom_size = Some(size);
        }
    }
    if let Ok(mut sprite) = mask_sprite.single_mut() {
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

    let mut timings = InferenceTimings::default();
    let output_result = if profile_enabled {
        pipeline
            .infer_profiled(frame.width, frame.height, frame_for_infer)
            .map(|(output, timing)| {
                timings = timing;
                output
            })
    } else {
        pipeline.infer(frame.width, frame.height, frame_for_infer)
    };

    match output_result {
        Ok(output) => {
            debug.last_pose_shape = Some(output.pose_output_shape.clone());
            debug.last_seg_shape = Some(output.seg_output_shape.clone());
            debug.last_proto_shape = Some(output.proto_shape.clone());
            debug.people_count = output.people.len();
            debug.keypoints_world = build_world_keypoints(&output);
            if pipeline.seg_enabled() {
                debug.mask_rgba = build_mask_rgba(&output.people, output.frame_w, output.frame_h);
                debug.mask_size = Some((output.frame_w, output.frame_h));
                if profile_enabled {
                    let tex_start = Instant::now();
                    update_mask_texture(&mask_image.0, &mut images, &debug);
                    texture_ms += tex_start.elapsed().as_secs_f64() * 1000.0;
                } else {
                    update_mask_texture(&mask_image.0, &mut images, &debug);
                }
            }
        }
        Err(err) => {
            eprintln!("inference error: {err}");
        }
    }

    if profile_enabled {
        let total_ms = frame_timer
            .expect("profile timer should be set")
            .elapsed()
            .as_secs_f64()
            * 1000.0;
        debug.profile.sample_count += 1;
        debug.profile.sum_preprocess_ms += timings.preprocess_ms;
        debug.profile.sum_pose_ms += timings.pose_infer_ms;
        debug.profile.sum_seg_ms += timings.seg_infer_ms;
        debug.profile.sum_postprocess_ms += timings.postprocess_ms;
        debug.profile.sum_texture_ms += texture_ms;
        debug.profile.sum_total_ms += total_ms;
    }

    if now - debug.last_log >= 1.0 {
        debug.last_log = now;
        let pose_shape = format_shape(debug.last_pose_shape.as_ref());
        let seg_shape = format_shape(debug.last_seg_shape.as_ref());
        let proto_shape = format_shape(debug.last_proto_shape.as_ref());
        println!(
            "pose: people={} keypoints={} pose_shape={} seg_shape={} proto_shape={}",
            debug.people_count,
            debug.keypoints_world.len(),
            pose_shape,
            seg_shape,
            proto_shape
        );
        if debug.profile.enabled && debug.profile.sample_count > 0 {
            let denom = debug.profile.sample_count as f64;
            println!(
                "profile: pre={:.2} pose={:.2} seg={:.2} post={:.2} tex={:.2} total={:.2}",
                debug.profile.sum_preprocess_ms / denom,
                debug.profile.sum_pose_ms / denom,
                debug.profile.sum_seg_ms / denom,
                debug.profile.sum_postprocess_ms / denom,
                debug.profile.sum_texture_ms / denom,
                debug.profile.sum_total_ms / denom
            );
            debug.profile.sample_count = 0;
            debug.profile.sum_preprocess_ms = 0.0;
            debug.profile.sum_pose_ms = 0.0;
            debug.profile.sum_seg_ms = 0.0;
            debug.profile.sum_postprocess_ms = 0.0;
            debug.profile.sum_texture_ms = 0.0;
            debug.profile.sum_total_ms = 0.0;
        }
    }
}

fn update_texture(
    handle: &Handle<Image>,
    images: &mut Assets<Image>,
    extent: Extent3d,
    format: TextureFormat,
    data: Vec<u8>,
) {
    let image = images.get_mut(handle).expect("image should exist");
    if image.texture_descriptor.size != extent || image.texture_descriptor.format != format {
        *image = Image::new(
            extent,
            TextureDimension::D2,
            data,
            format,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
    } else {
        image.data = Some(data);
    }
}

fn update_mask_texture(handle: &Handle<Image>, images: &mut Assets<Image>, debug: &PoseDebug) {
    let Some((width, height)) = debug.mask_size else {
        return;
    };
    let extent = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    update_texture(
        handle,
        images,
        extent,
        TextureFormat::Rgba8UnormSrgb,
        debug.mask_rgba.clone(),
    );
}

fn build_world_keypoints(output: &InferenceOutput) -> Vec<Vec2> {
    let mut points = Vec::new();
    for person in &output.people {
        for kp in &person.keypoints {
            if kp.score < KEYPOINT_SCORE_THRESHOLD {
                continue;
            }
            let world = image_to_world(kp.x, kp.y, output.frame_w, output.frame_h);
            points.push(world);
        }
    }
    points
}

fn build_mask_rgba(people: &[PersonResult], frame_w: u32, frame_h: u32) -> Vec<u8> {
    let pixel_count = (frame_w * frame_h) as usize;
    let mut combined = vec![0.0f32; pixel_count];

    for person in people {
        let Some(mask) = &person.mask else {
            continue;
        };
        if mask.len() != pixel_count {
            continue;
        }
        for (i, value) in mask.iter().enumerate() {
            if *value > combined[i] {
                combined[i] = *value;
            }
        }
    }

    let mut rgba = vec![0u8; pixel_count * 4];
    for i in 0..pixel_count {
        let alpha = (combined[i] * MASK_ALPHA * 255.0).clamp(0.0, 255.0) as u8;
        let offset = i * 4;
        rgba[offset] = 0;
        rgba[offset + 1] = 255;
        rgba[offset + 2] = 0;
        rgba[offset + 3] = alpha;
    }

    rgba
}

fn image_to_world(x: f32, y: f32, w: u32, h: u32) -> Vec2 {
    let half_w = w as f32 / 2.0;
    let half_h = h as f32 / 2.0;
    Vec2::new(x - half_w, half_h - y)
}

fn format_shape(shape: Option<&Vec<usize>>) -> String {
    shape
        .map(|dims| {
            dims.iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join("x")
        })
        .unwrap_or_else(|| "unknown".to_string())
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
