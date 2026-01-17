use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::{
    AppState,
    pose::{PeopleDataRes, PoseRenderSettings},
};

use super::settings::FlagRaiseSettings;

const LINE_COLOR: Color = Color::srgb(0.7, 0.7, 0.8);
const SLOT_LINE_THICKNESS: f32 = 4.0;
const FLAG_WHITE_COLOR: Color = Color::srgb(0.97, 0.97, 1.0);
const FLAG_RED_COLOR: Color = Color::srgb(0.95, 0.2, 0.2);
const FLAG_SIZE: Vec2 = Vec2::new(40.0, 60.0);
const FLAG_POLE_THICKNESS: f32 = 4.0;
const FLAG_Z_OFFSET: f32 = 0.04;

#[derive(Resource)]
pub struct SlotLineAssets {
    rect: Handle<Mesh>,
    material: Handle<ColorMaterial>,
}

#[derive(Component)]
pub(super) struct SlotLineRoot;

#[derive(Resource)]
pub struct FlagRenderAssets {
    rect: Handle<Mesh>,
    white: Handle<ColorMaterial>,
    red: Handle<ColorMaterial>,
}

#[derive(Component)]
pub(super) struct FlagRenderPart;

pub fn setup_render_settings(render_settings: &mut PoseRenderSettings) {
    render_settings.enabled = true;
    render_settings.limb_thickness = 38.0;
    render_settings.torso_thickness = 52.0;
    render_settings.head_radius_scale = 0.35;
    render_settings.head_radius_min = 26.0;
    render_settings.head_radius_max = 96.0;
    render_settings.limb_color = Color::srgb(0.2, 0.88, 0.95);
    render_settings.torso_color = Color::srgb(0.98, 0.78, 0.2);
    render_settings.head_color = Color::srgb(1.0, 0.75, 0.2);
    render_settings.z_base = 1.0;
}

pub fn setup_slot_line_assets(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
) {
    commands.insert_resource(SlotLineAssets {
        rect: meshes.add(Rectangle::new(1.0, 1.0)),
        material: materials.add(LINE_COLOR),
    });
}

pub fn setup_flag_render_assets(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
) {
    commands.insert_resource(FlagRenderAssets {
        rect: meshes.add(Rectangle::new(1.0, 1.0)),
        white: materials.add(FLAG_WHITE_COLOR),
        red: materials.add(FLAG_RED_COLOR),
    });
}

pub(super) fn sync_slot_lines(
    mut commands: Commands,
    settings: Res<FlagRaiseSettings>,
    window: Query<&Window, With<PrimaryWindow>>,
    window_changed: Query<(), (With<PrimaryWindow>, Changed<Window>)>,
    assets: Option<Res<SlotLineAssets>>,
    existing: Query<Entity, With<SlotLineRoot>>,
) {
    if !settings.is_changed() && window_changed.is_empty() {
        return;
    }

    for entity in &existing {
        commands.entity(entity).despawn();
    }

    if settings.player_count <= 1 {
        return;
    }

    let Ok(window) = window.single() else {
        return;
    };
    let Some(assets) = assets else {
        return;
    };

    let frame_size = Vec2::new(window.resolution.width(), window.resolution.height());
    let half_width = frame_size.x * 0.5;
    let slot_width = frame_size.x / settings.player_count as f32;

    commands
        .spawn((
            SlotLineRoot,
            Transform::default(),
            GlobalTransform::default(),
            InheritedVisibility::default(),
            DespawnOnExit(AppState::FlagRaise),
        ))
        .with_children(|parent| {
            for i in 1..settings.player_count {
                let x = -half_width + slot_width * i as f32;
                parent.spawn((
                    Mesh2d(assets.rect.clone()),
                    MeshMaterial2d(assets.material.clone()),
                    Transform {
                        translation: Vec3::new(x, 0.0, 2.0),
                        rotation: Quat::IDENTITY,
                        scale: Vec3::new(SLOT_LINE_THICKNESS, frame_size.y, 1.0),
                    },
                ));
            }
        });
}

pub(super) fn draw_flags(
    mut commands: Commands,
    people: Res<PeopleDataRes>,
    window: Query<&Window, With<PrimaryWindow>>,
    assets: Option<Res<FlagRenderAssets>>,
    render_settings: Res<PoseRenderSettings>,
    existing: Query<Entity, With<FlagRenderPart>>,
) {
    let Ok(window) = window.single() else {
        return;
    };
    let Some(assets) = assets else {
        return;
    };
    let frame_size = Vec2::new(window.resolution.width(), window.resolution.height());
    let z = render_settings.z_base + FLAG_Z_OFFSET;

    for entity in &existing {
        commands.entity(entity).despawn();
    }

    for person in people.iter() {
        if let Some(right_wrist) = keypoint_world(&person.keypoints, 10, frame_size) {
            draw_flag(
                &mut commands,
                &assets.rect,
                &assets.white,
                right_wrist,
                FLAG_SIZE,
                FLAG_POLE_THICKNESS,
                z,
            );
        }

        if let Some(left_wrist) = keypoint_world(&person.keypoints, 9, frame_size) {
            draw_flag(
                &mut commands,
                &assets.rect,
                &assets.red,
                left_wrist,
                FLAG_SIZE,
                FLAG_POLE_THICKNESS,
                z,
            );
        }
    }
}

fn draw_flag(
    commands: &mut Commands,
    mesh: &Handle<Mesh>,
    material: &Handle<ColorMaterial>,
    origin: Vec2,
    size: Vec2,
    pole_thickness: f32,
    z: f32,
) {
    let pole_center = origin + Vec2::new(0.0, size.y * 0.5);
    let pole_size = Vec2::new(pole_thickness, size.y);
    spawn_rect(commands, mesh, material, pole_center, pole_size, z, 0.0);

    let flag_height = size.y * 0.6;
    let flag_center = origin + Vec2::new(size.x * 0.5, size.y * 0.6);
    let flag_size = Vec2::new(size.x, flag_height);
    spawn_rect(
        commands,
        mesh,
        material,
        flag_center,
        flag_size,
        z + 0.01,
        0.0,
    );
}

fn spawn_rect(
    commands: &mut Commands,
    mesh: &Handle<Mesh>,
    material: &Handle<ColorMaterial>,
    center: Vec2,
    size: Vec2,
    z: f32,
    rotation: f32,
) {
    commands.spawn((
        FlagRenderPart,
        Mesh2d(mesh.clone()),
        MeshMaterial2d(material.clone()),
        Transform {
            translation: Vec3::new(center.x, center.y, z),
            rotation: Quat::from_rotation_z(rotation),
            scale: Vec3::new(size.x, size.y, 1.0),
        },
    ));
}

fn keypoint_world(keypoints: &[Option<[f64; 2]>], index: usize, frame_size: Vec2) -> Option<Vec2> {
    let keypoint = keypoints.get(index).and_then(|kp| *kp)?;
    Some(Vec2::new(
        (keypoint[0] as f32 - 0.5) * frame_size.x,
        (0.5 - keypoint[1] as f32) * frame_size.y,
    ))
}
