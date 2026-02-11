use bevy::prelude::*;

use crate::{AppState, assets::UiFont};

use super::game::{
    DarkOverlay, EndlessRunnerPhase, GameSettings, LaneLayout, LaneLine, Player, PlayerId,
    lane_to_x_for_player, spawn_hud, spawn_lanes_and_overlays,
};
use super::ui;

const PLAYER_Z: f32 = 10.0;

const SETUP_TITLE_SIZE: f32 = 58.0;
const SETUP_LABEL_SIZE: f32 = 36.0;
const SETUP_VALUE_SIZE: f32 = 44.0;
const SETUP_HINT_SIZE: f32 = 28.0;

#[derive(Component)]
pub struct SetupText;

#[derive(Component)]
pub struct PlayerCountText;

pub fn is_setup(phase: Res<EndlessRunnerPhase>) -> bool {
    *phase == EndlessRunnerPhase::Setup
}

pub fn setup_ui_on_enter(
    mut commands: Commands,
    ui_font: Res<UiFont>,
    settings: Res<GameSettings>,
    existing: Query<Entity, With<SetupText>>,
    phase: Res<EndlessRunnerPhase>,
) {
    if *phase != EndlessRunnerPhase::Setup {
        return;
    }

    if !existing.is_empty() {
        return;
    }

    spawn_setup_ui(&mut commands, &ui_font, &settings);
}

pub fn setup_ui(commands: &mut Commands, ui_font: &Res<UiFont>, settings: &Res<GameSettings>) {
    spawn_setup_ui(commands, ui_font, settings);
}

pub fn spawn_setup_ui(commands: &mut Commands, ui_font: &UiFont, settings: &GameSettings) {
    commands
        .spawn((
            SetupText,
            DespawnOnExit(AppState::EndlessRunner),
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
                Text::new("エンドレスランナー"),
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
                Text::new(format!("{}", settings.num_players)),
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

pub fn handle_setup_input(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<GameSettings>,
    layout: Res<LaneLayout>,
    mut player_count_text: Query<&mut Text, With<PlayerCountText>>,
    lane_lines: Query<Entity, With<LaneLine>>,
    dark_overlays: Query<Entity, With<DarkOverlay>>,
) {
    let mut changed = false;

    if input.just_pressed(KeyCode::ArrowUp) && settings.num_players < 2 {
        settings.num_players += 1;
        changed = true;
    }

    if input.just_pressed(KeyCode::ArrowDown) && settings.num_players > 1 {
        settings.num_players -= 1;
        changed = true;
    }

    if changed {
        if let Ok(mut text) = player_count_text.single_mut() {
            *text = Text::new(format!("{}", settings.num_players));
        }

        for entity in lane_lines.iter() {
            commands.entity(entity).despawn();
        }
        for entity in dark_overlays.iter() {
            commands.entity(entity).despawn();
        }

        spawn_lanes_and_overlays(&mut commands, &layout, &settings);
    }
}

pub fn handle_setup_phase(
    mut commands: Commands,
    mut phase: ResMut<EndlessRunnerPhase>,
    input: Res<ButtonInput<KeyCode>>,
    setup_text: Query<Entity, With<SetupText>>,
    settings: Res<GameSettings>,
    lane_layout: Res<LaneLayout>,
    ui_font: Res<UiFont>,
    distance_texts: Query<Entity, With<ui::DistanceText>>,
) {
    if input.just_pressed(KeyCode::Enter) {
        *phase = EndlessRunnerPhase::Playing;

        for entity in setup_text.iter() {
            commands.entity(entity).despawn();
        }

        // Recreate HUD with the current player count (ensures P2 text exists when 2P is selected).
        for entity in distance_texts.iter() {
            commands.entity(entity).despawn();
        }
        spawn_hud(&mut commands, &ui_font, &settings);

        let player_size = lane_layout.player_size;
        let start_lane = 1;

        let player1_x = lane_to_x_for_player(
            start_lane,
            &lane_layout,
            PlayerId::Player1,
            settings.num_players,
        );
        commands.spawn((
            Sprite::from_color(
                Color::srgb(1.0, 0.4, 0.7),
                Vec2::new(player_size, player_size),
            ),
            Transform::from_xyz(player1_x, -200.0, PLAYER_Z),
            Player {
                id: PlayerId::Player1,
                current_lane: start_lane,
                target_lane: start_lane,
                is_alive: true,
            },
            DespawnOnExit(AppState::EndlessRunner),
        ));

        if settings.num_players == 2 {
            let player2_x = lane_to_x_for_player(
                start_lane,
                &lane_layout,
                PlayerId::Player2,
                settings.num_players,
            );
            commands.spawn((
                Sprite::from_color(
                    Color::srgb(1.0, 0.6, 0.2),
                    Vec2::new(player_size, player_size),
                ),
                Transform::from_xyz(player2_x, -200.0, PLAYER_Z),
                Player {
                    id: PlayerId::Player2,
                    current_lane: start_lane,
                    target_lane: start_lane,
                    is_alive: true,
                },
                DespawnOnExit(AppState::EndlessRunner),
            ));
        }
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
