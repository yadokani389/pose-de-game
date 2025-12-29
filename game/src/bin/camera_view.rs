use std::{sync::mpsc, thread, time::Duration};

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use clap::Parser;
use nokhwa::pixel_format::RgbAFormat;
use nokhwa::utils::{ApiBackend, CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::{Camera, nokhwa_initialize, query};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(long, default_value_t = 0)]
    camera: u32,
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

fn main() {
    let args = Args::parse();

    nokhwa_initialize(|_granted| {});

    if args.list_cameras {
        list_cameras();
        return;
    }

    let (tx, rx) = mpsc::sync_channel(1);
    spawn_capture_thread(args.camera, tx);

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_non_send_resource(FrameReceiver(rx))
        .add_systems(Startup, setup)
        .add_systems(Update, apply_frame)
        .run();
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

fn apply_frame(
    receiver: NonSend<FrameReceiver>,
    camera_image: Res<CameraImage>,
    mut images: ResMut<Assets<Image>>,
    mut sprite: Query<&mut Sprite, With<CameraSprite>>,
) {
    let mut latest = None;
    while let Ok(frame) = receiver.0.try_recv() {
        latest = Some(frame);
    }

    let Some(frame) = latest else {
        return;
    };

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
