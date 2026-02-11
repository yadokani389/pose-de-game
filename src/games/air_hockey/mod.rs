use std::cmp::Ordering;

use bevy::{
    asset::RenderAssetUsages,
    camera::{ScalingMode, Viewport, visibility::RenderLayers},
    math::{Rect, primitives::Triangle2d},
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
        enable_pose_frame_capture, enable_pose_runtime, estimate_center_x,
    },
};

const BOARD_WIDTH: f32 = 600.0;
const BOARD_HEIGHT: f32 = 600.0;
const BOARD_THICKNESS: f32 = 12.0;
const CENTER_LINE_THICKNESS: f32 = 6.0;
const GOAL_WIDTH: f32 = 210.0;
const CORNER_TRIANGLE_SIZE: f32 = 30.0;
const CORNER_TRIANGLE_Z: f32 = 1.0;
const WALL_INNER_OFFSET: f32 = BOARD_THICKNESS * 0.5;

const PUCK_RADIUS: f32 = 16.0;
const MALLET_RADIUS: f32 = 40.0;
const MALLET_MAX_SPEED: f32 = 2000.0;
const MALLET_VELOCITY_SAMPLES: usize = 3;
const MALLET_RESTITUTION: f32 = 1.0;
const PUCK_MAX_SPEED: f32 = 720.0;
const PUCK_DRAG_PER_SECOND: f32 = 0.9;
const PUCK_STOP_SPEED: f32 = 5.0;
const CAMERA_OVERLAY_ALPHA: f32 = 0.05;
const CAMERA_OVERLAY_Z: f32 = 0.5;
const SWEEP_EPS: f32 = 1.0e-5;
const SWEEP_OVERLAP_MARGIN: f32 = 0.5;
const LEFT_OVERLAY_LAYER: usize = 1;
const RIGHT_OVERLAY_LAYER: usize = 2;
const SCORE_LAYER: usize = 3;
const HAND_Y_SCALE: f32 = 2.0;
const LEFT_HAND_KEYPOINT: usize = 9;
const RIGHT_HAND_KEYPOINT: usize = 10;
const PLAYER_SIDE_SPLIT_X: f64 = 0.5;
const PLAYER_RELEASE_MARGIN_X: f64 = 0.08;
const SCORE_FONT_SIZE: f32 = 48.0;
const SCORE_EDGE_MARGIN: f32 = 20.0;
const KEYMAP_FONT_SIZE: f32 = 20.0;
const WIN_SCORE: u32 = 5;

const RESULT_HEADER_SIZE: f32 = 52.0;
const RESULT_TITLE_SIZE: f32 = 44.0;
const RESULT_DETAIL_SIZE: f32 = 24.0;
const RESULT_SCORE_SIZE: f32 = 32.0;
const RESULT_BUTTON_WIDTH: f32 = 320.0;
const RESULT_BUTTON_HEIGHT: f32 = 72.0;
const RESULT_BUTTON_TEXT_SIZE: f32 = 28.0;
const RESULT_BUTTON_TEXT_SIZE_SECONDARY: f32 = 24.0;
const RESULT_OVERLAY_COLOR: Color = Color::srgba(0.02, 0.02, 0.05, 0.85);
const RESULT_HEADER_COLOR: Color = Color::srgb(0.85, 0.9, 1.0);
const RESULT_DETAIL_COLOR: Color = Color::srgb(0.8, 0.8, 0.85);
const RESULT_BUTTON_NORMAL: Color = Color::srgb(0.15, 0.15, 0.15);
const RESULT_BUTTON_HOVERED: Color = Color::srgb(0.25, 0.25, 0.25);
const RESULT_BUTTON_PRESSED: Color = Color::srgb(0.35, 0.35, 0.35);

pub struct AirHockeyPlugin;

