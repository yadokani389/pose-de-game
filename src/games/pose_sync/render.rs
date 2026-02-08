use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::{AppState, args::Args, pose::PoseRenderSettings};

use super::{
    game::{
        self, CommandState, HOLE_EDGES, PoseTemplate, PoseTemplateId, RoundStage, SEQUENCE_LEN,
        SHOW_DRAW_EDGES,
    },
    settings::{Difficulty, PoseSyncSettings},
};

const LINE_COLOR: Color = Color::srgb(0.68, 0.68, 0.75);
const SLOT_LINE_THICKNESS: f32 = 4.0;
const PREVIEW_COLOR_IDLE: Color = Color::srgba(0.84, 0.84, 0.9, 0.6);
const PREVIEW_COLOR_ACTIVE: Color = Color::srgb(0.96, 0.96, 1.0);
const PREVIEW_COLOR_DONE: Color = Color::srgb(0.52, 0.95, 0.72);
const PREVIEW_Y_MARGIN: f32 = 320.0;
const PREVIEW_SCALE: f32 = 62.0;
const PREVIEW_LINE_THICKNESS: f32 = 18.0;
const PREVIEW_HEAD_RADIUS: f32 = 30.0;
const PREVIEW_STEP_SPACING: f32 = 290.0;

#[derive(Resource)]
pub struct SlotLineAssets {
    rect: Handle<Mesh>,
    material: Handle<ColorMaterial>,
}

#[derive(Resource)]
pub struct PosePreviewAssets {
    rect: Handle<Mesh>,
    circle: Handle<Mesh>,
    idle_material: Handle<ColorMaterial>,
    active_material: Handle<ColorMaterial>,
    done_material: Handle<ColorMaterial>,
}

#[derive(Component)]
pub(super) struct SlotLineRoot;

#[derive(Component)]
pub(super) struct PosePreviewPart;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct PreviewRenderState {
    stage: RoundStage,
    step_index: usize,
    sequence: [PoseTemplateId; SEQUENCE_LEN],
    difficulty: Difficulty,
    mirror_camera: bool,
}

