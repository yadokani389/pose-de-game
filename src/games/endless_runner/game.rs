use std::cmp::Ordering;

use bevy::asset::RenderAssetUsages;
use bevy::camera::ScalingMode;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::{
    AppState,
    assets::UiFont,
    pose::{LatestFrameRes, PeopleDataRes},
};

use super::{CAMERA_USED_MARGIN, CAMERA_USED_PORTION, game_world_size, ui};

const NUM_LANES: usize = 3;
const SPAWN_DISTANCE: f32 = 600.0;
const DESPAWN_DISTANCE: f32 = -300.0;
const SPAWN_INTERVAL: f32 = 1.0;
const MIN_SPAWN_INTERVAL: f32 = 0.4;
const LANE_TRANSITION_SPEED: f32 = 8.0;

const SIZE_UNIT: f32 = 1.0 / 12.0;
const OBSTACLE_SIZE_RATIO: f32 = SIZE_UNIT;
const PLAYER_SIZE_RATIO: f32 = SIZE_UNIT * 0.8;
const LANE_WIDTH_RATIO: f32 = SIZE_UNIT * 3.2;

const CAMERA_OVERLAY_ALPHA_LANE: f32 = 0.15;
const CAMERA_OVERLAY_ALPHA_DARK: f32 = 0.6;
const CAMERA_OVERLAY_Z: f32 = 0.5;

const NOSE_MARKER_SIZE: f32 = 20.0;
const NOSE_MARKER_Z: f32 = 15.0;

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum EndlessRunnerPhase {
    #[default]
    Setup,
    Playing,
    Result,
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct GameSettings {
    pub num_players: usize,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self { num_players: 1 }
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerId {
    Player1,
    #[allow(dead_code)]
    Player2,
}

#[derive(Component)]
pub struct Player {
    pub id: PlayerId,
    pub current_lane: usize,
    pub target_lane: usize,
    pub is_alive: bool,
}

#[derive(Component)]
pub struct Obstacle {
    pub player_id: PlayerId,
    #[allow(dead_code)]
    pub lane: usize,
}

#[derive(Resource, Default)]
pub struct Scoreboard {
    pub player1_distance: f32,
    pub player2_distance: f32,
}

#[derive(Resource)]
pub struct GameSpeed {
    pub speed: f32,
}

impl Default for GameSpeed {
    fn default() -> Self {
        Self { speed: 300.0 }
    }
}

#[derive(Resource)]
pub struct ObstacleSpawner {
    timer: Timer,
    rng_state: u64,
}

impl Default for ObstacleSpawner {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(SPAWN_INTERVAL, TimerMode::Repeating),
            rng_state: 12345,
        }
    }
}

#[derive(Component)]
struct Background;

#[derive(Component)]
pub struct LaneLine;

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
pub struct CameraOverlay;

#[derive(Component)]
pub struct DarkOverlay;

#[derive(Resource)]
pub struct CameraOverlayImageHandle(pub Handle<Image>);

#[derive(Resource, Clone)]
pub struct LaneLayout {
    pub lane_width: f32,
    pub player_size: f32,
    pub obstacle_size: f32,
    pub collision_distance: f32,
    pub game_area_width: f32,
    pub game_height: f32,
    pub game_width: f32,
    pub dark_area_width: f32,

    pub player1_center_x: f32,
    pub player2_center_x: f32,
}

#[derive(Resource, Default)]
pub struct PlayerTargets {
    pub player1_lane: Option<usize>,
    pub player2_lane: Option<usize>,
    pub player1_nose: Option<(f32, f32)>,
    pub player2_nose: Option<(f32, f32)>,
}

#[derive(Component)]
pub struct NoseMarker {
    pub player_id: PlayerId,
}

#[derive(Component)]
pub struct PlayerGameOverText {
    pub player_id: PlayerId,
}

