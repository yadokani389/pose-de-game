use bevy::color::Hsla;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::collections::{HashMap, HashSet};

use crate::pose::PeopleDataRes;

const FACE_KEYPOINTS: &[usize] = &[0, 1, 2, 3, 4];
const LIMB_EDGES: &[(usize, usize)] = &[
    (5, 7),
    (7, 9),
    (6, 8),
    (8, 10),
    (11, 13),
    (13, 15),
    (12, 14),
    (14, 16),
];
const HEAD_FACE_RADIUS_MULT: f32 = 2.4;
const PERSON_HUE_STEP: f32 = 137.5;

pub fn configure(app: &mut App) {
    app.init_resource::<PoseRenderSettings>()
        .init_resource::<PoseRenderAssets>()
        .init_resource::<PoseRenderMaterials>()
        .add_systems(
            Update,
            (update_pose_render_materials, render_pose_people).chain(),
        );
}

#[derive(Resource, Debug, Clone)]
pub struct PoseRenderSettings {
    pub enabled: bool,
    pub limb_thickness: f32,
    pub torso_thickness: f32,
    pub head_radius_scale: f32,
    pub head_radius_min: f32,
    pub head_radius_max: f32,
    pub limb_color: Color,
    pub torso_color: Color,
    pub head_color: Color,
    pub z_base: f32,
}

impl Default for PoseRenderSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            limb_thickness: 14.0,
            torso_thickness: 20.0,
            head_radius_scale: 0.35,
            head_radius_min: 12.0,
            head_radius_max: 48.0,
            limb_color: Color::srgb(0.9, 0.9, 0.95),
            torso_color: Color::srgb(0.74, 0.6, 0.42),
            head_color: Color::srgb(0.9, 0.85, 0.78),
            z_base: 1.0,
        }
    }
}

#[derive(Resource)]
struct PoseRenderAssets {
    rect: Handle<Mesh>,
    circle: Handle<Mesh>,
}

impl FromWorld for PoseRenderAssets {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let rect = meshes.add(Rectangle::new(1.0, 1.0));
        let circle = meshes.add(Circle::new(1.0));
        Self { rect, circle }
    }
}

#[derive(Resource, Default)]
struct PoseRenderMaterials {
    limb: Handle<ColorMaterial>,
    torso: Handle<ColorMaterial>,
    head: Handle<ColorMaterial>,
    person: HashMap<u64, PersonMaterials>,
    initialized: bool,
}

#[derive(Clone)]
struct PersonMaterials {
    limb: Handle<ColorMaterial>,
    torso: Handle<ColorMaterial>,
    head: Handle<ColorMaterial>,
}

#[derive(Component)]
struct PoseRenderPart;

pub fn enable_pose_render(mut settings: ResMut<PoseRenderSettings>) {
    settings.enabled = true;
}

pub fn disable_pose_render(mut settings: ResMut<PoseRenderSettings>) {
    settings.enabled = false;
}

fn update_pose_render_materials(
    settings: Res<PoseRenderSettings>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut render_materials: ResMut<PoseRenderMaterials>,
) {
    if render_materials.initialized && !settings.is_changed() {
        return;
    }

    update_material(
        &mut materials,
        &mut render_materials.limb,
        settings.limb_color,
    );
    update_material(
        &mut materials,
        &mut render_materials.torso,
        settings.torso_color,
    );
    update_material(
        &mut materials,
        &mut render_materials.head,
        settings.head_color,
    );

    render_materials.initialized = true;
}

fn update_material(
    materials: &mut Assets<ColorMaterial>,
    handle: &mut Handle<ColorMaterial>,
    color: Color,
) {
    if let Some(material) = materials.get_mut(&*handle) {
        material.color = color;
    } else {
        *handle = materials.add(color);
    }
}

fn render_pose_people(
    mut commands: Commands,
    settings: Res<PoseRenderSettings>,
    people: Res<PeopleDataRes>,
    window: Query<&Window, With<PrimaryWindow>>,
    assets: Res<PoseRenderAssets>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
    mut materials: ResMut<PoseRenderMaterials>,
    existing: Query<Entity, With<PoseRenderPart>>,
) {
    if !settings.enabled {
        for entity in &existing {
            commands.entity(entity).despawn();
        }
        return;
    }

    let Ok(window) = window.single() else {
        return;
    };
    let frame_size = Vec2::new(window.resolution.width(), window.resolution.height());

    for entity in &existing {
        commands.entity(entity).despawn();
    }

    if settings.is_changed() {
        clear_person_materials(&mut materials, &mut color_materials);
    }

    let mut used_ids = HashSet::new();
    for person in people.iter() {
        used_ids.insert(person.id);
        let person_materials =
            ensure_person_materials(&mut color_materials, &mut materials, &settings, person.id);
        spawn_person(
            &mut commands,
            &person.keypoints,
            frame_size,
            &settings,
            &assets,
            &person_materials,
        );
    }

    prune_person_materials(&mut materials, &mut color_materials, &used_ids);
}

