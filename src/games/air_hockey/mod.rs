use std::cmp::Ordering;

use bevy::{
    asset::RenderAssetUsages,
    camera::{ScalingMode, Viewport, visibility::RenderLayers},
    math::Rect,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    sprite::Anchor,
    window::{PrimaryWindow, Window},
};

use crate::{
    AppState,
    assets::UiFont,
    pose::{
        LatestFrameRes, PeopleDataRes, disable_pose_frame_capture, disable_pose_runtime,
        enable_pose_frame_capture, enable_pose_runtime,
    },
};

const BOARD_WIDTH: f32 = 600.0;
const BOARD_HEIGHT: f32 = 600.0;
const BOARD_THICKNESS: f32 = 12.0;
const CENTER_LINE_THICKNESS: f32 = 6.0;
const GOAL_WIDTH: f32 = 150.0;

const PUCK_RADIUS: f32 = 16.0;
const MALLET_RADIUS: f32 = 30.0;
const MALLET_MAX_SPEED: f32 = 2000.0;
const MALLET_VELOCITY_SAMPLES: usize = 3;
const MALLET_RESTITUTION: f32 = 1.0;
const PUCK_START_SPEED: f32 = 360.0;
const PUCK_MAX_SPEED: f32 = 720.0;
const PUCK_DRAG_PER_SECOND: f32 = 0.9;
const PUCK_STOP_SPEED: f32 = 5.0;
const CAMERA_OVERLAY_ALPHA: f32 = 0.05;
const CAMERA_OVERLAY_Z: f32 = 0.5;
const LEFT_OVERLAY_LAYER: usize = 1;
const RIGHT_OVERLAY_LAYER: usize = 2;
const SCORE_LAYER: usize = 3;
const HAND_Y_SCALE: f32 = 2.0;
const LEFT_HAND_KEYPOINT: usize = 9;
const RIGHT_HAND_KEYPOINT: usize = 10;
const SCORE_FONT_SIZE: f32 = 48.0;
const SCORE_EDGE_MARGIN: f32 = 20.0;
const KEYMAP_FONT_SIZE: f32 = 20.0;

pub struct AirHockeyPlugin;

impl Plugin for AirHockeyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Scoreboard>()
            .init_resource::<MalletTargets>()
            .init_resource::<CameraOverlayState>()
            .init_resource::<HandSelection>()
            .add_systems(
                OnEnter(AppState::AirHockey),
                (setup, enable_pose_runtime, enable_pose_frame_capture),
            )
            .add_systems(
                OnExit(AppState::AirHockey),
                (
                    cleanup_camera_overlay,
                    disable_pose_frame_capture,
                    disable_pose_runtime,
                ),
            )
            .add_systems(
                Update,
                (
                    handle_escape_to_menu,
                    handle_overlay_toggle,
                    handle_hand_toggle,
                    update_viewports,
                    update_camera_overlay,
                    update_mallet_targets,
                    move_mallets,
                    move_puck,
                    update_score_text,
                )
                    .run_if(in_state(AppState::AirHockey)),
            );
        app.add_systems(
            PostUpdate,
            update_score_ui_positions
                .after(bevy::camera::CameraUpdateSystems)
                .run_if(in_state(AppState::AirHockey)),
        );
    }
}

#[derive(Component, Copy, Clone, Eq, PartialEq, Debug)]
enum PlayerSide {
    Left,
    Right,
}

#[derive(Component)]
struct Mallet {
    side: PlayerSide,
}

#[derive(Component, Clone, Copy)]
struct MalletKinematics {
    prev_pos: Vec2,
    velocity: Vec2,
    samples: [Vec2; MALLET_VELOCITY_SAMPLES],
    sample_index: usize,
    sample_count: usize,
}

impl MalletKinematics {
    fn new(initial_pos: Vec2) -> Self {
        Self {
            prev_pos: initial_pos,
            velocity: Vec2::ZERO,
            samples: [Vec2::ZERO; MALLET_VELOCITY_SAMPLES],
            sample_index: 0,
            sample_count: 0,
        }
    }

