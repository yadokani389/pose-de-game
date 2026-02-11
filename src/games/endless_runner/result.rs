use bevy::prelude::*;

use crate::{AppState, assets::UiFont};

use super::game::{
    EndlessRunnerPhase, GameSettings, GameSpeed, NoseMarker, Obstacle, ObstacleSpawner, Player,
    PlayerGameOverText, PlayerTargets, Scoreboard, spawn_hud, spawn_nose_markers,
};
use super::setup::spawn_setup_ui;
use super::ui::{DistanceText, Player1DistanceText, Player2DistanceText};

const RESULT_HEADER_SIZE: f32 = 52.0;
const RESULT_TITLE_SIZE: f32 = 44.0;
const RESULT_DETAIL_SIZE: f32 = 28.0;
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

#[derive(Component)]
pub struct ResultRoot;

#[derive(Component)]
pub struct RetryButton;

#[derive(Component)]
pub struct MenuButton;

pub fn spawn_result_ui(
    mut commands: Commands,
    ui_font: Res<UiFont>,
    scoreboard: Res<Scoreboard>,
    settings: Res<GameSettings>,
    existing: Query<Entity, With<ResultRoot>>,
) {
    if !existing.is_empty() {
        return;
    }

    let distance1 = scoreboard.player1_distance as u32;
    let distance2 = scoreboard.player2_distance as u32;

    commands
        .spawn((
            ResultRoot,
            DespawnOnExit(AppState::EndlessRunner),
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
                Text::new("GAME OVER"),
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

            if settings.num_players == 1 {
                parent.spawn((
                    Text::new(format!("{} m", distance1)),
                    TextFont {
                        font: ui_font.0.clone(),
                        font_size: RESULT_TITLE_SIZE,
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 0.4, 0.7)),
                    Node {
                        margin: UiRect::bottom(Val::Px(12.0)),
                        ..default()
                    },
                ));

                parent.spawn((
                    Text::new("Distance Traveled"),
                    TextFont {
                        font: ui_font.0.clone(),
                        font_size: RESULT_DETAIL_SIZE,
                        ..default()
                    },
                    TextColor(RESULT_DETAIL_COLOR),
                    Node {
                        margin: UiRect::bottom(Val::Px(28.0)),
                        ..default()
                    },
                ));
            } else {
                let (winner_text, winner_color) = if distance1 > distance2 {
                    ("Player 1 Wins!", Color::srgb(1.0, 0.4, 0.7))
                } else if distance2 > distance1 {
                    ("Player 2 Wins!", Color::srgb(1.0, 0.6, 0.2))
                } else {
                    ("Draw!", Color::srgb(0.8, 0.8, 0.8))
                };

                parent.spawn((
                    Text::new(winner_text),
                    TextFont {
                        font: ui_font.0.clone(),
                        font_size: RESULT_TITLE_SIZE,
                        ..default()
                    },
                    TextColor(winner_color),
                    Node {
                        margin: UiRect::bottom(Val::Px(18.0)),
                        ..default()
                    },
                ));

                parent.spawn((
                    Text::new(format!("P1: {} m  P2: {} m", distance1, distance2)),
                    TextFont {
                        font: ui_font.0.clone(),
                        font_size: RESULT_DETAIL_SIZE,
                        ..default()
                    },
                    TextColor(RESULT_DETAIL_COLOR),
                    Node {
                        margin: UiRect::bottom(Val::Px(28.0)),
                        ..default()
                    },
                ));
            }

            parent
                .spawn((
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

pub fn handle_result_input(
    input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut phase: ResMut<EndlessRunnerPhase>,
    mut scoreboard: ResMut<Scoreboard>,
    mut game_speed: ResMut<GameSpeed>,
    mut spawner: ResMut<ObstacleSpawner>,
    settings: Res<GameSettings>,
    ui_font: Res<UiFont>,
    result_ui: Query<Entity, With<ResultRoot>>,
    players: Query<Entity, With<Player>>,
    obstacles: Query<Entity, With<Obstacle>>,
    game_over_texts: Query<Entity, With<PlayerGameOverText>>,
    nose_markers: Query<Entity, With<NoseMarker>>,
    distance_texts: Query<
        Entity,
        (
            With<DistanceText>,
            Or<(With<Player1DistanceText>, With<Player2DistanceText>)>,
        ),
    >,
) {
    if input.just_pressed(KeyCode::Space) {
        reset_game(
            &mut commands,
            &mut phase,
            &mut scoreboard,
            &mut game_speed,
            &mut spawner,
            &settings,
            &ui_font,
            &result_ui,
            &players,
            &obstacles,
            &game_over_texts,
            &nose_markers,
            &distance_texts,
        );
    }
}

pub fn button_system(
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
    mut commands: Commands,
    mut phase: ResMut<EndlessRunnerPhase>,
    mut scoreboard: ResMut<Scoreboard>,
    mut game_speed: ResMut<GameSpeed>,
    mut spawner: ResMut<ObstacleSpawner>,
    settings: Res<GameSettings>,
    ui_font: Res<UiFont>,
    result_ui: Query<Entity, With<ResultRoot>>,
    players: Query<Entity, With<Player>>,
    obstacles: Query<Entity, With<Obstacle>>,
    game_over_texts: Query<Entity, With<PlayerGameOverText>>,
    nose_markers: Query<Entity, With<NoseMarker>>,
    distance_texts: Query<
        Entity,
        (
            With<DistanceText>,
            Or<(With<Player1DistanceText>, With<Player2DistanceText>)>,
        ),
    >,
) {
    for (interaction, mut color, mut border_color, retry, menu) in &mut query {
        match *interaction {
            Interaction::Pressed => {
                *color = RESULT_BUTTON_PRESSED.into();
                border_color.set_all(Color::srgb(0.9, 0.9, 0.9));
                if retry.is_some() {
                    reset_game(
                        &mut commands,
                        &mut phase,
                        &mut scoreboard,
                        &mut game_speed,
                        &mut spawner,
                        &settings,
                        &ui_font,
                        &result_ui,
                        &players,
                        &obstacles,
                        &game_over_texts,
                        &nose_markers,
                        &distance_texts,
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

#[allow(clippy::too_many_arguments)]
fn reset_game(
    commands: &mut Commands,
    phase: &mut ResMut<EndlessRunnerPhase>,
    scoreboard: &mut ResMut<Scoreboard>,
    game_speed: &mut ResMut<GameSpeed>,
    spawner: &mut ResMut<ObstacleSpawner>,
    settings: &GameSettings,
    ui_font: &UiFont,
    result_ui: &Query<Entity, With<ResultRoot>>,
    players: &Query<Entity, With<Player>>,
    obstacles: &Query<Entity, With<Obstacle>>,
    game_over_texts: &Query<Entity, With<PlayerGameOverText>>,
    nose_markers: &Query<Entity, With<NoseMarker>>,
    distance_texts: &Query<
        Entity,
        (
            With<DistanceText>,
            Or<(With<Player1DistanceText>, With<Player2DistanceText>)>,
        ),
    >,
) {
    for entity in result_ui.iter() {
        commands.entity(entity).despawn();
    }
    for entity in players.iter() {
        commands.entity(entity).despawn();
    }
    for entity in obstacles.iter() {
        commands.entity(entity).despawn();
    }
    for entity in game_over_texts.iter() {
        commands.entity(entity).despawn();
    }
    for entity in nose_markers.iter() {
        commands.entity(entity).despawn();
    }
    for entity in distance_texts.iter() {
        commands.entity(entity).despawn();
    }

    **phase = EndlessRunnerPhase::Setup;
    **scoreboard = Scoreboard::default();
    **game_speed = GameSpeed::default();
    **spawner = ObstacleSpawner::default();
    commands.insert_resource(PlayerTargets::default());

    spawn_setup_ui(commands, ui_font, settings);

    spawn_nose_markers(commands, settings);

    spawn_hud(commands, ui_font, settings);
}
