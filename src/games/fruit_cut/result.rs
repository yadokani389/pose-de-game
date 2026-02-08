use bevy::prelude::*;

use crate::{AppState, assets::UiFont};

use super::game::{
    ComboState, FruitCutEntity, FruitCutPhase, FruitSpawner, GameResult, GameTimer, HandSelection,
    Scoreboard,
};
use super::hand_tracker::HandTrackers;

const RESULT_HEADER_SIZE: f32 = 52.0;
const RESULT_TITLE_SIZE: f32 = 44.0;
const RESULT_DETAIL_SIZE: f32 = 24.0;
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
    result: Option<Res<GameResult>>,
    existing: Query<Entity, With<ResultRoot>>,
) {
    if !existing.is_empty() {
        return;
    }

    let Some(result) = result else {
        return;
    };

    let accuracy = if result.total_sliced + result.total_missed > 0 {
        (result.total_sliced as f32 / (result.total_sliced + result.total_missed) as f32) * 100.0
    } else {
        0.0
    };

    commands
        .spawn((
            ResultRoot,
            FruitCutEntity,
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
                Text::new("GAME OVER!"),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: RESULT_HEADER_SIZE,
                    ..default()
                },
                TextColor(RESULT_HEADER_COLOR),
                Node {
                    margin: UiRect::bottom(Val::Px(24.0)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::new(format!("Score: {}", result.final_score)),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: RESULT_TITLE_SIZE,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.9, 0.3)),
                Node {
                    margin: UiRect::bottom(Val::Px(32.0)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::new(format!("Fruits Sliced: {}", result.total_sliced)),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: RESULT_DETAIL_SIZE,
                    ..default()
                },
                TextColor(RESULT_DETAIL_COLOR),
                Node {
                    margin: UiRect::bottom(Val::Px(8.0)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::new(format!("Max Combo: {}", result.max_combo)),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: RESULT_DETAIL_SIZE,
                    ..default()
                },
                TextColor(RESULT_DETAIL_COLOR),
                Node {
                    margin: UiRect::bottom(Val::Px(8.0)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::new(format!("Accuracy: {:.1}%", accuracy)),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: RESULT_DETAIL_SIZE,
                    ..default()
                },
                TextColor(RESULT_DETAIL_COLOR),
                Node {
                    margin: UiRect::bottom(Val::Px(8.0)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::new(format!("Bombs Hit: {}", result.bombs_hit)),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: RESULT_DETAIL_SIZE,
                    ..default()
                },
                TextColor(if result.bombs_hit > 0 {
                    Color::srgb(1.0, 0.4, 0.4)
                } else {
                    RESULT_DETAIL_COLOR
                }),
                Node {
                    margin: UiRect::bottom(Val::Px(32.0)),
                    ..default()
                },
            ));

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
    mut phase: ResMut<FruitCutPhase>,
    mut scoreboard: ResMut<Scoreboard>,
    mut combo: ResMut<ComboState>,
    mut game_timer: ResMut<GameTimer>,
    mut spawner: ResMut<FruitSpawner>,
    mut hand_trackers: ResMut<HandTrackers>,
    mut hand_selection: ResMut<HandSelection>,
    result_ui: Query<Entity, With<ResultRoot>>,
) {
    if input.just_pressed(KeyCode::Space) {
        for entity in &result_ui {
            commands.entity(entity).despawn();
        }
        reset_game(
            &mut commands,
            &mut phase,
            &mut scoreboard,
            &mut combo,
            &mut game_timer,
            &mut spawner,
            &mut hand_trackers,
            &mut hand_selection,
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
    mut phase: ResMut<FruitCutPhase>,
    mut scoreboard: ResMut<Scoreboard>,
    mut combo: ResMut<ComboState>,
    mut game_timer: ResMut<GameTimer>,
    mut spawner: ResMut<FruitSpawner>,
    mut hand_trackers: ResMut<HandTrackers>,
    mut hand_selection: ResMut<HandSelection>,
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
                    reset_game(
                        &mut commands,
                        &mut phase,
                        &mut scoreboard,
                        &mut combo,
                        &mut game_timer,
                        &mut spawner,
                        &mut hand_trackers,
                        &mut hand_selection,
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

fn reset_game(
    commands: &mut Commands,
    phase: &mut ResMut<FruitCutPhase>,
    scoreboard: &mut ResMut<Scoreboard>,
    combo: &mut ResMut<ComboState>,
    game_timer: &mut ResMut<GameTimer>,
    spawner: &mut ResMut<FruitSpawner>,
    hand_trackers: &mut ResMut<HandTrackers>,
    hand_selection: &mut ResMut<HandSelection>,
) {
    **phase = FruitCutPhase::Setup;
    **scoreboard = Scoreboard::default();
    **combo = ComboState::default();
    **game_timer = GameTimer::default();
    **spawner = FruitSpawner::default();
    **hand_trackers = HandTrackers::default();
    **hand_selection = HandSelection::default();
    commands.remove_resource::<GameResult>();
}
