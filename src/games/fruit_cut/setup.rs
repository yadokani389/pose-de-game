use bevy::asset::RenderAssetUsages;
use bevy::camera::ScalingMode;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::{
    AppState,
    assets::UiFont,
    pose::{LatestFrameRes, PeopleDataRes},
};

use super::{
    CAMERA_USED_MARGIN,
    game::{
        ComboState, FruitCutEntity, FruitCutPhase, FruitSpawner, GameTimer, HandSelection,
        Scoreboard,
    },
    game_world_size,
    hand_tracker::HandTrackers,
};
const CAMERA_OVERLAY_ALPHA: f32 = 0.15;
const CAMERA_OVERLAY_Z: f32 = 0.5;

const SETUP_ACTIVATION_THRESHOLD: f32 = 300.0;

#[derive(Component)]
pub struct SetupText;

#[derive(Component)]
pub struct CameraOverlay;

#[derive(Resource)]
pub struct CameraOverlayImageHandle(pub Handle<Image>);

pub fn is_setup(phase: Res<FruitCutPhase>) -> bool {
    *phase == FruitCutPhase::Setup
}

pub fn setup(
    mut commands: Commands,
    mut phase: ResMut<FruitCutPhase>,
    mut scoreboard: ResMut<Scoreboard>,
    mut combo: ResMut<ComboState>,
    mut game_timer: ResMut<GameTimer>,
    mut spawner: ResMut<FruitSpawner>,
    mut hand_trackers: ResMut<HandTrackers>,
    mut hand_selection: ResMut<HandSelection>,
    mut latest_frame: ResMut<LatestFrameRes>,
    ui_font: Res<UiFont>,
    mut images: ResMut<Assets<Image>>,
    window: Single<&Window>,
) {
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

    *phase = FruitCutPhase::Setup;
    *scoreboard = Scoreboard::default();
    *combo = ComboState::default();
    *game_timer = GameTimer::default();
    *spawner = FruitSpawner::default();
    *hand_trackers = HandTrackers::default();
    *hand_selection = HandSelection::default();
    latest_frame.frame = None;

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
                Text::new("Wave your hands to start!"),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                Node {
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::new("Esc: Menu / Left/Right: Switch Hand"),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ));
        });
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
    mut overlay_sprite: Query<&mut Sprite, With<CameraOverlay>>,
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

    if let Ok(mut sprite) = overlay_sprite.single_mut() {
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

pub fn handle_setup_phase(
    mut phase: ResMut<FruitCutPhase>,
    hand_trackers: Res<HandTrackers>,
    people: Res<PeopleDataRes>,
    mut commands: Commands,
    setup_text: Query<Entity, With<SetupText>>,
) {
    if people.is_empty() {
        return;
    }

    let left_fast = hand_trackers
        .left_velocity()
        .map(|v| v.length() >= SETUP_ACTIVATION_THRESHOLD)
        .unwrap_or(false);

    let right_fast = hand_trackers
        .right_velocity()
        .map(|v| v.length() >= SETUP_ACTIVATION_THRESHOLD)
        .unwrap_or(false);

    if left_fast || right_fast {
        *phase = FruitCutPhase::Playing;

        for entity in setup_text.iter() {
            commands.entity(entity).despawn();
        }
    }
}