    fn update(&mut self, new_pos: Vec2, dt: f32) {
        let inst_velocity = if dt > 0.0 {
            (new_pos - self.prev_pos) / dt
        } else {
            Vec2::ZERO
        };

        self.prev_pos = new_pos;
        self.samples[self.sample_index] = inst_velocity;
        self.sample_index = (self.sample_index + 1) % MALLET_VELOCITY_SAMPLES;
        if self.sample_count < MALLET_VELOCITY_SAMPLES {
            self.sample_count += 1;
        }

        let mut avg = Vec2::ZERO;
        for i in 0..self.sample_count {
            avg += self.samples[i];
        }
        if self.sample_count > 0 {
            avg /= self.sample_count as f32;
        }

        self.velocity = clamp_vec2_length(avg, MALLET_MAX_SPEED);
    }
}

#[derive(Component)]
struct Puck;

#[derive(Component)]
struct Velocity(Vec2);

#[derive(Component)]
struct LeftCamera;

#[derive(Component)]
struct RightCamera;

#[derive(Component)]
struct LeftCameraOverlay;

#[derive(Component)]
struct RightCameraOverlay;

#[derive(Component)]
struct ScoreCamera;

#[derive(Resource, Default)]
struct Scoreboard {
    left: u32,
    right: u32,
}

#[derive(Resource, Debug, Clone, Copy)]
struct CameraOverlayState {
    visible: bool,
}