fn spawn_person(
    commands: &mut Commands,
    keypoints: &[Option<[f64; 2]>],
    frame_size: Vec2,
    settings: &PoseRenderSettings,
    assets: &PoseRenderAssets,
    person_materials: &PersonMaterials,
) {
    draw_torso(
        commands,
        keypoints,
        frame_size,
        settings,
        assets,
        &person_materials.torso,
    );
    draw_limbs(
        commands,
        keypoints,
        frame_size,
        settings,
        assets,
        &person_materials.limb,
    );
    draw_head(
        commands,
        keypoints,
        frame_size,
        settings,
        assets,
        &person_materials.head,
    );
}

fn draw_torso(
    commands: &mut Commands,
    keypoints: &[Option<[f64; 2]>],
    frame_size: Vec2,
    settings: &PoseRenderSettings,
    assets: &PoseRenderAssets,
    material: &Handle<ColorMaterial>,
) {
    let Some(left_shoulder) = keypoint_world(keypoints, 5, frame_size) else {
        return;
    };
    let Some(right_shoulder) = keypoint_world(keypoints, 6, frame_size) else {
        return;
    };
    let Some(left_hip) = keypoint_world(keypoints, 11, frame_size) else {
        return;
    };
    let Some(right_hip) = keypoint_world(keypoints, 12, frame_size) else {
        return;
    };

    let shoulder_center = (left_shoulder + right_shoulder) * 0.5;
    let hip_center = (left_hip + right_hip) * 0.5;
    draw_segment(
        commands,
        assets,
        shoulder_center,
        hip_center,
        settings.torso_thickness,
        settings.z_base + 0.01,
        material,
    );
}

fn draw_limbs(
    commands: &mut Commands,
    keypoints: &[Option<[f64; 2]>],
    frame_size: Vec2,
    settings: &PoseRenderSettings,
    assets: &PoseRenderAssets,
    material: &Handle<ColorMaterial>,
) {
    for (start, end) in LIMB_EDGES {
        let Some(start_pos) = keypoint_world(keypoints, *start, frame_size) else {
            continue;
        };
        let Some(end_pos) = keypoint_world(keypoints, *end, frame_size) else {
            continue;
        };
        draw_segment(
            commands,
            assets,
            start_pos,
            end_pos,
            settings.limb_thickness,
            settings.z_base,
            material,
        );
    }
}

fn draw_head(
    commands: &mut Commands,
    keypoints: &[Option<[f64; 2]>],
    frame_size: Vec2,
    settings: &PoseRenderSettings,
    assets: &PoseRenderAssets,
    material: &Handle<ColorMaterial>,
) {
    let Some(center) = head_center(keypoints, frame_size) else {
        return;
    };
    let radius = head_radius(keypoints, frame_size, center, settings);
    draw_circle(
        commands,
        assets,
        center,
        radius,
        settings.z_base + 0.03,
        material,
    );
}

fn ensure_person_materials(
    materials: &mut Assets<ColorMaterial>,
    render_materials: &mut PoseRenderMaterials,
    settings: &PoseRenderSettings,
    person_id: u64,
) -> PersonMaterials {
    if let Some(existing) = render_materials.person.get(&person_id) {
        return existing.clone();
    }

    let hue_offset = person_hue_offset(person_id);
    let base_color = rotate_hue(settings.limb_color, hue_offset);

    let entry = PersonMaterials {
        limb: materials.add(base_color),
        torso: materials.add(base_color),
        head: materials.add(base_color),
    };
    render_materials.person.insert(person_id, entry.clone());
    entry
}

fn person_hue_offset(person_id: u64) -> f32 {
    (person_id as f32) * PERSON_HUE_STEP
}

fn rotate_hue(color: Color, hue_offset: f32) -> Color {
    let hsla: Hsla = color.into();
    Color::from(hsla.rotate_hue(hue_offset))
}

fn clear_person_materials(
    render_materials: &mut PoseRenderMaterials,
    materials: &mut Assets<ColorMaterial>,
) {
    for entry in render_materials.person.values() {
        materials.remove(entry.limb.id());
        materials.remove(entry.torso.id());
        materials.remove(entry.head.id());
    }
    render_materials.person.clear();
}

