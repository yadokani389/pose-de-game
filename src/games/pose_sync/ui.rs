use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::PrimaryWindow;

use crate::{AppState, assets::UiFont};

use super::{
    game::{CommandState, GAME_LIMIT_SECS, GameTimer, RoundStage, SEQUENCE_LEN, Scoreboard},
    settings::{MAX_PLAYERS, PoseSyncSettings},
};

const SCORE_FONT_SIZE: f32 = 36.0;
const TURN_TIMER_FONT_SIZE: f32 = 40.0;
const GAME_TIMER_FONT_SIZE: f32 = 26.0;
const SCORE_MARGIN_TOP: f32 = 152.0;
const GAME_TIMER_MARGIN_TOP: f32 = 24.0;
const GAME_TIMER_MARGIN_RIGHT: f32 = 24.0;
const HINT_MARGIN_BOTTOM: f32 = 26.0;

#[derive(Component)]
pub(super) struct GameUi;

#[derive(Component)]
pub(super) struct TurnTimerText;

#[derive(Component)]
pub(super) struct GameTimerText;

#[derive(Component)]
pub(super) struct HintText;

#[derive(Component)]
pub(super) struct ScoreText {
    index: usize,
}

pub(super) fn spawn_game_ui(commands: &mut Commands, ui_font: &UiFont) {
    commands.spawn((
        GameUi,
        TurnTimerText,
        Text2d::new("0.0"),
        TextFont {
            font: ui_font.0.clone(),
            font_size: TURN_TIMER_FONT_SIZE,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
        TextColor(Color::srgb(0.92, 0.92, 0.96)),
        Anchor::CENTER,
        Transform::from_xyz(0.0, 0.0, 12.0),
        DespawnOnExit(AppState::PoseSync),
    ));

    commands.spawn((
        GameUi,
        GameTimerText,
        Text2d::new(format!("{:.1}", GAME_LIMIT_SECS)),
        TextFont {
            font: ui_font.0.clone(),
            font_size: GAME_TIMER_FONT_SIZE,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
        TextColor(Color::srgb(0.85, 0.85, 0.9)),
        Anchor::TOP_RIGHT,
        Transform::from_xyz(0.0, 0.0, 12.0),
        DespawnOnExit(AppState::PoseSync),
    ));

    commands.spawn((
        GameUi,
        HintText,
        Text2d::new("見る 1/3"),
        TextFont {
            font: ui_font.0.clone(),
            font_size: 22.0,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
        TextColor(Color::srgb(0.7, 0.72, 0.78)),
        Anchor::BOTTOM_CENTER,
        Transform::from_xyz(0.0, 0.0, 12.0),
        DespawnOnExit(AppState::PoseSync),
    ));

    for index in 0..MAX_PLAYERS {
        commands.spawn((
            GameUi,
            ScoreText { index },
            Text2d::new("0"),
            TextFont {
                font: ui_font.0.clone(),
                font_size: SCORE_FONT_SIZE,
                ..default()
            },
            TextLayout::new_with_justify(Justify::Center),
            TextColor(Color::srgb(0.92, 0.92, 0.96)),
            Anchor::TOP_CENTER,
            Transform::from_xyz(0.0, 0.0, 12.0),
            DespawnOnExit(AppState::PoseSync),
        ));
    }
}

pub(super) fn update_score_texts(
    settings: Res<PoseSyncSettings>,
    scoreboard: Res<Scoreboard>,
    mut query: Query<(&ScoreText, &mut Text2d, &mut Visibility)>,
) {
    for (slot, mut text, mut visibility) in &mut query {
        if slot.index < settings.player_count {
            *visibility = Visibility::Visible;
            *text = Text2d::new(format!(
                "P{} {}",
                slot.index + 1,
                scoreboard.scores[slot.index]
            ));
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

pub(super) fn update_turn_timer_text(
    command: Res<CommandState>,
    mut query: Query<&mut Text2d, With<TurnTimerText>>,
) {
    if let Ok(mut text) = query.single_mut() {
        *text = Text2d::new(format!("{:.1}", command.timer.remaining_secs().max(0.0)));
    }
}

pub(super) fn update_hint_text(
    mut command: ResMut<CommandState>,
    mut query: Query<&mut Text2d, With<HintText>>,
) {
    if !command.dirty {
        return;
    }

    let phase = match command.stage {
        RoundStage::Intro => "開始待ち",
        RoundStage::Show => "見る",
        RoundStage::Repeat => "再現",
    };
    let step = command.step_index + 1;

    if let Ok(mut text) = query.single_mut() {
        *text = Text2d::new(format!("{} {}/{}", phase, step, SEQUENCE_LEN));
    }

    command.dirty = false;
}

pub(super) fn update_game_timer_text(
    game_timer: Res<GameTimer>,
    mut query: Query<&mut Text2d, With<GameTimerText>>,
) {
    if let Ok(mut text) = query.single_mut() {
        *text = Text2d::new(format!("{:.1}", game_timer.timer.remaining_secs().max(0.0)));
    }
}

pub(super) fn update_text_positions(
    window: Query<&Window, With<PrimaryWindow>>,
    settings: Res<PoseSyncSettings>,
    mut turn_timer: Query<
        &mut Transform,
        (
            With<TurnTimerText>,
            Without<ScoreText>,
            Without<GameTimerText>,
            Without<HintText>,
        ),
    >,
    mut game_timer: Query<
        &mut Transform,
        (
            With<GameTimerText>,
            Without<ScoreText>,
            Without<TurnTimerText>,
            Without<HintText>,
        ),
    >,
    mut hint: Query<
        &mut Transform,
        (
            With<HintText>,
            Without<ScoreText>,
            Without<TurnTimerText>,
            Without<GameTimerText>,
        ),
    >,
    mut scores: Query<
        (&ScoreText, &mut Transform),
        (
            Without<TurnTimerText>,
            Without<GameTimerText>,
            Without<HintText>,
        ),
    >,
) {
    let Ok(window) = window.single() else {
        return;
    };
    let frame_size = Vec2::new(window.resolution.width(), window.resolution.height());
    let half_height = frame_size.y * 0.5;
    let half_width = frame_size.x * 0.5;

    if let Ok(mut transform) = turn_timer.single_mut() {
        transform.translation = Vec3::new(0.0, 0.0, 12.0);
    }
    if let Ok(mut transform) = game_timer.single_mut() {
        transform.translation = Vec3::new(
            half_width - GAME_TIMER_MARGIN_RIGHT,
            half_height - GAME_TIMER_MARGIN_TOP,
            12.0,
        );
    }
    if let Ok(mut transform) = hint.single_mut() {
        transform.translation = Vec3::new(0.0, -half_height + HINT_MARGIN_BOTTOM, 12.0);
    }

    let slot_width = frame_size.x / settings.player_count as f32;
    for (slot, mut transform) in &mut scores {
        let x = -half_width + slot_width * (slot.index as f32 + 0.5);
        transform.translation = Vec3::new(x, half_height - SCORE_MARGIN_TOP, 12.0);
    }
}
