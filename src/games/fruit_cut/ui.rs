use bevy::ecs::system::ParamSet;
use bevy::prelude::*;

use crate::assets::UiFont;

use super::game::{
    ComboState, FruitCutEntity, FruitCutSettings, GAME_DURATION, GameTimer, Scoreboard,
};

#[derive(Component)]
pub struct ScoreText;

#[derive(Component)]
pub struct LeftScoreText;

#[derive(Component)]
pub struct RightScoreText;

#[derive(Component)]
pub struct ComboText;

#[derive(Component)]
pub struct LeftComboText;

#[derive(Component)]
pub struct RightComboText;

#[derive(Component)]
pub struct TimerText;

pub fn update_hud(
    mut commands: Commands,
    scoreboard: Res<Scoreboard>,
    combo: Res<ComboState>,
    game_timer: Res<GameTimer>,
    settings: Res<FruitCutSettings>,
    ui_font: Res<UiFont>,
    score_query: Query<Entity, With<ScoreText>>,
    left_score_query: Query<Entity, With<LeftScoreText>>,
    mut text_queries: ParamSet<(
        Query<&mut Text, With<ScoreText>>,
        Query<&mut Text, With<ComboText>>,
        Query<&mut Text, With<LeftScoreText>>,
        Query<&mut Text, With<RightScoreText>>,
        Query<&mut Text, With<LeftComboText>>,
        Query<&mut Text, With<RightComboText>>,
        Query<&mut Text, With<TimerText>>,
    )>,
) {
    if settings.player_count == 1 {
        if score_query.is_empty() {
            spawn_single_player_hud(&mut commands, &ui_font, &scoreboard);
        }

        if scoreboard.is_changed() {
            if let Ok(mut text) = text_queries.p0().single_mut() {
                *text = Text::new(format!("Score: {}", scoreboard.left_score));
            }
        }

        if combo.is_changed() {
            if let Ok(mut text) = text_queries.p1().single_mut() {
                if combo.left.current_combo > 0 {
                    *text = Text::new(format!("COMBO x{}", combo.left.current_combo));
                } else {
                    *text = Text::new("");
                }
            }
        }
    } else {
        if left_score_query.is_empty() {
            spawn_two_player_hud(&mut commands, &ui_font, &scoreboard);
        }

        if scoreboard.is_changed() {
            if let Ok(mut text) = text_queries.p2().single_mut() {
                *text = Text::new(format!("P1: {}", scoreboard.left_score));
            }
            if let Ok(mut text) = text_queries.p3().single_mut() {
                *text = Text::new(format!("P2: {}", scoreboard.right_score));
            }
        }

        if combo.is_changed() {
            if let Ok(mut text) = text_queries.p4().single_mut() {
                if combo.left.current_combo > 0 {
                    *text = Text::new(format!("x{}", combo.left.current_combo));
                } else {
                    *text = Text::new("");
                }
            }
            if let Ok(mut text) = text_queries.p5().single_mut() {
                if combo.right.current_combo > 0 {
                    *text = Text::new(format!("x{}", combo.right.current_combo));
                } else {
                    *text = Text::new("");
                }
            }
        }
    }

    if game_timer.is_changed() {
        if let Ok(mut text) = text_queries.p6().single_mut() {
            let remaining = GAME_DURATION - game_timer.elapsed;
            *text = Text::new(format!("Time: {:.0}s", remaining.max(0.0)));
        }
    }
}

fn spawn_single_player_hud(commands: &mut Commands, ui_font: &UiFont, scoreboard: &Scoreboard) {
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
                                Text::new(format!("Score: {}", scoreboard.left_score)),
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
                    top_bar.spawn((
                        TimerText,
                        Text::new(format!("Time: {:.0}s", GAME_DURATION)),
                        TextFont {
                            font: ui_font.0.clone(),
                            font_size: 32.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    ));
                });
        });
}

fn spawn_two_player_hud(commands: &mut Commands, ui_font: &UiFont, scoreboard: &Scoreboard) {
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
                    top_bar
                        .spawn((Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Start,
                            ..default()
                        },))
                        .with_children(|left_container| {
                            left_container.spawn((
                                LeftScoreText,
                                Text::new(format!("P1: {}", scoreboard.left_score)),
                                TextFont {
                                    font: ui_font.0.clone(),
                                    font_size: 40.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.3, 0.6, 1.0)),
                            ));

                            left_container.spawn((
                                LeftComboText,
                                Text::new(""),
                                TextFont {
                                    font: ui_font.0.clone(),
                                    font_size: 28.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(1.0, 0.8, 0.2)),
                            ));
                        });

                    top_bar.spawn((
                        TimerText,
                        Text::new(format!("Time: {:.0}s", GAME_DURATION)),
                        TextFont {
                            font: ui_font.0.clone(),
                            font_size: 32.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    ));

                    top_bar
                        .spawn((Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::End,
                            ..default()
                        },))
                        .with_children(|right_container| {
                            right_container.spawn((
                                RightScoreText,
                                Text::new(format!("P2: {}", scoreboard.right_score)),
                                TextFont {
                                    font: ui_font.0.clone(),
                                    font_size: 40.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(1.0, 0.3, 0.4)),
                            ));

                            right_container.spawn((
                                RightComboText,
                                Text::new(""),
                                TextFont {
                                    font: ui_font.0.clone(),
                                    font_size: 28.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(1.0, 0.8, 0.2)),
                            ));
                        });
                });
        });
}