fn prune_person_materials(
    render_materials: &mut PoseRenderMaterials,
    materials: &mut Assets<ColorMaterial>,
    used_ids: &HashSet<u64>,
) {
    let mut stale_ids = Vec::new();
    for id in render_materials.person.keys() {
        if !used_ids.contains(id) {
            stale_ids.push(*id);
        }
    }

    for id in stale_ids {
        if let Some(entry) = render_materials.person.remove(&id) {
            materials.remove(entry.limb.id());
            materials.remove(entry.torso.id());
            materials.remove(entry.head.id());
        }
    }
}

fn draw_segment(
    commands: &mut Commands,
    assets: &PoseRenderAssets,
    start: Vec2,
    end: Vec2,
    thickness: f32,
    z: f32,
    color: &Handle<ColorMaterial>,
) {
    let center = (start + end) * 0.5;
    spawn_segment(
        commands,
        &assets.rect,
        color,
        center,
        end - start,
        thickness,
        z,
    );
}

fn draw_circle(
    commands: &mut Commands,
    assets: &PoseRenderAssets,
    center: Vec2,
    radius: f32,
    z: f32,
    color: &Handle<ColorMaterial>,
) {
    spawn_circle(commands, &assets.circle, color, center, radius, z);
}

fn spawn_segment(
    commands: &mut Commands,
    mesh: &Handle<Mesh>,
    material: &Handle<ColorMaterial>,
    center: Vec2,
    direction: Vec2,
    thickness: f32,
    z: f32,
) {
    let length = direction.length();
    if length <= f32::EPSILON {
        return;
    }
    let angle = direction.y.atan2(direction.x);
    spawn_rect(
        commands,
        mesh,
        material,
        center,
        Vec2::new(length, thickness),
        z,
        angle,
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
        PoseRenderPart,
        Mesh2d(mesh.clone()),
        MeshMaterial2d(material.clone()),
        Transform {
            translation: Vec3::new(center.x, center.y, z),
            rotation: Quat::from_rotation_z(rotation),
            scale: Vec3::new(size.x, size.y, 1.0),
        },
    ));
}

fn spawn_circle(
    commands: &mut Commands,
    mesh: &Handle<Mesh>,
    material: &Handle<ColorMaterial>,
    center: Vec2,
    radius: f32,
    z: f32,
) {
    commands.spawn((
        PoseRenderPart,
        Mesh2d(mesh.clone()),
        MeshMaterial2d(material.clone()),
        Transform {
            translation: Vec3::new(center.x, center.y, z),
            rotation: Quat::IDENTITY,
            scale: Vec3::splat(radius),
        },
    ));
}

fn normalized_to_world(keypoint: [f64; 2], frame_size: Vec2) -> Vec2 {
    Vec2::new(
        (keypoint[0] as f32 - 0.5) * frame_size.x,
        (0.5 - keypoint[1] as f32) * frame_size.y,
    )
}

fn keypoint_world(keypoints: &[Option<[f64; 2]>], index: usize, frame_size: Vec2) -> Option<Vec2> {
    let keypoint = keypoints.get(index).and_then(|kp| *kp)?;
    Some(normalized_to_world(keypoint, frame_size))
}

fn head_center(keypoints: &[Option<[f64; 2]>], frame_size: Vec2) -> Option<Vec2> {
    let mut sum = Vec2::ZERO;
    let mut count = 0.0;
    for index in FACE_KEYPOINTS {
        let Some(pos) = keypoint_world(keypoints, *index, frame_size) else {
            continue;
        };
        sum += pos;
        count += 1.0;
    }
    if count > 0.0 {
        return Some(sum / count);
    }

    if let (Some(left), Some(right)) = (
        keypoint_world(keypoints, 5, frame_size),
        keypoint_world(keypoints, 6, frame_size),
    ) {
        return Some((left + right) * 0.5);
    }

    None
}

fn head_radius(
    keypoints: &[Option<[f64; 2]>],
    frame_size: Vec2,
    center: Vec2,
    settings: &PoseRenderSettings,
) -> f32 {
    if let (Some(left), Some(right)) = (
        keypoint_world(keypoints, 5, frame_size),
        keypoint_world(keypoints, 6, frame_size),
    ) {
        let span = left.distance(right);
        let base = span * settings.head_radius_scale;
        return base.clamp(settings.head_radius_min, settings.head_radius_max);
    }

    let mut max_dist: f32 = 0.0;
    let mut found = false;
    for index in FACE_KEYPOINTS {
        let Some(pos) = keypoint_world(keypoints, *index, frame_size) else {
            continue;
        };
        max_dist = max_dist.max(center.distance(pos));
        found = true;
    }

    let base = if found {
        max_dist * HEAD_FACE_RADIUS_MULT
    } else {
        settings.limb_thickness * 1.4
    };
    base.clamp(settings.head_radius_min, settings.head_radius_max)
}