impl Plugin for AirHockeyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Scoreboard>()
            .init_resource::<MalletTargets>()
            .init_resource::<PlayerAssignments>()
            .init_resource::<CameraOverlayState>()
            .init_resource::<HandSelection>()
            .init_resource::<AirHockeyPhase>()
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
                    handle_overlay_toggle.run_if(is_playing),
                    handle_hand_toggle.run_if(is_playing),
                    update_viewports,
                    update_camera_overlay,
                    update_mallet_targets.run_if(is_playing),
                    move_mallets.run_if(is_playing),
                    move_puck.run_if(is_playing),
                    update_score_text,
                    spawn_result_ui.run_if(is_result),
                    result_input.run_if(is_result),
                    button_system.run_if(is_result),
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
    curr_pos: Vec2,
    velocity: Vec2,
    samples: [Vec2; MALLET_VELOCITY_SAMPLES],
    sample_index: usize,
    sample_count: usize,
}

impl MalletKinematics {
    fn new(initial_pos: Vec2) -> Self {
        Self {
            prev_pos: initial_pos,
            curr_pos: initial_pos,
            velocity: Vec2::ZERO,
            samples: [Vec2::ZERO; MALLET_VELOCITY_SAMPLES],
            sample_index: 0,
            sample_count: 0,
        }
    }