pub fn setup_render_settings(render_settings: &mut PoseRenderSettings) {
    render_settings.enabled = true;
    render_settings.limb_thickness = 20.0;
    render_settings.torso_thickness = 26.0;
    render_settings.head_radius_scale = 0.36;
    render_settings.head_radius_min = 16.0;
    render_settings.head_radius_max = 64.0;
    render_settings.limb_color = Color::srgb(0.38, 0.78, 1.0);
    render_settings.torso_color = Color::srgb(0.55, 0.88, 1.0);
    render_settings.head_color = Color::srgb(0.8, 0.95, 1.0);
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

pub fn setup_pose_preview_assets(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
) {
    commands.insert_resource(PosePreviewAssets {
        rect: meshes.add(Rectangle::new(1.0, 1.0)),
        circle: meshes.add(Circle::new(1.0)),
        idle_material: materials.add(PREVIEW_COLOR_IDLE),
        active_material: materials.add(PREVIEW_COLOR_ACTIVE),
        done_material: materials.add(PREVIEW_COLOR_DONE),
    });
}

pub(super) fn sync_slot_lines(
    mut commands: Commands,
    settings: Res<PoseSyncSettings>,
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
            DespawnOnExit(AppState::PoseSync),
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

pub(super) fn draw_pose_preview(
    mut commands: Commands,
    args: Res<Args>,
    command: Res<CommandState>,
    settings: Res<PoseSyncSettings>,
    window: Query<&Window, With<PrimaryWindow>>,
    window_changed: Query<(), (With<PrimaryWindow>, Changed<Window>)>,
    assets: Option<Res<PosePreviewAssets>>,
    existing: Query<Entity, With<PosePreviewPart>>,
    mut last_state: Local<Option<PreviewRenderState>>,
) {
    let Some(assets) = assets else {
        return;
    };
    let Ok(window) = window.single() else {
        return;
    };

    let state = PreviewRenderState {
        stage: command.stage,
        step_index: command.step_index,
        sequence: command.sequence,
        difficulty: settings.difficulty,
        mirror_camera: args.mirror_camera,
    };

    if !existing.is_empty() && window_changed.is_empty() && last_state.as_ref() == Some(&state) {
        return;
    }
    *last_state = Some(state);

    for entity in &existing {
        commands.entity(entity).despawn();
    }

    let frame_size = Vec2::new(window.resolution.width(), window.resolution.height());
    let base_y = frame_size.y * 0.5 - PREVIEW_Y_MARGIN;
    let mirror_x = if args.mirror_camera { -1.0 } else { 1.0 };

    if settings.difficulty == Difficulty::Hard {
        if command.stage == RoundStage::Show {
            let base = Vec2::new(0.0, base_y);
            spawn_preview(
                &mut commands,
                &assets,
                command.active_pose(),
                base,
                assets.active_material.clone(),
                mirror_x,
                SHOW_DRAW_EDGES,
            );
        }
        return;
    }

    if command.stage == RoundStage::Intro {
        return;
    }

    let offset_start = -((SEQUENCE_LEN - 1) as f32) * PREVIEW_STEP_SPACING * 0.5;
    for idx in 0..SEQUENCE_LEN {
        let x = offset_start + idx as f32 * PREVIEW_STEP_SPACING;
        let base = Vec2::new(x, base_y);
        let pose = command.sequence[idx];
        let material = select_material(&assets, command.stage, command.step_index, idx);
        let edges = if command.stage == RoundStage::Show || command.stage == RoundStage::Repeat {
            SHOW_DRAW_EDGES
        } else {
            HOLE_EDGES
        };
        spawn_preview(
            &mut commands,
            &assets,
            pose,
            base,
            material,
            mirror_x,
            edges,
        );
    }
}

fn spawn_preview(
    commands: &mut Commands,
    assets: &PosePreviewAssets,
    pose_id: PoseTemplateId,
    base: Vec2,
    material: Handle<ColorMaterial>,
    mirror_x: f32,
    edges: &[(usize, usize)],
) {
    let pose = game::template(pose_id);
    for (start, end) in edges {
        let Some(a) = pose_point(pose, *start) else {
            continue;
        };
        let Some(b) = pose_point(pose, *end) else {
            continue;
        };
        let start = base + Vec2::new(a.x * mirror_x, a.y) * PREVIEW_SCALE;
        let end = base + Vec2::new(b.x * mirror_x, b.y) * PREVIEW_SCALE;
        let center = (start + end) * 0.5;
        let delta = end - start;
        let length = delta.length();
        if length <= f32::EPSILON {
            continue;
        }
        commands.spawn((
            PosePreviewPart,
            Mesh2d(assets.rect.clone()),
            MeshMaterial2d(material.clone()),
            Transform {
                translation: Vec3::new(center.x, center.y, 10.0),
                rotation: Quat::from_rotation_z(delta.y.atan2(delta.x)),
                scale: Vec3::new(length, PREVIEW_LINE_THICKNESS, 1.0),
            },
            DespawnOnExit(AppState::PoseSync),
        ));
    }

    let head = Vec2::new(pose.head_center[0] * mirror_x, pose.head_center[1]);
    let head_pos = base + head * PREVIEW_SCALE;
    commands.spawn((
        PosePreviewPart,
        Mesh2d(assets.circle.clone()),
        MeshMaterial2d(material),
        Transform {
            translation: Vec3::new(head_pos.x, head_pos.y, 10.1),
            rotation: Quat::IDENTITY,
            scale: Vec3::splat(PREVIEW_HEAD_RADIUS),
        },
        DespawnOnExit(AppState::PoseSync),
    ));
}

fn select_material(
    assets: &PosePreviewAssets,
    stage: RoundStage,
    step_index: usize,
    index: usize,
) -> Handle<ColorMaterial> {
    match stage {
        RoundStage::Intro => assets.idle_material.clone(),
        RoundStage::Show => {
            if index == step_index {
                assets.active_material.clone()
            } else {
                assets.idle_material.clone()
            }
        }
        RoundStage::Repeat => {
            if index <= step_index {
                assets.done_material.clone()
            } else {
                assets.idle_material.clone()
            }
        }
    }
}

fn pose_point(pose: &PoseTemplate, index: usize) -> Option<Vec2> {
    pose.targets
        .iter()
        .find(|target| target.index == index)
        .map(|target| Vec2::new(target.pos[0], target.pos[1]))
}