pub fn setup(
    mut commands: Commands,
    ui_font: Res<UiFont>,
    mut phase: ResMut<EndlessRunnerPhase>,
    mut scoreboard: ResMut<Scoreboard>,
    mut game_speed: ResMut<GameSpeed>,
    mut spawner: ResMut<ObstacleSpawner>,
    mut images: ResMut<Assets<Image>>,
    mut latest_frame: ResMut<LatestFrameRes>,
    window: Single<&Window>,
    settings: Res<GameSettings>,
) {
    *phase = EndlessRunnerPhase::Setup;
    scoreboard.player1_distance = 0.0;
    scoreboard.player2_distance = 0.0;
    *game_speed = GameSpeed::default();
    *spawner = ObstacleSpawner::default();
    commands.init_resource::<PlayerTargets>();
    latest_frame.frame = None;

    let game_size = game_world_size(&window);

    let obstacle_size = game_size.y * OBSTACLE_SIZE_RATIO;
    let player_size = game_size.y * PLAYER_SIZE_RATIO;
    let lane_width = game_size.y * LANE_WIDTH_RATIO;
    let game_area_width = lane_width * NUM_LANES as f32;

    let dark_area_width = (game_size.x - game_area_width) / 2.0;
    let collision_distance = (player_size + obstacle_size) * 0.4;

    let player1_center_x = -game_size.x / 4.0;
    let player2_center_x = game_size.x / 4.0;

    let lane_layout = LaneLayout {
        lane_width,
        player_size,
        obstacle_size,
        collision_distance,
        game_area_width,
        game_height: game_size.y,
        game_width: game_size.x,
        dark_area_width,
        player1_center_x,
        player2_center_x,
    };
    commands.insert_resource(lane_layout.clone());

    let projection = Projection::Orthographic(OrthographicProjection {
        scaling_mode: ScalingMode::Fixed {
            width: game_size.x,
            height: game_size.y,
        },
        ..OrthographicProjection::default_2d()
    });

    commands.spawn((
        Camera2d,
        projection,
        MainCamera,
        DespawnOnExit(AppState::EndlessRunner),
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

    commands.spawn((
        Sprite {
            image: overlay_handle.clone(),
            color: Color::srgba(1.0, 1.0, 1.0, CAMERA_OVERLAY_ALPHA_LANE),
            ..default()
        },
        CameraOverlay,
        Transform::from_xyz(0.0, 0.0, CAMERA_OVERLAY_Z),
        DespawnOnExit(AppState::EndlessRunner),
    ));

    let gradient_steps = 10;
    let bg_height = game_size.y * 1.5;
    for i in 0..gradient_steps {
        let t = i as f32 / gradient_steps as f32;
        let color = Color::srgb(
            0.1 * (1.0 - t) + 0.05 * t,
            0.1 * (1.0 - t) + 0.05 * t,
            0.12 * (1.0 - t) + 0.15 * t,
        );
        let y = bg_height * 0.5 - (bg_height / gradient_steps as f32) * i as f32;
        commands.spawn((
            Sprite::from_color(
                color,
                Vec2::new(game_size.x * 2.0, bg_height / gradient_steps as f32),
            ),
            Transform::from_xyz(0.0, y, 0.0),
            Background,
            DespawnOnExit(AppState::EndlessRunner),
        ));
    }

    spawn_lanes_and_overlays(&mut commands, &lane_layout, &settings);

    spawn_nose_markers(&mut commands, &settings);

    spawn_hud(&mut commands, &ui_font, &settings);

    super::setup::setup_ui(&mut commands, &ui_font, &settings);
}

pub fn spawn_lanes_and_overlays(
    commands: &mut Commands,
    layout: &LaneLayout,
    settings: &GameSettings,
) {
    let lane_line_height = layout.game_height * 2.0;
    let lane_line_width = layout.player_size * 0.08;
    let lane_line_color = Color::srgba(0.4, 0.4, 0.5, 0.6);

    if settings.num_players == 1 {
        let center_x = 0.0;

        for i in 1..NUM_LANES {
            let x = center_x + (i as f32 - NUM_LANES as f32 / 2.0) * layout.lane_width;
            commands.spawn((
                Sprite::from_color(
                    lane_line_color,
                    Vec2::new(lane_line_width, lane_line_height),
                ),
                Transform::from_xyz(x, 0.0, 1.0),
                LaneLine,
                DespawnOnExit(AppState::EndlessRunner),
            ));
        }

        let dark_color = Color::srgba(0.0, 0.0, 0.0, CAMERA_OVERLAY_ALPHA_DARK);
        let left_x = -layout.game_width / 2.0 + layout.dark_area_width / 2.0;
        let right_x = layout.game_width / 2.0 - layout.dark_area_width / 2.0;

        commands.spawn((
            Sprite::from_color(
                dark_color,
                Vec2::new(layout.dark_area_width, layout.game_height),
            ),
            Transform::from_xyz(left_x, 0.0, 4.0),
            DarkOverlay,
            DespawnOnExit(AppState::EndlessRunner),
        ));

        commands.spawn((
            Sprite::from_color(
                dark_color,
                Vec2::new(layout.dark_area_width, layout.game_height),
            ),
            Transform::from_xyz(right_x, 0.0, 4.0),
            DarkOverlay,
            DespawnOnExit(AppState::EndlessRunner),
        ));
    } else {
        let centers = [layout.player1_center_x, layout.player2_center_x];

        for &center_x in &centers {
            for i in 1..NUM_LANES {
                let x = center_x + (i as f32 - NUM_LANES as f32 / 2.0) * layout.lane_width;
                commands.spawn((
                    Sprite::from_color(
                        lane_line_color,
                        Vec2::new(lane_line_width, lane_line_height),
                    ),
                    Transform::from_xyz(x, 0.0, 1.0),
                    LaneLine,
                    DespawnOnExit(AppState::EndlessRunner),
                ));
            }
        }

        let dark_color = Color::srgba(0.0, 0.0, 0.0, CAMERA_OVERLAY_ALPHA_DARK);

        let left_edge = -layout.game_width / 2.0;
        let p1_left_lane = layout.player1_center_x - layout.game_area_width / 2.0;
        let left_dark_width = p1_left_lane - left_edge;
        if left_dark_width > 0.0 {
            commands.spawn((
                Sprite::from_color(dark_color, Vec2::new(left_dark_width, layout.game_height)),
                Transform::from_xyz(left_edge + left_dark_width / 2.0, 0.0, 4.0),
                DarkOverlay,
                DespawnOnExit(AppState::EndlessRunner),
            ));
        }

        let p1_right_lane = layout.player1_center_x + layout.game_area_width / 2.0;
        let p2_left_lane = layout.player2_center_x - layout.game_area_width / 2.0;
        let center_dark_width = p2_left_lane - p1_right_lane;
        if center_dark_width > 0.0 {
            commands.spawn((
                Sprite::from_color(dark_color, Vec2::new(center_dark_width, layout.game_height)),
                Transform::from_xyz(p1_right_lane + center_dark_width / 2.0, 0.0, 4.0),
                DarkOverlay,
                DespawnOnExit(AppState::EndlessRunner),
            ));
        }

        let right_edge = layout.game_width / 2.0;
        let p2_right_lane = layout.player2_center_x + layout.game_area_width / 2.0;
        let right_dark_width = right_edge - p2_right_lane;
        if right_dark_width > 0.0 {
            commands.spawn((
                Sprite::from_color(dark_color, Vec2::new(right_dark_width, layout.game_height)),
                Transform::from_xyz(p2_right_lane + right_dark_width / 2.0, 0.0, 4.0),
                DarkOverlay,
                DespawnOnExit(AppState::EndlessRunner),
            ));
        }
    }
}

pub fn spawn_nose_markers(commands: &mut Commands, settings: &GameSettings) {
    commands
        .spawn((
            Sprite::from_color(
                Color::srgba(1.0, 0.4, 0.7, 0.8),
                Vec2::new(NOSE_MARKER_SIZE, NOSE_MARKER_SIZE),
            ),
            Transform::from_xyz(0.0, 0.0, NOSE_MARKER_Z),
            Visibility::Hidden,
            NoseMarker {
                player_id: PlayerId::Player1,
            },
            DespawnOnExit(AppState::EndlessRunner),
        ))
        .with_children(|parent| {
            parent.spawn((
                Sprite::from_color(
                    Color::srgba(0.0, 0.0, 0.0, 0.0),
                    Vec2::new(NOSE_MARKER_SIZE * 0.5, NOSE_MARKER_SIZE * 0.5),
                ),
                Transform::from_xyz(0.0, 0.0, 0.1),
            ));
        });

    if settings.num_players == 2 {
        commands
            .spawn((
                Sprite::from_color(
                    Color::srgba(1.0, 0.6, 0.2, 0.8),
                    Vec2::new(NOSE_MARKER_SIZE, NOSE_MARKER_SIZE),
                ),
                Transform::from_xyz(0.0, 0.0, NOSE_MARKER_Z),
                Visibility::Hidden,
                NoseMarker {
                    player_id: PlayerId::Player2,
                },
                DespawnOnExit(AppState::EndlessRunner),
            ))
            .with_children(|parent| {
                parent.spawn((
                    Sprite::from_color(
                        Color::srgba(0.0, 0.0, 0.0, 0.0),
                        Vec2::new(NOSE_MARKER_SIZE * 0.5, NOSE_MARKER_SIZE * 0.5),
                    ),
                    Transform::from_xyz(0.0, 0.0, 0.1),
                ));
            });
    }
}

pub fn spawn_hud(commands: &mut Commands, ui_font: &UiFont, settings: &GameSettings) {
    if settings.num_players == 1 {
        commands.spawn((
            Text::new("0 m"),
            TextFont {
                font: ui_font.0.clone(),
                font_size: 36.0,
                ..default()
            },
            TextColor(Color::srgb(0.95, 0.95, 0.95)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(20.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            ui::DistanceText,
            ui::Player1DistanceText,
            DespawnOnExit(AppState::EndlessRunner),
        ));
    } else {
        commands.spawn((
            Text::new("P1: 0 m"),
            TextFont {
                font: ui_font.0.clone(),
                font_size: 32.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.4, 0.7)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(20.0),
                left: Val::Px(50.0),
                ..default()
            },
            ui::DistanceText,
            ui::Player1DistanceText,
            DespawnOnExit(AppState::EndlessRunner),
        ));

        commands.spawn((
            Text::new("P2: 0 m"),
            TextFont {
                font: ui_font.0.clone(),
                font_size: 32.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.6, 0.2)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(20.0),
                right: Val::Px(50.0),
                ..default()
            },
            ui::DistanceText,
            ui::Player2DistanceText,
            DespawnOnExit(AppState::EndlessRunner),
        ));
    }
}

pub fn cleanup(mut commands: Commands) {
    commands.remove_resource::<PlayerTargets>();
    commands.remove_resource::<LaneLayout>();
}

pub fn cleanup_camera_overlay(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    overlay_handle: Option<Res<CameraOverlayImageHandle>>,
) {
    if let Some(handle) = overlay_handle {
        images.remove(handle.0.id());
        commands.remove_resource::<CameraOverlayImageHandle>();
    }
}

pub fn update_camera_overlay(
    latest_frame: Res<LatestFrameRes>,
    overlay_handle: Option<Res<CameraOverlayImageHandle>>,
    mut images: ResMut<Assets<Image>>,
    mut overlay_sprites: Query<&mut Sprite, With<CameraOverlay>>,
    window: Single<&Window>,
) {
    let Some(overlay_handle) = overlay_handle else {
        return;
    };

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

    let frame_size = Vec2::new(frame.width as f32, frame.height as f32);
    let game_size = game_world_size(&window);

    for mut sprite in overlay_sprites.iter_mut() {
        sprite.custom_size = Some(game_size);
        sprite.rect = Some(Rect {
            min: Vec2::new(
                frame_size.x * CAMERA_USED_MARGIN,
                frame_size.y * CAMERA_USED_MARGIN,
            ),
            max: Vec2::new(
                frame_size.x * (1.0 - CAMERA_USED_MARGIN),
                frame_size.y * (1.0 - CAMERA_USED_MARGIN),
            ),
        });
    }
}

pub fn update_player_target_lane(
    people: Res<PeopleDataRes>,
    settings: Res<GameSettings>,
    layout: Option<Res<LaneLayout>>,
    mut targets: ResMut<PlayerTargets>,
) {
    if !people.is_changed() {
        return;
    }

    let Some(layout) = layout else { return };

    let assignments = assign_players(&people, &settings);

    if let Some((nose_x, nose_y)) = assignments.player1_nose {
        targets.player1_nose = Some((nose_x as f32, nose_y as f32));

        let world_x = nose_to_world_x(nose_x as f32, &layout);

        let center_x = if settings.num_players == 1 {
            0.0
        } else {
            layout.player1_center_x
        };

        let lane = world_x_to_lane(world_x, center_x, layout.lane_width);
        targets.player1_lane = Some(lane);
    } else {
        targets.player1_nose = None;
    }

    if let Some((nose_x, nose_y)) = assignments.player2_nose {
        targets.player2_nose = Some((nose_x as f32, nose_y as f32));

        let world_x = nose_to_world_x(nose_x as f32, &layout);

        let lane = world_x_to_lane(world_x, layout.player2_center_x, layout.lane_width);
        targets.player2_lane = Some(lane);
    } else {
        targets.player2_nose = None;
    }
}

pub fn update_nose_markers(
    targets: Res<PlayerTargets>,
    layout: Option<Res<LaneLayout>>,
    mut markers: Query<(&NoseMarker, &mut Transform, &mut Visibility)>,
) {
    let Some(layout) = layout else { return };

    for (marker, mut transform, mut visibility) in markers.iter_mut() {
        let nose_pos = match marker.player_id {
            PlayerId::Player1 => targets.player1_nose,
            PlayerId::Player2 => targets.player2_nose,
        };

        if let Some((nx, ny)) = nose_pos {
            *visibility = Visibility::Visible;

            let clamped_x = nx.clamp(CAMERA_USED_MARGIN, 1.0 - CAMERA_USED_MARGIN);
            let clamped_y = ny.clamp(CAMERA_USED_MARGIN, 1.0 - CAMERA_USED_MARGIN);

            let norm_x = (clamped_x - CAMERA_USED_MARGIN) / CAMERA_USED_PORTION;
            let norm_y = (clamped_y - CAMERA_USED_MARGIN) / CAMERA_USED_PORTION;

            let world_x = (norm_x - 0.5) * layout.game_width;
            let world_y = (0.5 - norm_y) * layout.game_height;

            transform.translation.x = world_x;
            transform.translation.y = world_y;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

pub fn move_players(
    time: Res<Time>,
    targets: Res<PlayerTargets>,
    lane_layout: Res<LaneLayout>,
    settings: Res<GameSettings>,
    mut query: Query<(&mut Player, &mut Transform)>,
) {
    let dt = time.delta_secs();

    for (mut player, mut transform) in query.iter_mut() {
        if !player.is_alive {
            continue;
        }

        match player.id {
            PlayerId::Player1 => {
                if let Some(lane) = targets.player1_lane {
                    player.target_lane = lane;
                }
            }
            PlayerId::Player2 => {
                if let Some(lane) = targets.player2_lane {
                    player.target_lane = lane;
                }
            }
        }

        let target_x = lane_to_x_for_player(
            player.target_lane,
            &lane_layout,
            player.id,
            settings.num_players,
        );
        let current_x = transform.translation.x;
        let diff = target_x - current_x;

        if diff.abs() > 1.0 {
            let move_amount = diff.signum() * LANE_TRANSITION_SPEED * lane_layout.lane_width * dt;
            let new_x = if move_amount.abs() > diff.abs() {
                target_x
            } else {
                current_x + move_amount
            };
            transform.translation.x = new_x;
        } else {
            transform.translation.x = target_x;
            player.current_lane = player.target_lane;
        }
    }
}

pub fn spawn_obstacles(
    mut commands: Commands,
    time: Res<Time>,
    lane_layout: Res<LaneLayout>,
    settings: Res<GameSettings>,
    scoreboard: Res<Scoreboard>,
    mut spawner: ResMut<ObstacleSpawner>,
    players: Query<&Player>,
) {
    let max_distance = scoreboard.player1_distance.max(scoreboard.player2_distance);
    let interval = (SPAWN_INTERVAL - max_distance / 100.0 * 0.1).max(MIN_SPAWN_INTERVAL);

    if (spawner.timer.duration().as_secs_f32() - interval).abs() > 0.01 {
        spawner
            .timer
            .set_duration(std::time::Duration::from_secs_f32(interval));
    }

    spawner.timer.tick(time.delta());

    if spawner.timer.just_finished() {
        for player in players.iter() {
            if !player.is_alive {
                continue;
            }

            let lane = random_lane(&mut spawner.rng_state);
            let x = lane_to_x_for_player(lane, &lane_layout, player.id, settings.num_players);

            commands.spawn((
                Sprite::from_color(
                    Color::srgb(0.9, 0.2, 0.2),
                    Vec2::new(lane_layout.obstacle_size, lane_layout.obstacle_size),
                ),
                Transform::from_xyz(x, SPAWN_DISTANCE, 5.0),
                Obstacle {
                    player_id: player.id,
                    lane,
                },
                DespawnOnExit(AppState::EndlessRunner),
            ));
        }
    }
}

pub fn move_obstacles(
    mut commands: Commands,
    time: Res<Time>,
    game_speed: Res<GameSpeed>,
    mut query: Query<(Entity, &mut Transform), With<Obstacle>>,
) {
    let dt = time.delta_secs();
    let move_amount = game_speed.speed * dt;

    for (entity, mut transform) in query.iter_mut() {
        transform.translation.y -= move_amount;

        if transform.translation.y < DESPAWN_DISTANCE {
            commands.entity(entity).despawn();
        }
    }
}

pub fn check_collisions(
    mut commands: Commands,
    lane_layout: Res<LaneLayout>,
    ui_font: Res<UiFont>,
    settings: Res<GameSettings>,
    mut players: Query<(&mut Player, &Transform, &mut Sprite)>,
    obstacles: Query<(&Obstacle, &Transform)>,
    existing_game_over: Query<&PlayerGameOverText>,
) {
    for (mut player, player_transform, mut sprite) in players.iter_mut() {
        if !player.is_alive {
            continue;
        }

        for (obstacle, obstacle_transform) in obstacles.iter() {
            if obstacle.player_id != player.id {
                continue;
            }

            let distance = player_transform
                .translation
                .truncate()
                .distance(obstacle_transform.translation.truncate());

            if distance < lane_layout.collision_distance {
                player.is_alive = false;

                sprite.color = Color::srgba(0.3, 0.3, 0.3, 0.5);

                let already_shown = existing_game_over
                    .iter()
                    .any(|go| go.player_id == player.id);

                if !already_shown {
                    spawn_player_game_over(
                        &mut commands,
                        &ui_font,
                        player.id,
                        &lane_layout,
                        &settings,
                    );
                }
                break;
            }
        }
    }
}

fn spawn_player_game_over(
    commands: &mut Commands,
    ui_font: &UiFont,
    player_id: PlayerId,
    layout: &LaneLayout,
    settings: &GameSettings,
) {
    let x = if settings.num_players == 1 {
        0.0
    } else {
        match player_id {
            PlayerId::Player1 => layout.player1_center_x,
            PlayerId::Player2 => layout.player2_center_x,
        }
    };

    commands.spawn((
        Text2d::new("GAME OVER"),
        TextFont {
            font: ui_font.0.clone(),
            font_size: 48.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.3, 0.3)),
        Transform::from_xyz(x, 0.0, 20.0),
        PlayerGameOverText { player_id },
        DespawnOnExit(AppState::EndlessRunner),
    ));
}

pub fn update_distances(
    time: Res<Time>,
    mut game_speed: ResMut<GameSpeed>,
    mut scoreboard: ResMut<Scoreboard>,
    players: Query<&Player>,
) {
    let dt = time.delta_secs();

    let max_distance = scoreboard.player1_distance.max(scoreboard.player2_distance);
    let base_speed = 300.0;

    let speed_multiplier = 1.5_f32.powf(max_distance / 100.0);
    game_speed.speed = base_speed * speed_multiplier;

    let distance_delta = game_speed.speed * dt / 100.0;

    for player in players.iter() {
        if !player.is_alive {
            continue;
        }

        match player.id {
            PlayerId::Player1 => scoreboard.player1_distance += distance_delta,
            PlayerId::Player2 => scoreboard.player2_distance += distance_delta,
        }
    }
}

pub fn check_game_over(mut phase: ResMut<EndlessRunnerPhase>, players: Query<&Player>) {
    let all_dead = players.iter().all(|p| !p.is_alive);

    if all_dead && !players.is_empty() {
        *phase = EndlessRunnerPhase::Result;
    }
}

fn get_nose_pos(keypoints: &[Option<[f64; 2]>]) -> Option<(f64, f64)> {
    if let Some(nose) = keypoints.first().and_then(|kp| *kp) {
        return Some((nose[0], nose[1]));
    }

    let left_eye = keypoints.get(1).and_then(|kp| *kp);
    let right_eye = keypoints.get(2).and_then(|kp| *kp);

    match (left_eye, right_eye) {
        (Some(l), Some(r)) => Some(((l[0] + r[0]) * 0.5, (l[1] + r[1]) * 0.5)),
        (Some(l), None) => Some((l[0], l[1])),
        (None, Some(r)) => Some((r[0], r[1])),
        (None, None) => None,
    }
}

struct PlayerAssignments {
    player1_nose: Option<(f64, f64)>,
    player2_nose: Option<(f64, f64)>,
}

fn assign_players(people: &PeopleDataRes, settings: &GameSettings) -> PlayerAssignments {
    let mut people_with_nose: Vec<(f64, f64)> = people
        .iter()
        .filter_map(|person| get_nose_pos(&person.keypoints))
        .collect();

    people_with_nose.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));

    match settings.num_players {
        1 => PlayerAssignments {
            player1_nose: people_with_nose.first().copied(),
            player2_nose: None,
        },
        2 => {
            let p1 = people_with_nose.iter().find(|&&(x, _)| x < 0.5).copied();
            let p2 = people_with_nose
                .iter()
                .rev()
                .find(|&&(x, _)| x >= 0.5)
                .copied();
            PlayerAssignments {
                player1_nose: p1,
                player2_nose: p2,
            }
        }
        _ => PlayerAssignments {
            player1_nose: None,
            player2_nose: None,
        },
    }
}

fn nose_to_world_x(nose_x: f32, layout: &LaneLayout) -> f32 {
    let clamped_x = nose_x.clamp(CAMERA_USED_MARGIN, 1.0 - CAMERA_USED_MARGIN);
    let norm_x = (clamped_x - CAMERA_USED_MARGIN) / CAMERA_USED_PORTION;

    (norm_x - 0.5) * layout.game_width
}

fn world_x_to_lane(world_x: f32, center_x: f32, lane_width: f32) -> usize {
    let local_x = world_x - center_x;

    if local_x < -0.5 * lane_width {
        0
    } else if local_x < 0.5 * lane_width {
        1
    } else {
        2
    }
}

pub fn lane_to_x_for_player(
    lane: usize,
    lane_layout: &LaneLayout,
    player_id: PlayerId,
    num_players: usize,
) -> f32 {
    let lane_offset = lane as f32 - (NUM_LANES as f32 - 1.0) / 2.0;
    let local_x = lane_offset * lane_layout.lane_width;

    if num_players == 1 {
        local_x
    } else {
        match player_id {
            PlayerId::Player1 => lane_layout.player1_center_x + local_x,
            PlayerId::Player2 => lane_layout.player2_center_x + local_x,
        }
    }
}

fn lcg(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    *state
}

fn random_f32(state: &mut u64) -> f32 {
    (lcg(state) as f32) / (u64::MAX as f32)
}

fn random_lane(state: &mut u64) -> usize {
    (random_f32(state) * NUM_LANES as f32) as usize
}

pub fn is_playing(phase: Option<Res<EndlessRunnerPhase>>) -> bool {
    matches!(phase.as_deref(), Some(EndlessRunnerPhase::Playing))
}

pub fn is_result(phase: Option<Res<EndlessRunnerPhase>>) -> bool {
    matches!(phase.as_deref(), Some(EndlessRunnerPhase::Result))
}