impl Default for CameraOverlayState {
    fn default() -> Self {
        Self { visible: true }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HandPreference {
    Left,
    Right,
}

#[derive(Resource, Debug, Clone, Copy)]
struct HandSelection {
    left: HandPreference,
    right: HandPreference,
}

impl Default for HandSelection {
    fn default() -> Self {
        Self {
            left: HandPreference::Right,
            right: HandPreference::Right,
        }
    }
}

#[derive(Resource)]
struct MalletTargets {
    left: Vec2,
    right: Vec2,
}

impl Default for MalletTargets {
    fn default() -> Self {
        let half_height = BOARD_HEIGHT * 0.5;
        Self {
            left: Vec2::new(0.0, -half_height * 0.5),
            right: Vec2::new(0.0, half_height * 0.5),
        }
    }
}

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct KeymapText;

#[derive(Resource)]
struct CameraOverlayImageHandle(Handle<Image>);

fn setup(
    mut commands: Commands,
    ui_font: Res<UiFont>,
    mut scoreboard: ResMut<Scoreboard>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut latest_frame: ResMut<LatestFrameRes>,
    mut overlay_state: ResMut<CameraOverlayState>,
    mut hand_selection: ResMut<HandSelection>,
) {
    scoreboard.left = 0;
    scoreboard.right = 0;
    latest_frame.frame = None;
    overlay_state.visible = true;
    hand_selection.left = HandPreference::Right;
    hand_selection.right = HandPreference::Right;

    let projection = Projection::Orthographic(OrthographicProjection {
        scaling_mode: ScalingMode::FixedHorizontal {
            viewport_width: BOARD_WIDTH,
        },
        ..OrthographicProjection::default_2d()
    });

    commands.spawn((
        Camera2d,
        Camera {
            order: 0,
            ..default()
        },
        LeftCamera,
        projection.clone(),
        RenderLayers::default().with(LEFT_OVERLAY_LAYER),
        DespawnOnExit(AppState::AirHockey),
    ));

    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            ..default()
        },
        RightCamera,
        projection.clone(),
        Transform::from_xyz(0.0, 0.0, 0.0)
            .with_rotation(Quat::from_rotation_z(std::f32::consts::PI)),
        RenderLayers::default().with(RIGHT_OVERLAY_LAYER),
        DespawnOnExit(AppState::AirHockey),
    ));

    commands.spawn((
        Camera2d,
        Camera {
            order: 2,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        ScoreCamera,
        // projection,
        RenderLayers::layer(SCORE_LAYER),
        DespawnOnExit(AppState::AirHockey),
    ));

    commands.spawn((
        Sprite::from_color(
            Color::srgb(0.08, 0.08, 0.1),
            Vec2::new(BOARD_WIDTH, BOARD_HEIGHT),
        ),
        Transform::from_xyz(0.0, 0.0, 0.0),
        DespawnOnExit(AppState::AirHockey),
    ));

    commands.spawn((
        Sprite::from_color(
            Color::srgb(0.2, 0.2, 0.25),
            Vec2::new(BOARD_WIDTH, BOARD_THICKNESS),
        ),
        Transform::from_xyz(0.0, BOARD_HEIGHT * 0.5, 1.0),
        DespawnOnExit(AppState::AirHockey),
    ));
    commands.spawn((
        Sprite::from_color(
            Color::srgb(0.2, 0.2, 0.25),
            Vec2::new(BOARD_WIDTH, BOARD_THICKNESS),
        ),
        Transform::from_xyz(0.0, -BOARD_HEIGHT * 0.5, 1.0),
        DespawnOnExit(AppState::AirHockey),
    ));
    commands.spawn((
        Sprite::from_color(
            Color::srgb(0.2, 0.2, 0.25),
            Vec2::new(BOARD_THICKNESS, BOARD_HEIGHT),
        ),
        Transform::from_xyz(BOARD_WIDTH * 0.5, 0.0, 1.0),
        DespawnOnExit(AppState::AirHockey),
    ));
    commands.spawn((
        Sprite::from_color(
            Color::srgb(0.2, 0.2, 0.25),
            Vec2::new(BOARD_THICKNESS, BOARD_HEIGHT),
        ),
        Transform::from_xyz(-BOARD_WIDTH * 0.5, 0.0, 1.0),
        DespawnOnExit(AppState::AirHockey),
    ));

    commands.spawn((
        Sprite::from_color(
            Color::srgb(0.35, 0.35, 0.4),
            Vec2::new(BOARD_WIDTH, CENTER_LINE_THICKNESS),
        ),
        Transform::from_xyz(0.0, 0.0, 2.0),
        DespawnOnExit(AppState::AirHockey),
    ));

    let goal_marker_size = Vec2::new(GOAL_WIDTH, BOARD_THICKNESS * 0.5);
    commands.spawn((
        Sprite::from_color(Color::srgb(0.8, 0.4, 0.3), goal_marker_size),
        Transform::from_xyz(0.0, BOARD_HEIGHT * 0.5, 2.0),
        DespawnOnExit(AppState::AirHockey),
    ));
    commands.spawn((
        Sprite::from_color(Color::srgb(0.3, 0.6, 0.9), goal_marker_size),
        Transform::from_xyz(0.0, -BOARD_HEIGHT * 0.5, 2.0),
        DespawnOnExit(AppState::AirHockey),
    ));

    let left_mallet_pos = Vec2::new(0.0, -BOARD_HEIGHT * 0.25);
    let right_mallet_pos = Vec2::new(0.0, BOARD_HEIGHT * 0.25);

    commands.spawn((
        Mallet {
            side: PlayerSide::Left,
        },
        MalletKinematics::new(left_mallet_pos),
        Mesh2d(meshes.add(Mesh::from(Circle::new(MALLET_RADIUS)))),
        MeshMaterial2d(materials.add(Color::srgb(0.4, 0.6, 1.0))),
        Transform::from_xyz(left_mallet_pos.x, left_mallet_pos.y, 5.0),
        DespawnOnExit(AppState::AirHockey),
    ));
    commands.spawn((
        Mallet {
            side: PlayerSide::Right,
        },
        MalletKinematics::new(right_mallet_pos),
        Mesh2d(meshes.add(Mesh::from(Circle::new(MALLET_RADIUS)))),
        MeshMaterial2d(materials.add(Color::srgb(1.0, 0.5, 0.3))),
        Transform::from_xyz(right_mallet_pos.x, right_mallet_pos.y, 5.0),
        DespawnOnExit(AppState::AirHockey),
    ));

    commands.spawn((
        Puck,
        Velocity(Vec2::new(0.0, PUCK_START_SPEED)),
        Mesh2d(meshes.add(Mesh::from(Circle::new(PUCK_RADIUS)))),
        MeshMaterial2d(materials.add(Color::WHITE)),
        Transform::from_xyz(0.0, 0.0, 4.0),
        DespawnOnExit(AppState::AirHockey),
    ));

    commands.spawn((
        ScoreText,
        Text2d::new("0 - 0"),
        TextFont {
            font: ui_font.0.clone(),
            font_size: SCORE_FONT_SIZE,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
        TextColor(Color::srgb(0.95, 0.95, 0.95)),
        Transform::from_xyz(0.0, BOARD_HEIGHT * 0.5 + 40.0, 10.0),
        RenderLayers::layer(SCORE_LAYER),
        Anchor::TOP_CENTER,
        DespawnOnExit(AppState::AirHockey),
    ));

    commands.spawn((
        KeymapText,
        Text2d::new("Esc: Menu / O: Overlay / Left/Right: Hand"),
        TextFont {
            font: ui_font.0.clone(),
            font_size: KEYMAP_FONT_SIZE,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
        TextColor(Color::srgb(0.8, 0.8, 0.8)),
        Transform::from_xyz(0.0, BOARD_HEIGHT * 0.5 + 10.0, 10.0),
        RenderLayers::layer(SCORE_LAYER),
        Anchor::BOTTOM_RIGHT,
        DespawnOnExit(AppState::AirHockey),
    ));

    let overlay_image = Image::new_fill(
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
    let overlay_handle = images.add(overlay_image);
    commands.insert_resource(CameraOverlayImageHandle(overlay_handle.clone()));

    let overlay_size = Vec2::new(BOARD_WIDTH, BOARD_HEIGHT * 0.5);
    let overlay_color = Color::srgba(1.0, 1.0, 1.0, CAMERA_OVERLAY_ALPHA);

    commands.spawn((
        Sprite {
            image: overlay_handle.clone(),
            color: overlay_color,
            custom_size: Some(overlay_size),
            ..default()
        },
        Anchor::TOP_CENTER,
        LeftCameraOverlay,
        RenderLayers::layer(LEFT_OVERLAY_LAYER),
        Transform::from_xyz(0.0, 0.0, CAMERA_OVERLAY_Z).with_scale(Vec3::new(
            1.0,
            HAND_Y_SCALE,
            1.0,
        )),
        DespawnOnExit(AppState::AirHockey),
    ));

    commands.spawn((
        Sprite {
            image: overlay_handle,
            color: overlay_color,
            custom_size: Some(overlay_size),
            flip_x: true,
            flip_y: true,
            ..default()
        },
        Anchor::BOTTOM_CENTER,
        RightCameraOverlay,
        RenderLayers::layer(RIGHT_OVERLAY_LAYER),
        Transform::from_xyz(0.0, 0.0, CAMERA_OVERLAY_Z).with_scale(Vec3::new(
            1.0,
            HAND_Y_SCALE,
            1.0,
        )),
        DespawnOnExit(AppState::AirHockey),
    ));
}

fn handle_escape_to_menu(
    input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if input.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::MainMenu);
    }
}

fn handle_overlay_toggle(
    input: Res<ButtonInput<KeyCode>>,
    mut overlay_state: ResMut<CameraOverlayState>,
    mut overlays: Query<&mut Visibility, Or<(With<LeftCameraOverlay>, With<RightCameraOverlay>)>>,
) {
    if !input.just_pressed(KeyCode::KeyO) {
        return;
    }

    overlay_state.visible = !overlay_state.visible;
    let visibility = if overlay_state.visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut overlay_visibility in overlays.iter_mut() {
        *overlay_visibility = visibility;
    }
}

fn handle_hand_toggle(input: Res<ButtonInput<KeyCode>>, mut selection: ResMut<HandSelection>) {
    if input.just_pressed(KeyCode::ArrowLeft) {
        selection.left = toggle_hand(selection.left);
    }
    if input.just_pressed(KeyCode::ArrowRight) {
        selection.right = toggle_hand(selection.right);
    }
}

fn toggle_hand(hand: HandPreference) -> HandPreference {
    match hand {
        HandPreference::Left => HandPreference::Right,
        HandPreference::Right => HandPreference::Left,
    }
}

fn update_viewports(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut left: Query<&mut Camera, (With<LeftCamera>, Without<RightCamera>)>,
    mut right: Query<&mut Camera, (With<RightCamera>, Without<LeftCamera>)>,
) {
    let Some(window) = windows.iter().next() else {
        return;
    };
    let width = window.physical_width();
    let height = window.physical_height();
    if width == 0 || height == 0 {
        return;
    }
    let half = width / 2;
    let left_viewport = Viewport {
        physical_position: UVec2::new(0, 0),
        physical_size: UVec2::new(half, height),
        depth: 0.0..1.0,
    };
    let right_viewport = Viewport {
        physical_position: UVec2::new(half, 0),
        physical_size: UVec2::new(width - half, height),
        depth: 0.0..1.0,
    };
    if let Some(mut camera) = left.iter_mut().next() {
        camera.viewport = Some(left_viewport);
    }
    if let Some(mut camera) = right.iter_mut().next() {
        camera.viewport = Some(right_viewport);
    }
}

fn update_camera_overlay(
    latest_frame: Res<LatestFrameRes>,
    overlay_handle: Res<CameraOverlayImageHandle>,
    mut images: ResMut<Assets<Image>>,
    mut sprites: ParamSet<(
        Query<&mut Sprite, With<LeftCameraOverlay>>,
        Query<&mut Sprite, With<RightCameraOverlay>>,
    )>,
) {
    if !latest_frame.is_changed() {
        return;
    }

    let Some(frame) = latest_frame.frame.as_ref() else {
        return;
    };

    let image = images
        .get_mut(&overlay_handle.0)
        .expect("camera overlay image should exist");

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

    let overlay_size = Vec2::new(BOARD_WIDTH, BOARD_HEIGHT * 0.5);
    let width = frame.width as f32;
    let height = frame.height as f32;
    let half_width = width * 0.5;
    let left_rect = Rect::new(0.0, 0.0, half_width, height);
    let right_rect = Rect::new(half_width, 0.0, width, height);

    if let Ok(mut sprite) = sprites.p0().single_mut() {
        if sprite.custom_size != Some(overlay_size) {
            sprite.custom_size = Some(overlay_size);
        }
        if sprite.rect != Some(left_rect) {
            sprite.rect = Some(left_rect);
        }
    }

    if let Ok(mut sprite) = sprites.p1().single_mut() {
        if sprite.custom_size != Some(overlay_size) {
            sprite.custom_size = Some(overlay_size);
        }
        if sprite.rect != Some(right_rect) {
            sprite.rect = Some(right_rect);
        }
    }
}

fn cleanup_camera_overlay(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    overlay_handle: Option<Res<CameraOverlayImageHandle>>,
) {
    if let Some(handle) = overlay_handle {
        images.remove(handle.0.id());
        commands.remove_resource::<CameraOverlayImageHandle>();
    }
}

fn update_mallet_targets(
    people: Res<PeopleDataRes>,
    selection: Res<HandSelection>,
    mut targets: ResMut<MalletTargets>,
) {
    if !people.is_changed() && !selection.is_changed() {
        return;
    }

    let mut people_hands: Vec<PersonHands> = people
        .iter()
        .filter_map(|person| {
            let left = person.keypoints.get(LEFT_HAND_KEYPOINT).and_then(|kp| *kp);
            let right = person.keypoints.get(RIGHT_HAND_KEYPOINT).and_then(|kp| *kp);
            let center_x = person_center_x(left, right)?;
            Some(PersonHands {
                left,
                right,
                center_x,
            })
        })
        .collect();

    people_hands.sort_by(|a, b| {
        a.center_x
            .partial_cmp(&b.center_x)
            .unwrap_or(Ordering::Equal)
    });

    match people_hands.len() {
        0 => {}
        1 => {
            let person = &people_hands[0];
            if person.center_x < 0.5 {
                if let Some(hand) = select_hand(person, selection.left) {
                    targets.left = map_hand_to_world(PlayerSide::Left, hand);
                }
            } else if let Some(hand) = select_hand(person, selection.right) {
                targets.right = map_hand_to_world(PlayerSide::Right, hand);
            }
        }
        _ => {
            let left_person = &people_hands[0];
            let right_person = &people_hands[people_hands.len() - 1];
            if let Some(hand) = select_hand(left_person, selection.left) {
                targets.left = map_hand_to_world(PlayerSide::Left, hand);
            }
            if let Some(hand) = select_hand(right_person, selection.right) {
                targets.right = map_hand_to_world(PlayerSide::Right, hand);
            }
        }
    }
}

struct PersonHands {
    left: Option<[f64; 2]>,
    right: Option<[f64; 2]>,
    center_x: f64,
}

fn person_center_x(left: Option<[f64; 2]>, right: Option<[f64; 2]>) -> Option<f64> {
    match (left, right) {
        (Some(l), Some(r)) => Some((l[0] + r[0]) * 0.5),
        (Some(l), None) => Some(l[0]),
        (None, Some(r)) => Some(r[0]),
        (None, None) => None,
    }
}

fn select_hand(person: &PersonHands, preference: HandPreference) -> Option<[f64; 2]> {
    match preference {
        HandPreference::Left => person.left,
        HandPreference::Right => person.right,
    }
}

fn map_hand_to_world(side: PlayerSide, hand: [f64; 2]) -> Vec2 {
    let x_n = hand[0] as f32;
    let y_n = hand[1] as f32;

    let x_local = match side {
        PlayerSide::Left => x_n * 2.0,
        PlayerSide::Right => (x_n - 0.5) * 2.0,
    }
    .clamp(0.0, 1.0);

    let y_local = (y_n * HAND_Y_SCALE).clamp(0.0, 1.0);

    let x_camera = (x_local - 0.5) * BOARD_WIDTH;
    let y_camera = -y_local * (BOARD_HEIGHT * 0.5);

    let mut world = match side {
        PlayerSide::Left => Vec2::new(x_camera, y_camera),
        PlayerSide::Right => Vec2::new(-x_camera, -y_camera),
    };

    world = clamp_mallet_pos(side, world);
    world
}

fn clamp_mallet_pos(side: PlayerSide, mut pos: Vec2) -> Vec2 {
    let half_width = BOARD_WIDTH * 0.5 - MALLET_RADIUS;
    let half_height = BOARD_HEIGHT * 0.5 - MALLET_RADIUS;

    pos.x = pos.x.clamp(-half_width, half_width);
    match side {
        PlayerSide::Left => {
            pos.y = pos.y.clamp(-half_height, -MALLET_RADIUS);
        }
        PlayerSide::Right => {
            pos.y = pos.y.clamp(MALLET_RADIUS, half_height);
        }
    }
    pos
}

fn move_mallets(
    time: Res<Time>,
    targets: Res<MalletTargets>,
    mut query: Query<(&Mallet, &mut Transform, &mut MalletKinematics)>,
) {
    let dt = time.delta_secs();
    for (mallet, mut transform, mut kinematics) in query.iter_mut() {
        let target = match mallet.side {
            PlayerSide::Left => targets.left,
            PlayerSide::Right => targets.right,
        };
        kinematics.update(target, dt);
        transform.translation.x = target.x;
        transform.translation.y = target.y;
    }
}

fn move_puck(
    time: Res<Time>,
    mut puck_query: Query<(&mut Transform, &mut Velocity), With<Puck>>,
    mallets: Query<(&Transform, &Mallet, &MalletKinematics), Without<Puck>>,
    mut scoreboard: ResMut<Scoreboard>,
) {
    let Some((mut transform, mut velocity)) = puck_query.iter_mut().next() else {
        return;
    };

    let dt = time.delta_secs();
    if dt > 0.0 {
        let drag = PUCK_DRAG_PER_SECOND.powf(dt);
        velocity.0 *= drag;
        if velocity.0.length() < PUCK_STOP_SPEED {
            velocity.0 = Vec2::ZERO;
        }
    }
    let mut pos = transform.translation.truncate();
    pos += velocity.0 * dt;

    let half_width = BOARD_WIDTH * 0.5;
    let half_height = BOARD_HEIGHT * 0.5;
    let goal_half_width = GOAL_WIDTH * 0.5;

    if half_width - PUCK_RADIUS < pos.x {
        pos.x = half_width - PUCK_RADIUS;
        velocity.0.x = -velocity.0.x.abs();
    }
    if pos.x < -half_width + PUCK_RADIUS {
        pos.x = -half_width + PUCK_RADIUS;
        velocity.0.x = velocity.0.x.abs();
    }

    if half_height - PUCK_RADIUS < pos.y {
        if goal_half_width < pos.x.abs() {
            pos.y = half_height - PUCK_RADIUS;
            velocity.0.y = -velocity.0.y.abs();
        } else {
            scoreboard.left = scoreboard.left.saturating_add(1);
            reset_puck(&mut pos, &mut velocity.0, PlayerSide::Left);
        }
    }

    if pos.y < -half_height + PUCK_RADIUS {
        if goal_half_width < pos.x.abs() {
            pos.y = -half_height + PUCK_RADIUS;
            velocity.0.y = velocity.0.y.abs();
        } else {
            scoreboard.right = scoreboard.right.saturating_add(1);
            reset_puck(&mut pos, &mut velocity.0, PlayerSide::Right);
        }
    }

    for (mallet_transform, mallet, kinematics) in mallets.iter() {
        let mallet_pos = mallet_transform.translation.truncate();
        let mallet_velocity = kinematics.velocity;
        let delta = pos - mallet_pos;
        let min_dist = MALLET_RADIUS + PUCK_RADIUS;
        let dist = delta.length();
        if dist < min_dist && dist != 0.0 {
            let normal = delta / dist;
            pos = mallet_pos + normal * min_dist;
            let relative = velocity.0 - mallet_velocity;
            let rel_dot = relative.dot(normal);
            if rel_dot < 0.0 {
                let reflected = relative - (1.0 + MALLET_RESTITUTION) * rel_dot * normal;
                velocity.0 = reflected + mallet_velocity;
                if mallet.side == PlayerSide::Left {
                    velocity.0.y = velocity.0.y.max(0.0);
                } else {
                    velocity.0.y = velocity.0.y.min(0.0);
                }
            }
        }
    }

    let speed = velocity.0.length();
    if PUCK_MAX_SPEED < speed {
        velocity.0 = velocity.0 / speed * PUCK_MAX_SPEED;
    }

    transform.translation.x = pos.x;
    transform.translation.y = pos.y;
}

fn clamp_vec2_length(vec: Vec2, max: f32) -> Vec2 {
    let len = vec.length();
    if len > max && len > 0.0 {
        vec / len * max
    } else {
        vec
    }
}

fn reset_puck(pos: &mut Vec2, velocity: &mut Vec2, scorer: PlayerSide) {
    *pos = match scorer {
        PlayerSide::Left => Vec2::new(0.0, BOARD_HEIGHT * 0.25),
        PlayerSide::Right => Vec2::new(0.0, -BOARD_HEIGHT * 0.25),
    };
    *velocity = Vec2::ZERO;
}

fn update_score_text(scoreboard: Res<Scoreboard>, mut query: Query<&mut Text2d, With<ScoreText>>) {
    if !scoreboard.is_changed() {
        return;
    }
    if let Some(mut text) = query.iter_mut().next() {
        *text = Text2d::new(format!("{} - {}", scoreboard.left, scoreboard.right));
    }
}

fn update_score_ui_positions(
    cameras: Query<&Projection, With<ScoreCamera>>,
    mut score_text: Query<&mut Transform, With<ScoreText>>,
    mut keymap_text: Query<&mut Transform, (With<KeymapText>, Without<ScoreText>)>,
) {
    let Some(projection) = cameras.iter().next() else {
        return;
    };

    let Projection::Orthographic(projection) = projection else {
        return;
    };
    let area = projection.area;
    let top = area.max.y - SCORE_EDGE_MARGIN;
    let right = area.max.x - SCORE_EDGE_MARGIN;
    let bottom = area.min.y + SCORE_EDGE_MARGIN;

    if let Ok(mut transform) = score_text.single_mut() {
        transform.translation.x = 0.0;
        transform.translation.y = top;
    }

    if let Ok(mut transform) = keymap_text.single_mut() {
        transform.translation.x = right;
        transform.translation.y = bottom;
    }
}
