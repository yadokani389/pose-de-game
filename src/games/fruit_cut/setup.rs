use bevy::asset::RenderAssetUsages;
use bevy::camera::ScalingMode;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::{AppState, assets::UiFont, pose::LatestFrameRes};

use super::{
    CAMERA_USED_MARGIN,
    game::{
        ComboState, FruitCutEntity, FruitCutPhase, FruitCutSettings, FruitSpawner, GameTimer,
        MAX_PLAYERS, MIN_PLAYERS, Scoreboard,
    },
    game_world_size,
    hand_tracker::HandTrackers,
};

const CAMERA_OVERLAY_ALPHA: f32 = 0.15;
const CAMERA_OVERLAY_Z: f32 = 0.5;
const SETUP_TITLE_SIZE: f32 = 58.0;
const SETUP_LABEL_SIZE: f32 = 36.0;
const SETUP_VALUE_SIZE: f32 = 44.0;
const SETUP_HINT_SIZE: f32 = 28.0;
const CENTER_LINE_THICKNESS: f32 = 4.0;
const CENTER_LINE_COLOR: Color = Color::srgb(0.7, 0.7, 0.8);

#[derive(Component)]
pub struct SetupText;

#[derive(Component)]
pub struct PlayerCountText;

#[derive(Component)]
pub struct CameraOverlay;

#[derive(Component)]
pub struct CenterLine;

#[derive(Resource)]
pub struct CameraOverlayImageHandle(pub Handle<Image>);

pub fn is_setup(phase: Res<FruitCutPhase>) -> bool {
    *phase == FruitCutPhase::Setup
}

pub fn spawn_setup_ui(commands: &mut Commands, ui_font: &UiFont, settings: &FruitCutSettings) {
    commands
        .spawn((
            SetupText,
            FruitCutEntity,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("フルーツカット"),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: SETUP_TITLE_SIZE,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.95, 0.98)),
                Node {
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::new("プレイヤー数"),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: SETUP_LABEL_SIZE,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.85)),
                Node {
                    margin: UiRect::bottom(Val::Px(10.0)),
                    ..default()
                },
            ));

            parent.spawn((
                PlayerCountText,
                Text::new(format!("{}", settings.player_count)),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: SETUP_VALUE_SIZE,
                    ..default()
                },
                TextColor(Color::srgb(0.3, 0.9, 1.0)),
                Node {
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::new("↑↓: プレイヤー数変更"),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: SETUP_HINT_SIZE,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.8)),
                Node {
                    margin: UiRect::bottom(Val::Px(8.0)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::new("Enter: 開始  Esc: 戻る"),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: SETUP_HINT_SIZE,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.8)),
            ));
        });
}

pub fn setup(
    mut commands: Commands,
    mut phase: ResMut<FruitCutPhase>,
    mut settings: ResMut<FruitCutSettings>,
    mut scoreboard: ResMut<Scoreboard>,
    mut combo: ResMut<ComboState>,
    mut game_timer: ResMut<GameTimer>,
    mut spawner: ResMut<FruitSpawner>,
    mut hand_trackers: ResMut<HandTrackers>,
    mut latest_frame: ResMut<LatestFrameRes>,
    ui_font: Res<UiFont>,
    mut images: ResMut<Assets<Image>>,
    window: Single<&Window>,
) {
    *phase = FruitCutPhase::Setup;
    *settings = FruitCutSettings::default();
    *scoreboard = Scoreboard::default();
    *combo = ComboState::default();
    *game_timer = GameTimer::default();
    *spawner = FruitSpawner::default();
    *hand_trackers = HandTrackers::default();
    latest_frame.frame = None;

    let game_size = game_world_size(&window);
    let projection = Projection::Orthographic(OrthographicProjection {
        scaling_mode: ScalingMode::Fixed {
            width: game_size.x,
            height: game_size.y,
        },
        ..OrthographicProjection::default_2d()
    });

    commands.spawn((Camera2d, projection, FruitCutEntity));

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
            image: overlay_handle,
            color: Color::srgba(1.0, 1.0, 1.0, CAMERA_OVERLAY_ALPHA),
            ..default()
        },
        CameraOverlay,
        Transform::from_xyz(0.0, 0.0, CAMERA_OVERLAY_Z),
        FruitCutEntity,
    ));

    let game_size = game_world_size(&window);
    commands.spawn((
        Sprite::from_color(
            CENTER_LINE_COLOR,
            Vec2::new(CENTER_LINE_THICKNESS, game_size.y),
        ),
        CenterLine,
        Transform::from_xyz(0.0, 0.0, 2.0),
        Visibility::Hidden,
        FruitCutEntity,
    ));

    spawn_setup_ui(&mut commands, &ui_font, &settings);
}

pub fn cleanup(mut commands: Commands, entities: Query<Entity, With<FruitCutEntity>>) {
    for entity in entities.iter() {
        commands.entity(entity).despawn();
    }
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
    let overlay_size = game_world_size(&window);

    for mut sprite in overlay_sprites.iter_mut() {
        sprite.custom_size = Some(overlay_size);

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

pub fn handle_escape_to_menu(
    input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if input.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::MainMenu);
    }
}

pub fn handle_setup_input(
    input: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<FruitCutSettings>,
    mut player_count_text: Query<&mut Text, With<PlayerCountText>>,
    mut center_line: Query<&mut Visibility, With<CenterLine>>,
) {
    let mut changed = false;

    if input.just_pressed(KeyCode::ArrowUp) && settings.player_count < MAX_PLAYERS {
        settings.player_count += 1;
        changed = true;
    }

    if input.just_pressed(KeyCode::ArrowDown) && settings.player_count > MIN_PLAYERS {
        settings.player_count -= 1;
        changed = true;
    }

    if changed {
        if let Ok(mut text) = player_count_text.single_mut() {
            **text = format!("{}", settings.player_count);
        }

        if let Ok(mut visibility) = center_line.single_mut() {
            *visibility = if settings.player_count == 2 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}

pub fn handle_setup_phase(
    mut commands: Commands,
    mut phase: ResMut<FruitCutPhase>,
    settings: Res<FruitCutSettings>,
    input: Res<ButtonInput<KeyCode>>,
    setup_text: Query<Entity, With<SetupText>>,
    mut center_line: Query<&mut Visibility, With<CenterLine>>,
) {
    if input.just_pressed(KeyCode::Enter) {
        *phase = FruitCutPhase::Playing;

        for entity in setup_text.iter() {
            commands.entity(entity).despawn();
        }

        if settings.player_count == 2
            && let Ok(mut visibility) = center_line.single_mut()
        {
            *visibility = Visibility::Visible;
        }
    }
}
