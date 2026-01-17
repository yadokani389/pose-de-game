use bevy::prelude::*;

use crate::{AppState, assets::UiFont};

use super::game::{GameOverReason, GameResult, Scoreboard};
use super::settings::{FlagRaisePhase, FlagRaiseSettings};
use super::setup;
use super::ui::GameUi;

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.35, 0.35);

#[derive(Component)]
pub(super) struct ResultRoot;

#[derive(Component)]
pub(super) struct RetryButton;

#[derive(Component)]
pub(super) struct MenuButton;

pub(super) fn spawn_result_ui(
    mut commands: Commands,
    ui_font: Res<UiFont>,
    settings: Res<FlagRaiseSettings>,
    scoreboard: Res<Scoreboard>,
    result: Option<Res<GameResult>>,
    existing: Query<Entity, With<ResultRoot>>,
    game_ui: Query<Entity, With<GameUi>>,
    phase: Res<FlagRaisePhase>,
) {
    if !matches!(*phase, FlagRaisePhase::Result) {
        return;
    }
    if !existing.is_empty() {
        return;
    }
    let Some(result) = result else {
        return;
    };

    for entity in &game_ui {
        commands.entity(entity).despawn();
    }

    let (header, header_color, title, title_color, detail) = match result.reason {
        GameOverReason::TimeUp => (
            "FINISH!".to_string(),
            Color::srgb(0.8, 0.9, 1.0),
            "".to_string(),
            Color::srgb(0.9, 0.9, 0.95),
            "".to_string(),
        ),
        GameOverReason::Failed => {
            let detail = if let Some(slot) = result.failed_slot {
                format!("プレイヤー{}が失敗しました", slot + 1)
            } else {
                "失敗しました".to_string()
            };
            (
                "GAME OVER".to_string(),
                Color::srgb(0.95, 0.95, 0.98),
                "FAILED".to_string(),
                Color::srgb(1.0, 0.6, 0.6),
                detail,
            )
        }
    };

    let mut ranked_players: Vec<(usize, u32)> = (0..settings.player_count)
        .map(|index| (index, scoreboard.scores[index]))
        .collect();

    ranked_players.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    commands
        .spawn((
            ResultRoot,
            DespawnOnExit(AppState::FlagRaise),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.05, 0.85)),
        ))
        .with_children(|parent| {
            parent.spawn((
                ResultRoot,
                Text::new(header),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: 48.0,
                    ..default()
                },
                TextColor(header_color),
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
                    font_size: 38.0,
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
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.85)),
                Node {
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                },
            ));

            parent
                .spawn((
                    ResultRoot,
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        margin: UiRect::bottom(Val::Px(30.0)),
                        ..default()
                    },
                ))
                .with_children(|score_parent| {
                    for (rank, (player_index, score)) in ranked_players.iter().enumerate() {
                        let rank_text = match rank {
                            0 => "★1st",
                            1 => "2nd",
                            2 => "3rd",
                            _ => &format!("{}th", rank + 1)[..],
                        };

                        let score_text = format!("{} P{}: {}", rank_text, player_index + 1, score);

                        let text_color = if rank == 0 {
                            Color::srgb(1.0, 0.84, 0.0) // Gold
                        } else {
                            Color::srgb(0.92, 0.92, 0.96) // Regular
                        };

                        let font_size = if rank == 0 { 30.0 } else { 26.0 };

                        let margin = if rank > 0 {
                            UiRect::left(Val::Px(20.0))
                        } else {
                            UiRect::default()
                        };

                        score_parent.spawn((
                            ResultRoot,
                            Text::new(score_text),
                            TextFont {
                                font: ui_font.0.clone(),
                                font_size,
                                ..default()
                            },
                            TextColor(text_color),
                            Node {
                                margin,
                                ..default()
                            },
                        ));
                    }
                });

            parent
                .spawn((
                    ResultRoot,
                    Button,
                    RetryButton,
                    Node {
                        width: Val::Px(320.0),
                        height: Val::Px(72.0),
                        border: UiRect::all(Val::Px(3.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BorderColor::all(Color::BLACK),
                    BorderRadius::MAX,
                    BackgroundColor(NORMAL_BUTTON),
                ))
                .with_children(|button| {
                    button.spawn((
                        ResultRoot,
                        Text::new("もう一度"),
                        TextFont {
                            font: ui_font.0.clone(),
                            font_size: 28.0,
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
                        width: Val::Px(320.0),
                        height: Val::Px(72.0),
                        border: UiRect::all(Val::Px(3.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        margin: UiRect::top(Val::Px(16.0)),
                        ..default()
                    },
                    BorderColor::all(Color::BLACK),
                    BorderRadius::MAX,
                    BackgroundColor(NORMAL_BUTTON),
                ))
                .with_children(|button| {
                    button.spawn((
                        ResultRoot,
                        Text::new("メニューへ戻る"),
                        TextFont {
                            font: ui_font.0.clone(),
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    ));
                });
        });
}

pub(super) fn result_input(
    input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    ui_font: Res<UiFont>,
    settings: Res<FlagRaiseSettings>,
    mut phase: ResMut<FlagRaisePhase>,
    result_ui: Query<Entity, With<ResultRoot>>,
) {
    if input.just_pressed(KeyCode::Space) {
        for entity in &result_ui {
            commands.entity(entity).despawn();
        }
        setup::return_to_setup(&mut commands, &ui_font, &settings, &mut phase);
    }
}

pub(super) fn button_system(
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
    mut phase: ResMut<FlagRaisePhase>,
    mut commands: Commands,
    ui_font: Res<UiFont>,
    settings: Res<FlagRaiseSettings>,
    result_ui: Query<Entity, With<ResultRoot>>,
) {
    for (interaction, mut color, mut border_color, retry, menu) in &mut query {
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
                border_color.set_all(Color::srgb(0.9, 0.9, 0.9));
                if retry.is_some() {
                    for entity in &result_ui {
                        commands.entity(entity).despawn();
                    }
                    setup::return_to_setup(&mut commands, &ui_font, &settings, &mut phase);
                } else if menu.is_some() {
                    next_state.set(AppState::MainMenu);
                    *phase = FlagRaisePhase::Setup;
                }
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
                border_color.set_all(Color::WHITE);
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
                border_color.set_all(Color::BLACK);
            }
        }
    }
}