    fn update(&mut self, new_pos: Vec2, dt: f32) {
        let inst_velocity = if dt > 0.0 {
            (new_pos - self.curr_pos) / dt
        } else {
            Vec2::ZERO
        };

        self.prev_pos = self.curr_pos;
        self.curr_pos = new_pos;
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

    fn reset(&mut self, pos: Vec2) {
        self.prev_pos = pos;
        self.curr_pos = pos;
        self.velocity = Vec2::ZERO;
        self.samples = [Vec2::ZERO; MALLET_VELOCITY_SAMPLES];
        self.sample_index = 0;
        self.sample_count = 0;
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

#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
enum AirHockeyPhase {
    #[default]
    Playing,
    Result,
}

#[derive(Resource, Clone, Copy, Debug)]
struct AirHockeyResult {
    winner: PlayerSide,
    left_score: u32,
    right_score: u32,
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
            left: HandPreference::Left,
            right: HandPreference::Left,
        }
    }
}

#[derive(Resource)]
struct MalletTargets {
    left: Vec2,
    right: Vec2,
}

#[derive(Resource, Debug, Clone, Copy, Default)]
struct PlayerAssignments {
    left_player_id: Option<u64>,
    right_player_id: Option<u64>,
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

#[derive(Component)]
struct ResultRoot;

#[derive(Component)]
struct RetryButton;

#[derive(Component)]
struct MenuButton;

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
    mut player_assignments: ResMut<PlayerAssignments>,
    mut phase: ResMut<AirHockeyPhase>,
) {
    scoreboard.left = 0;
    scoreboard.right = 0;
    latest_frame.frame = None;
    overlay_state.visible = true;
    hand_selection.left = HandPreference::Left;
    hand_selection.right = HandPreference::Left;
    *player_assignments = PlayerAssignments::default();
    *phase = AirHockeyPhase::Playing;
    commands.remove_resource::<AirHockeyResult>();

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

    let corner_triangle_mesh = meshes.add(Mesh::from(Triangle2d::new(
        Vec2::ZERO,
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, 1.0),
    )));
    let corner_triangle_material = materials.add(Color::srgb(0.2, 0.2, 0.25));
    let half_width = BOARD_WIDTH * 0.5;
    let half_height = BOARD_HEIGHT * 0.5;
    let inner_half_width = half_width - WALL_INNER_OFFSET;
    let inner_half_height = half_height - WALL_INNER_OFFSET;
    let corner_triangles = [
        (Vec2::new(-inner_half_width, -inner_half_height), 0.0),
        (
            Vec2::new(inner_half_width, -inner_half_height),
            std::f32::consts::FRAC_PI_2,
        ),
        (
            Vec2::new(inner_half_width, inner_half_height),
            std::f32::consts::PI,
        ),
        (
            Vec2::new(-inner_half_width, inner_half_height),
            -std::f32::consts::FRAC_PI_2,
        ),
    ];
    for (pos, rotation) in corner_triangles {
        commands.spawn((
            Mesh2d(corner_triangle_mesh.clone()),
            MeshMaterial2d(corner_triangle_material.clone()),
            Transform::from_xyz(pos.x, pos.y, CORNER_TRIANGLE_Z)
                .with_rotation(Quat::from_rotation_z(rotation))
                .with_scale(Vec3::new(CORNER_TRIANGLE_SIZE, CORNER_TRIANGLE_SIZE, 1.0)),
            DespawnOnExit(AppState::AirHockey),
        ));
    }

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
        Velocity(Vec2::ZERO),
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
        Text2d::new("Esc/MouseBack: Menu / O: Overlay / Left/Right: Hand"),
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
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if input.just_pressed(KeyCode::Escape) || mouse_buttons.just_pressed(MouseButton::Back) {
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
        Single<&mut Sprite, With<LeftCameraOverlay>>,
        Single<&mut Sprite, With<RightCameraOverlay>>,
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

    let mut sprite = sprites.p0();
    if sprite.custom_size != Some(overlay_size) {
        sprite.custom_size = Some(overlay_size);
    }
    if sprite.rect != Some(left_rect) {
        sprite.rect = Some(left_rect);
    }

    let mut sprite = sprites.p1();
    if sprite.custom_size != Some(overlay_size) {
        sprite.custom_size = Some(overlay_size);
    }
    if sprite.rect != Some(right_rect) {
        sprite.rect = Some(right_rect);
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
    mut player_assignments: ResMut<PlayerAssignments>,
) {
    if !people.is_changed() && !selection.is_changed() {
        return;
    }

    let people_hands: Vec<PersonHands> = people
        .iter()
        .filter_map(|person| {
            let left = person.keypoints.get(LEFT_HAND_KEYPOINT).and_then(|kp| *kp);
            let right = person.keypoints.get(RIGHT_HAND_KEYPOINT).and_then(|kp| *kp);
            let center_x = estimate_center_x(&person.keypoints)?;
            if !center_x.is_finite() {
                return None;
            }
            Some(PersonHands {
                id: person.id,
                left,
                right,
                center_x,
            })
        })
        .collect();

    let current_ids: Vec<u64> = people.iter().map(|person| person.id).collect();

    if let Some(left_id) = player_assignments.left_player_id
        && !current_ids.contains(&left_id)
    {
        player_assignments.left_player_id = None;
    }

    if let Some(right_id) = player_assignments.right_player_id
        && !current_ids.contains(&right_id)
    {
        player_assignments.right_player_id = None;
    }

    if let Some(left_id) = player_assignments.left_player_id
        && let Some(center_x) = center_of_person(&people_hands, left_id)
        && is_center_in_camera(center_x)
        && is_out_of_player_side(center_x, PlayerSide::Left)
    {
        player_assignments.left_player_id = None;
    }

    if let Some(right_id) = player_assignments.right_player_id
        && let Some(center_x) = center_of_person(&people_hands, right_id)
        && is_center_in_camera(center_x)
        && is_out_of_player_side(center_x, PlayerSide::Right)
    {
        player_assignments.right_player_id = None;
    }

    if player_assignments.left_player_id.is_some()
        && player_assignments.left_player_id == player_assignments.right_player_id
    {
        player_assignments.right_player_id = None;
    }

    if player_assignments.left_player_id.is_none() {
        player_assignments.left_player_id = pick_side_player(
            &people_hands,
            PlayerSide::Left,
            player_assignments.right_player_id,
        );
    }

    if player_assignments.right_player_id.is_none() {
        player_assignments.right_player_id = pick_side_player(
            &people_hands,
            PlayerSide::Right,
            player_assignments.left_player_id,
        );
    }

    for person in &people_hands {
        if Some(person.id) == player_assignments.left_player_id {
            if let Some(hand) = select_hand(person, selection.left) {
                targets.left = map_hand_to_world(PlayerSide::Left, hand);
            }
            continue;
        }

        if Some(person.id) == player_assignments.right_player_id
            && let Some(hand) = select_hand(person, selection.right)
        {
            targets.right = map_hand_to_world(PlayerSide::Right, hand);
        }
    }
}

struct PersonHands {
    id: u64,
    left: Option<[f64; 2]>,
    right: Option<[f64; 2]>,
    center_x: f64,
}

fn center_of_person(people_hands: &[PersonHands], person_id: u64) -> Option<f64> {
    people_hands
        .iter()
        .find(|person| person.id == person_id)
        .map(|person| person.center_x)
}

fn is_center_in_camera(center_x: f64) -> bool {
    0.0 <= center_x && center_x <= 1.0
}

fn is_out_of_player_side(center_x: f64, side: PlayerSide) -> bool {
    match side {
        PlayerSide::Left => PLAYER_SIDE_SPLIT_X + PLAYER_RELEASE_MARGIN_X < center_x,
        PlayerSide::Right => center_x < PLAYER_SIDE_SPLIT_X - PLAYER_RELEASE_MARGIN_X,
    }
}

fn pick_side_player(
    people_hands: &[PersonHands],
    side: PlayerSide,
    excluded_id: Option<u64>,
) -> Option<u64> {
    let candidates = people_hands.iter().filter(|person| {
        if Some(person.id) == excluded_id {
            return false;
        }
        if !is_center_in_camera(person.center_x) {
            return false;
        }
        match side {
            PlayerSide::Left => person.center_x <= PLAYER_SIDE_SPLIT_X,
            PlayerSide::Right => PLAYER_SIDE_SPLIT_X <= person.center_x,
        }
    });

    match side {
        PlayerSide::Left => candidates
            .min_by(|a, b| {
                a.center_x
                    .partial_cmp(&b.center_x)
                    .unwrap_or(Ordering::Equal)
            })
            .map(|person| person.id),
        PlayerSide::Right => candidates
            .max_by(|a, b| {
                a.center_x
                    .partial_cmp(&b.center_x)
                    .unwrap_or(Ordering::Equal)
            })
            .map(|person| person.id),
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
    apply_corner_clamp(&mut pos, MALLET_RADIUS);
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
    mallets: Query<&MalletKinematics, Without<Puck>>,
    mut scoreboard: ResMut<Scoreboard>,
    mut phase: ResMut<AirHockeyPhase>,
    mut commands: Commands,
) {
    let Some((mut transform, mut velocity)) = puck_query.iter_mut().next() else {
        return;
    };

    let dt = time.delta_secs();
    if dt > 0.0 {
        let drag = PUCK_DRAG_PER_SECOND.powf(dt);
        velocity.0 *= drag;
    }
    let mut pos = transform.translation.truncate();
    let mut puck_start = pos;
    pos += velocity.0 * dt;

    let half_width = BOARD_WIDTH * 0.5;
    let half_height = BOARD_HEIGHT * 0.5;
    let goal_half_width = GOAL_WIDTH * 0.5;
    let mut had_impact = false;

    if half_width - PUCK_RADIUS < pos.x {
        pos.x = half_width - PUCK_RADIUS;
        velocity.0.x = -velocity.0.x.abs();
        had_impact = true;
    }
    if pos.x < -half_width + PUCK_RADIUS {
        pos.x = -half_width + PUCK_RADIUS;
        velocity.0.x = velocity.0.x.abs();
        had_impact = true;
    }

    if half_height - PUCK_RADIUS < pos.y {
        if goal_half_width < pos.x.abs() {
            pos.y = half_height - PUCK_RADIUS;
            velocity.0.y = -velocity.0.y.abs();
            had_impact = true;
        } else {
            scoreboard.left = scoreboard.left.saturating_add(1);
            if scoreboard.left >= WIN_SCORE {
                apply_result(
                    &mut commands,
                    &mut phase,
                    &mut velocity.0,
                    &mut pos,
                    PlayerSide::Left,
                    scoreboard.left,
                    scoreboard.right,
                );
                transform.translation.x = pos.x;
                transform.translation.y = pos.y;
                return;
            } else {
                reset_puck(&mut pos, &mut velocity.0, PlayerSide::Left);
                puck_start = pos;
            }
        }
    }

    if pos.y < -half_height + PUCK_RADIUS {
        if goal_half_width < pos.x.abs() {
            pos.y = -half_height + PUCK_RADIUS;
            velocity.0.y = velocity.0.y.abs();
            had_impact = true;
        } else {
            scoreboard.right = scoreboard.right.saturating_add(1);
            if scoreboard.right >= WIN_SCORE {
                apply_result(
                    &mut commands,
                    &mut phase,
                    &mut velocity.0,
                    &mut pos,
                    PlayerSide::Right,
                    scoreboard.left,
                    scoreboard.right,
                );
                transform.translation.x = pos.x;
                transform.translation.y = pos.y;
                return;
            } else {
                reset_puck(&mut pos, &mut velocity.0, PlayerSide::Right);
                puck_start = pos;
            }
        }
    }

    if bounce_off_corners(&mut pos, &mut velocity.0, PUCK_RADIUS) {
        had_impact = true;
    }

    let puck_end = pos;
    let min_dist = MALLET_RADIUS + PUCK_RADIUS;
    let mut best_hit: Option<SweptMalletHit> = None;
    for kinematics in mallets {
        let mallet_start = kinematics.prev_pos;
        let mallet_end = kinematics.curr_pos;
        if let Some(hit) =
            sweep_circle_hit(puck_start, puck_end, mallet_start, mallet_end, min_dist)
        {
            let entry = SweptMalletHit {
                t: hit.t,
                normal: hit.normal,
                mallet_pos: hit.mallet_pos,
                velocity: kinematics.velocity,
            };
            if best_hit.is_none_or(|best| entry.t < best.t) {
                best_hit = Some(entry);
            }
        }
    }

    if let Some(hit) = best_hit {
        pos = hit.mallet_pos + hit.normal * min_dist;
        let relative = velocity.0 - hit.velocity;
        let rel_dot = relative.dot(hit.normal);
        if rel_dot < 0.0 {
            let reflected = relative - (1.0 + MALLET_RESTITUTION) * rel_dot * hit.normal;
            velocity.0 = reflected + hit.velocity;
            had_impact = true;
            if hit.normal.y.abs() < SWEEP_EPS {
                // keep as-is when the normal is nearly horizontal
            } else if hit.normal.y < 0.0 {
                velocity.0.y = velocity.0.y.min(0.0);
            } else {
                velocity.0.y = velocity.0.y.max(0.0);
            }
        }
    }

    let speed = velocity.0.length();
    if PUCK_MAX_SPEED < speed {
        velocity.0 = velocity.0 / speed * PUCK_MAX_SPEED;
    }
    if dt > 0.0 && !had_impact && velocity.0.length() < PUCK_STOP_SPEED {
        velocity.0 = Vec2::ZERO;
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

#[derive(Clone, Copy)]
struct CornerBarrier {
    normal: Vec2,
    point: Vec2,
}

fn corner_barriers() -> [CornerBarrier; 4] {
    let half_width = BOARD_WIDTH * 0.5;
    let half_height = BOARD_HEIGHT * 0.5;
    let left = -half_width;
    let right = half_width;
    let bottom = -half_height;
    let top = half_height;
    let size = CORNER_TRIANGLE_SIZE;
    let inner_offset = WALL_INNER_OFFSET;

    [
        CornerBarrier {
            normal: Vec2::new(1.0, 1.0).normalize_or_zero(),
            point: Vec2::new(left + inner_offset + size, bottom + inner_offset),
        },
        CornerBarrier {
            normal: Vec2::new(-1.0, 1.0).normalize_or_zero(),
            point: Vec2::new(right - inner_offset - size, bottom + inner_offset),
        },
        CornerBarrier {
            normal: Vec2::new(1.0, -1.0).normalize_or_zero(),
            point: Vec2::new(left + inner_offset + size, top - inner_offset),
        },
        CornerBarrier {
            normal: Vec2::new(-1.0, -1.0).normalize_or_zero(),
            point: Vec2::new(right - inner_offset - size, top - inner_offset),
        },
    ]
}

fn apply_corner_clamp(pos: &mut Vec2, radius: f32) {
    for barrier in corner_barriers() {
        let dist = barrier.normal.dot(*pos - barrier.point);
        if dist < radius {
            *pos += barrier.normal * (radius - dist);
        }
    }
}

fn bounce_off_corners(pos: &mut Vec2, velocity: &mut Vec2, radius: f32) -> bool {
    let mut hit = false;
    for barrier in corner_barriers() {
        let dist = barrier.normal.dot(*pos - barrier.point);
        if dist < radius {
            *pos += barrier.normal * (radius - dist);
            let vel_dot = velocity.dot(barrier.normal);
            if vel_dot < 0.0 {
                *velocity -= 2.0 * vel_dot * barrier.normal;
                hit = true;
            }
        }
    }
    hit
}

#[derive(Clone, Copy)]
struct SweptHit {
    t: f32,
    normal: Vec2,
    mallet_pos: Vec2,
}

#[derive(Clone, Copy)]
struct SweptMalletHit {
    t: f32,
    normal: Vec2,
    mallet_pos: Vec2,
    velocity: Vec2,
}

fn sweep_circle_hit(
    puck_start: Vec2,
    puck_end: Vec2,
    mallet_start: Vec2,
    mallet_end: Vec2,
    radius: f32,
) -> Option<SweptHit> {
    let puck_delta = puck_end - puck_start;
    let mallet_delta = mallet_end - mallet_start;
    let relative_delta = puck_delta - mallet_delta;
    let start_delta = puck_start - mallet_start;
    let radius_sq = radius * radius;

    let a = relative_delta.length_squared();
    let c = start_delta.length_squared() - radius_sq;
    let t = if c <= 0.0 {
        let separating = start_delta.dot(relative_delta) >= 0.0;
        let overlap_margin_sq = SWEEP_OVERLAP_MARGIN * SWEEP_OVERLAP_MARGIN;
        let deep_overlap = c < -overlap_margin_sq;
        if separating && !deep_overlap {
            return None;
        }
        0.0
    } else if a <= SWEEP_EPS {
        return None;
    } else {
        let b = 2.0 * start_delta.dot(relative_delta);
        let disc = b * b - 4.0 * a * c;
        if disc < 0.0 {
            return None;
        }
        let sqrt_disc = disc.sqrt();
        let inv = 0.5 / a;
        let t1 = (-b - sqrt_disc) * inv;
        let t2 = (-b + sqrt_disc) * inv;
        if (0.0..=1.0).contains(&t1) {
            t1
        } else if (0.0..=1.0).contains(&t2) {
            t2
        } else {
            return None;
        }
    };

    let mallet_pos = mallet_start + mallet_delta * t;
    let puck_pos = puck_start + puck_delta * t;
    let mut normal = puck_pos - mallet_pos;
    if normal.length_squared() <= SWEEP_EPS {
        let fallback = if start_delta.length_squared() > SWEEP_EPS {
            start_delta
        } else if a > SWEEP_EPS {
            relative_delta
        } else {
            Vec2::Y
        };
        normal = fallback.normalize_or_zero();
    } else {
        normal = normal.normalize_or_zero();
    }

    if normal.length_squared() <= SWEEP_EPS {
        normal = Vec2::Y;
    }

    Some(SweptHit {
        t,
        normal,
        mallet_pos,
    })
}

fn reset_puck(pos: &mut Vec2, velocity: &mut Vec2, scorer: PlayerSide) {
    *pos = match scorer {
        PlayerSide::Left => Vec2::new(0.0, BOARD_HEIGHT * 0.25),
        PlayerSide::Right => Vec2::new(0.0, -BOARD_HEIGHT * 0.25),
    };
    *velocity = Vec2::ZERO;
}

fn apply_result(
    commands: &mut Commands,
    phase: &mut ResMut<AirHockeyPhase>,
    velocity: &mut Vec2,
    pos: &mut Vec2,
    winner: PlayerSide,
    left_score: u32,
    right_score: u32,
) {
    **phase = AirHockeyPhase::Result;
    *velocity = Vec2::ZERO;
    *pos = Vec2::ZERO;
    commands.insert_resource(AirHockeyResult {
        winner,
        left_score,
        right_score,
    });
}

fn update_score_text(scoreboard: Res<Scoreboard>, mut query: Query<&mut Text2d, With<ScoreText>>) {
    if !scoreboard.is_changed() {
        return;
    }
    if let Some(mut text) = query.iter_mut().next() {
        *text = Text2d::new(format!("{} - {}", scoreboard.left, scoreboard.right));
    }
}

fn spawn_result_ui(
    mut commands: Commands,
    ui_font: Res<UiFont>,
    result: Option<Res<AirHockeyResult>>,
    existing: Query<Entity, With<ResultRoot>>,
) {
    if !existing.is_empty() {
        return;
    }
    let Some(result) = result else {
        return;
    };

    let (title, title_color) = match result.winner {
        PlayerSide::Left => ("LEFT WIN", Color::srgb(0.4, 0.6, 1.0)),
        PlayerSide::Right => ("RIGHT WIN", Color::srgb(1.0, 0.5, 0.3)),
    };

    let detail = format!("First to {}", WIN_SCORE);
    let score_text = format!("{} - {}", result.left_score, result.right_score);

    commands
        .spawn((
            ResultRoot,
            DespawnOnExit(AppState::AirHockey),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(RESULT_OVERLAY_COLOR),
        ))
        .with_children(|parent| {
            parent.spawn((
                ResultRoot,
                Text::new("FINISH!"),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: RESULT_HEADER_SIZE,
                    ..default()
                },
                TextColor(RESULT_HEADER_COLOR),
                Node {
                    margin: UiRect::bottom(Val::Px(18.0)),
                    ..default()
                },
            ));

            parent.spawn((
                ResultRoot,
                Text::new(title),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: RESULT_TITLE_SIZE,
                    ..default()
                },
                TextColor(title_color),
                Node {
                    margin: UiRect::bottom(Val::Px(12.0)),
                    ..default()
                },
            ));

            parent.spawn((
                ResultRoot,
                Text::new(detail),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: RESULT_DETAIL_SIZE,
                    ..default()
                },
                TextColor(RESULT_DETAIL_COLOR),
                Node {
                    margin: UiRect::bottom(Val::Px(12.0)),
                    ..default()
                },
            ));

            parent.spawn((
                ResultRoot,
                Text::new(score_text),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: RESULT_SCORE_SIZE,
                    ..default()
                },
                TextColor(Color::srgb(0.92, 0.92, 0.96)),
                Node {
                    margin: UiRect::bottom(Val::Px(28.0)),
                    ..default()
                },
            ));

            parent
                .spawn((
                    ResultRoot,
                    Button,
                    RetryButton,
                    Node {
                        width: Val::Px(RESULT_BUTTON_WIDTH),
                        height: Val::Px(RESULT_BUTTON_HEIGHT),
                        border: UiRect::all(Val::Px(3.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BorderColor::all(Color::BLACK),
                    BorderRadius::MAX,
                    BackgroundColor(RESULT_BUTTON_NORMAL),
                ))
                .with_children(|button| {
                    button.spawn((
                        ResultRoot,
                        Text::new("Retry"),
                        TextFont {
                            font: ui_font.0.clone(),
                            font_size: RESULT_BUTTON_TEXT_SIZE,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    ));
                });

            parent
                .spawn((
                    ResultRoot,
                    Button,
                    MenuButton,
                    Node {
                        width: Val::Px(RESULT_BUTTON_WIDTH),
                        height: Val::Px(RESULT_BUTTON_HEIGHT),
                        border: UiRect::all(Val::Px(3.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        margin: UiRect::top(Val::Px(16.0)),
                        ..default()
                    },
                    BorderColor::all(Color::BLACK),
                    BorderRadius::MAX,
                    BackgroundColor(RESULT_BUTTON_NORMAL),
                ))
                .with_children(|button| {
                    button.spawn((
                        ResultRoot,
                        Text::new("Back to Menu"),
                        TextFont {
                            font: ui_font.0.clone(),
                            font_size: RESULT_BUTTON_TEXT_SIZE_SECONDARY,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    ));
                });
        });
}

fn result_input(
    input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut phase: ResMut<AirHockeyPhase>,
    mut scoreboard: ResMut<Scoreboard>,
    mut targets: ResMut<MalletTargets>,
    mut player_assignments: ResMut<PlayerAssignments>,
    mut queries: ParamSet<(
        Query<(&Mallet, &mut Transform, &mut MalletKinematics)>,
        Query<(&mut Transform, &mut Velocity), With<Puck>>,
    )>,
    result_ui: Query<Entity, With<ResultRoot>>,
) {
    if input.just_pressed(KeyCode::Space) {
        for entity in &result_ui {
            commands.entity(entity).despawn();
        }
        reset_match(
            &mut commands,
            &mut phase,
            &mut scoreboard,
            &mut targets,
            &mut player_assignments,
            &mut queries,
        );
    }
}

fn button_system(
    mut query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            Option<&RetryButton>,
            Option<&MenuButton>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_state: ResMut<NextState<AppState>>,
    mut phase: ResMut<AirHockeyPhase>,
    mut commands: Commands,
    mut scoreboard: ResMut<Scoreboard>,
    mut targets: ResMut<MalletTargets>,
    mut player_assignments: ResMut<PlayerAssignments>,
    mut queries: ParamSet<(
        Query<(&Mallet, &mut Transform, &mut MalletKinematics)>,
        Query<(&mut Transform, &mut Velocity), With<Puck>>,
    )>,
    result_ui: Query<Entity, With<ResultRoot>>,
) {
    for (interaction, mut color, mut border_color, retry, menu) in &mut query {
        match *interaction {
            Interaction::Pressed => {
                *color = RESULT_BUTTON_PRESSED.into();
                border_color.set_all(Color::srgb(0.9, 0.9, 0.9));
                if retry.is_some() {
                    for entity in &result_ui {
                        commands.entity(entity).despawn();
                    }
                    reset_match(
                        &mut commands,
                        &mut phase,
                        &mut scoreboard,
                        &mut targets,
                        &mut player_assignments,
                        &mut queries,
                    );
                } else if menu.is_some() {
                    next_state.set(AppState::MainMenu);
                }
            }
            Interaction::Hovered => {
                *color = RESULT_BUTTON_HOVERED.into();
                border_color.set_all(Color::WHITE);
            }
            Interaction::None => {
                *color = RESULT_BUTTON_NORMAL.into();
                border_color.set_all(Color::BLACK);
            }
        }
    }
}

fn reset_match(
    commands: &mut Commands,
    phase: &mut ResMut<AirHockeyPhase>,
    scoreboard: &mut ResMut<Scoreboard>,
    targets: &mut ResMut<MalletTargets>,
    player_assignments: &mut ResMut<PlayerAssignments>,
    queries: &mut ParamSet<(
        Query<(&Mallet, &mut Transform, &mut MalletKinematics)>,
        Query<(&mut Transform, &mut Velocity), With<Puck>>,
    )>,
) {
    **phase = AirHockeyPhase::Playing;
    commands.remove_resource::<AirHockeyResult>();
    scoreboard.left = 0;
    scoreboard.right = 0;
    **targets = MalletTargets::default();
    **player_assignments = PlayerAssignments::default();

    let left_mallet_pos = Vec2::new(0.0, -BOARD_HEIGHT * 0.25);
    let right_mallet_pos = Vec2::new(0.0, BOARD_HEIGHT * 0.25);
    let mut mallets = queries.p0();
    for (mallet, mut transform, mut kinematics) in mallets.iter_mut() {
        let pos = match mallet.side {
            PlayerSide::Left => left_mallet_pos,
            PlayerSide::Right => right_mallet_pos,
        };
        transform.translation.x = pos.x;
        transform.translation.y = pos.y;
        kinematics.reset(pos);
    }

    let mut puck = queries.p1();
    if let Some((mut transform, mut velocity)) = puck.iter_mut().next() {
        transform.translation.x = 0.0;
        transform.translation.y = 0.0;
        velocity.0 = Vec2::ZERO;
    }
}

fn is_playing(phase: Option<Res<AirHockeyPhase>>) -> bool {
    matches!(phase.as_deref(), Some(AirHockeyPhase::Playing))
}

fn is_result(phase: Option<Res<AirHockeyPhase>>) -> bool {
    matches!(phase.as_deref(), Some(AirHockeyPhase::Result))
}

fn update_score_ui_positions(
    cameras: Query<&Projection, With<ScoreCamera>>,
    mut score_text: Single<&mut Transform, With<ScoreText>>,
    mut keymap_text: Single<&mut Transform, (With<KeymapText>, Without<ScoreText>)>,
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

    score_text.translation.x = 0.0;
    score_text.translation.y = top;

    keymap_text.translation.x = right;
    keymap_text.translation.y = bottom;
}
