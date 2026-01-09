use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use crate::AppState;
use crate::pose::{
    LatestFrameRes, PeopleDataRes, disable_pose_frame_capture, disable_pose_runtime,
    enable_pose_frame_capture, enable_pose_runtime,
};

const KEYPOINT_RADIUS: f32 = 6.0;
const KEYPOINT_COLOR: Color = Color::srgb(0.2, 1.0, 0.2);
const MASK_COLOR: [u8; 3] = [32, 255, 64];
const MASK_ALPHA: u8 = 64;

pub struct PoseDebugPlugin;

impl Plugin for PoseDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(AppState::PoseDebug),
            (enable_pose_runtime, enable_pose_frame_capture, setup),
        )
        .add_systems(
            OnExit(AppState::PoseDebug),
            (
                cleanup_debug_images,
                disable_pose_frame_capture,
                disable_pose_runtime,
            ),
        )
        .add_systems(
            Update,
            (
                handle_escape_to_menu,
                update_camera_view,
                draw_keypoints,
                update_mask_overlay,
            )
                .run_if(in_state(AppState::PoseDebug)),
        );
    }
}

#[derive(Resource)]
struct CameraImageHandle(Handle<Image>);

#[derive(Component)]
struct CameraSprite;

#[derive(Resource)]
struct MaskImageHandle(Handle<Image>);

#[derive(Component)]
struct MaskSprite;

fn handle_escape_to_menu(
    input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if input.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::MainMenu);
    }
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut latest_frame: ResMut<LatestFrameRes>,
) {
    commands.spawn((Camera2d, DespawnOnExit(AppState::PoseDebug)));
    latest_frame.frame = None;

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
    let handle = images.add(image);

    commands.spawn((
        Sprite::from_image(handle.clone()),
        CameraSprite,
        DespawnOnExit(AppState::PoseDebug),
    ));
    commands.insert_resource(CameraImageHandle(handle));

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
    commands.spawn((
        Sprite::from_image(mask_handle.clone()),
        MaskSprite,
        Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
        DespawnOnExit(AppState::PoseDebug),
    ));
    commands.insert_resource(MaskImageHandle(mask_handle));
}

fn cleanup_debug_images(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    camera_image: Option<Res<CameraImageHandle>>,
    mask_image: Option<Res<MaskImageHandle>>,
) {
    if let Some(handle) = camera_image {
        images.remove(handle.0.id());
        commands.remove_resource::<CameraImageHandle>();
    }
    if let Some(handle) = mask_image {
        images.remove(handle.0.id());
        commands.remove_resource::<MaskImageHandle>();
    }
}

fn update_camera_view(
    latest_frame: Res<LatestFrameRes>,
    camera_image: Res<CameraImageHandle>,
    mut images: ResMut<Assets<Image>>,
    mut sprite: Query<&mut Sprite, With<CameraSprite>>,
) {
    if !latest_frame.is_changed() {
        return;
    }

    let Some(frame) = latest_frame.frame.as_ref() else {
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
            frame.data.clone(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
    } else {
        match image.data.as_mut() {
            Some(data) if data.len() == frame.data.len() => {
                data.copy_from_slice(&frame.data);
            }
            _ => {
                image.data = Some(frame.data.clone());
            }
        }
    }

    if let Ok(mut sprite) = sprite.single_mut() {
        let size = Vec2::new(frame.width as f32, frame.height as f32);
        if sprite.custom_size != Some(size) {
            sprite.custom_size = Some(size);
        }
    }
}

fn draw_keypoints(
    latest_frame: Res<LatestFrameRes>,
    people: Res<PeopleDataRes>,
    mut gizmos: Gizmos,
) {
    let Some(frame) = latest_frame.frame.as_ref() else {
        return;
    };

    let width = frame.width as f32;
    let height = frame.height as f32;

    for person in people.iter() {
        for keypoint in &person.keypoints {
            let Some([x, y]) = keypoint else {
                continue;
            };
            let world_x = (*x as f32 - 0.5) * width;
            let world_y = (0.5 - *y as f32) * height;
            gizmos.circle_2d(Vec2::new(world_x, world_y), KEYPOINT_RADIUS, KEYPOINT_COLOR);
        }
    }
}

fn update_mask_overlay(
    args: Res<crate::args::Args>,
    people: Res<PeopleDataRes>,
    latest_frame: Res<LatestFrameRes>,
    mask_handle: Res<MaskImageHandle>,
    mut images: ResMut<Assets<Image>>,
    mut sprite: Query<&mut Sprite, With<MaskSprite>>,
) {
    let Some(frame) = latest_frame.frame.as_ref() else {
        return;
    };

    let width = frame.width;
    let height = frame.height;
    let pixel_count = (width * height) as usize;
    let extent = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    if !args.show_person {
        update_mask_texture(
            &mask_handle,
            &mut images,
            extent,
            vec![0u8; pixel_count * 4],
        );
        update_mask_sprite(&mut sprite, width, height);
        return;
    }

    if !people.is_changed() {
        return;
    }

    let mut rgba = vec![0u8; pixel_count * 4];
    let mut has_mask = false;
    for person in people.iter() {
        let Some(person_image) = &person.person_image else {
            continue;
        };
        let size = &person_image.texture_descriptor.size;
        if size.width != width || size.height != height {
            continue;
        }
        let Some(data) = person_image.data.as_ref() else {
            continue;
        };
        has_mask = true;
        for i in 0..pixel_count {
            let alpha = data[i * 4 + 3];
            if alpha == 0 {
                continue;
            }
            let offset = i * 4;
            if rgba[offset + 3] != 0 {
                continue;
            }
            rgba[offset] = MASK_COLOR[0];
            rgba[offset + 1] = MASK_COLOR[1];
            rgba[offset + 2] = MASK_COLOR[2];
            rgba[offset + 3] = MASK_ALPHA;
        }
    }

    if !has_mask {
        rgba.fill(0);
    }

    update_mask_texture(&mask_handle, &mut images, extent, rgba);
    update_mask_sprite(&mut sprite, width, height);
}

fn update_mask_texture(
    mask_handle: &MaskImageHandle,
    images: &mut Assets<Image>,
    extent: Extent3d,
    rgba: Vec<u8>,
) {
    let image = images
        .get_mut(&mask_handle.0)
        .expect("mask image should exist");
    if image.texture_descriptor.size != extent
        || image.texture_descriptor.format != TextureFormat::Rgba8UnormSrgb
    {
        *image = Image::new(
            extent,
            TextureDimension::D2,
            rgba,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
    } else {
        image.data = Some(rgba);
    }
}

fn update_mask_sprite(sprite: &mut Query<&mut Sprite, With<MaskSprite>>, width: u32, height: u32) {
    if let Ok(mut sprite) = sprite.single_mut() {
        let size = Vec2::new(width as f32, height as f32);
        if sprite.custom_size != Some(size) {
            sprite.custom_size = Some(size);
        }
    }
}
