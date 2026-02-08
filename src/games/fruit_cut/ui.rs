use bevy::prelude::*;

use crate::assets::UiFont;

use super::game::{
    ComboState, FruitCutEntity, GAME_DURATION, GameTimer, HandPreference, HandSelection, Scoreboard,
};

#[derive(Component)]
pub struct ScoreText;

#[derive(Component)]
pub struct ComboText;

#[derive(Component)]
pub struct TimerText;

#[derive(Component)]
pub struct HandModeText;

pub fn update_hud(
    mut commands: Commands,
    scoreboard: Res<Scoreboard>,
    combo: Res<ComboState>,
    game_timer: Res<GameTimer>,
    hand_selection: Res<HandSelection>,
    ui_font: Res<UiFont>,
    score_query: Query<Entity, With<ScoreText>>,
    mut score_text: Query<
        &mut Text,
        (
            With<ScoreText>,
            Without<ComboText>,
            Without<TimerText>,
            Without<HandModeText>,
        ),
    >,
    mut combo_text: Query<
        &mut Text,
        (
            With<ComboText>,
            Without<ScoreText>,
            Without<TimerText>,
            Without<HandModeText>,
        ),
    >,
    mut timer_text: Query<
        &mut Text,
        (
            With<TimerText>,
            Without<ScoreText>,
            Without<ComboText>,
            Without<HandModeText>,
        ),
    >,
    mut hand_mode_text: Query<
        &mut Text,
        (
            With<HandModeText>,
            Without<ScoreText>,
            Without<ComboText>,
            Without<TimerText>,
        ),
    >,
) {
    if score_query.is_empty() {
        commands
            .spawn((
                FruitCutEntity,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    position_type: PositionType::Absolute,
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent
                    .spawn((Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(100.0),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(20.0)),
                        ..default()
                    },))
                    .with_children(|top_bar| {
                        top_bar.spawn((Node {
                            width: Val::Px(150.0),
                            ..default()
                        },));

                        top_bar
                            .spawn((Node {
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                ..default()
                            },))
                            .with_children(|score_container| {
                                score_container.spawn((
                                    ScoreText,
                                    Text::new(format!("Score: {}", scoreboard.score)),
                                    TextFont {
                                        font: ui_font.0.clone(),
                                        font_size: 48.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(1.0, 1.0, 1.0)),
                                ));

                                score_container.spawn((
                                    ComboText,
                                    Text::new(""),
                                    TextFont {
                                        font: ui_font.0.clone(),
                                        font_size: 32.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(1.0, 0.8, 0.2)),
                                ));
                            });
                        top_bar
                            .spawn((Node {
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::End,
                                ..default()
                            },))
                            .with_children(|right_container| {
                                right_container.spawn((
                                    TimerText,
                                    Text::new(format!("Time: {:.0}s", GAME_DURATION)),
                                    TextFont {
                                        font: ui_font.0.clone(),
                                        font_size: 32.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                                ));

                                right_container.spawn((
                                    HandModeText,
                                    Text::new("Hand: Both"),
                                    TextFont {
                                        font: ui_font.0.clone(),
                                        font_size: 20.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.7, 0.7, 0.7)),
                                    Node {
                                        margin: UiRect::top(Val::Px(5.0)),
                                        ..default()
                                    },
                                ));
                            });
                    });
            });
    }

    if scoreboard.is_changed() {
        if let Ok(mut text) = score_text.single_mut() {
            *text = Text::new(format!("Score: {}", scoreboard.score));
        }
    }

    if combo.is_changed() {
        if let Ok(mut text) = combo_text.single_mut() {
            if combo.current_combo > 0 {
                *text = Text::new(format!("COMBO x{}", combo.current_combo));
            } else {
                *text = Text::new("");
            }
        }
    }

    if game_timer.is_changed() {
        if let Ok(mut text) = timer_text.single_mut() {
            let remaining = GAME_DURATION - game_timer.elapsed;
            *text = Text::new(format!("Time: {:.0}s", remaining.max(0.0)));
        }
    }

    if hand_selection.is_changed() {
        if let Ok(mut text) = hand_mode_text.single_mut() {
            let mode_str = match hand_selection.preference {
                HandPreference::Left => "Left",
                HandPreference::Right => "Right",
                HandPreference::Both => "Both",
            };
            *text = Text::new(format!("Hand: {}", mode_str));
        }
    }
}
