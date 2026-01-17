use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::text::TextLayoutInfo;
use bevy::window::PrimaryWindow;

use crate::{AppState, assets::UiFont};

use super::game::{CommandSpec, CommandState, FlagColor, GAME_LIMIT_SECS, GameTimer, Scoreboard};
use super::settings::FlagRaiseSettings;

const INSTRUCTION_FONT_SIZE: f32 = 96.0;
const SCORE_FONT_SIZE: f32 = 36.0;
const TIMER_FONT_SIZE: f32 = 40.0;
const GAME_TIMER_FONT_SIZE: f32 = 26.0;
const INSTRUCTION_MARGIN_TOP: f32 = 48.0;
const SCORE_MARGIN_TOP: f32 = 140.0;
const GAME_TIMER_MARGIN_TOP: f32 = 24.0;
const GAME_TIMER_MARGIN_RIGHT: f32 = 24.0;
const INSTRUCTION_NEUTRAL_COLOR: Color = Color::srgb(0.92, 0.92, 0.96);

#[derive(Component)]
pub(super) struct InstructionText;

#[derive(Component)]
pub(super) struct TimerText;

#[derive(Component)]
pub(super) struct GameUi;

#[derive(Component)]
pub(super) struct GameTimerText;

#[derive(Component)]
pub(super) struct ScoreText {
    index: usize,
}

#[derive(Component, Clone, Copy)]
pub(super) struct InstructionChunk {
    role: InstructionChunkRole,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InstructionChunkRole {
    PrimaryColor,
    PrimaryAction,
    Separator,
    SecondaryColor,
    SecondaryAction,
}

pub(super) fn spawn_game_ui(
    commands: &mut Commands,
    ui_font: &UiFont,
    command: &CommandSpec,
    mismatch_color: bool,
) {
    let chunks = instruction_chunk_contents(*command, mismatch_color);
    commands.spawn((
        GameUi,
        InstructionText,
        Text2d::new(""),
        TextFont {
            font: ui_font.0.clone(),
            font_size: INSTRUCTION_FONT_SIZE,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
        TextColor(INSTRUCTION_NEUTRAL_COLOR),
        Anchor::TOP_CENTER,
        Transform::from_xyz(0.0, 0.0, 10.0),
        DespawnOnExit(AppState::FlagRaise),
    ));

    spawn_instruction_chunk(
        commands,
        ui_font,
        chunks.primary_color,
        InstructionChunkRole::PrimaryColor,
    );
    spawn_instruction_chunk(
        commands,
        ui_font,
        chunks.primary_action,
        InstructionChunkRole::PrimaryAction,
    );
    spawn_instruction_chunk(
        commands,
        ui_font,
        chunks.separator,
        InstructionChunkRole::Separator,
    );
    spawn_instruction_chunk(
        commands,
        ui_font,
        chunks.secondary_color,
        InstructionChunkRole::SecondaryColor,
    );
    spawn_instruction_chunk(
        commands,
        ui_font,
        chunks.secondary_action,
        InstructionChunkRole::SecondaryAction,
    );

    commands.spawn((
        GameUi,
        TimerText,
        Text2d::new("0.0"),
        TextFont {
            font: ui_font.0.clone(),
            font_size: TIMER_FONT_SIZE,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
        TextColor(Color::srgb(0.92, 0.92, 0.96)),
        Anchor::CENTER,
        Transform::from_xyz(0.0, 0.0, 9.5),
        DespawnOnExit(AppState::FlagRaise),
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
        Transform::from_xyz(0.0, 0.0, 10.0),
        DespawnOnExit(AppState::FlagRaise),
    ));

    for index in 0..super::settings::MAX_PLAYERS {
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
            Transform::from_xyz(0.0, 0.0, 9.0),
            DespawnOnExit(AppState::FlagRaise),
        ));
    }
}

pub(super) fn update_instruction_text(
    mut command: ResMut<CommandState>,
    mut chunks: Query<(&InstructionChunk, &mut Text2d, &mut TextColor)>,
) {
    if !command.dirty {
        return;
    }
    let contents = instruction_chunk_contents(command.current, command.mismatch_color);
    for (chunk, mut text, mut color) in &mut chunks {
        let content = match chunk.role {
            InstructionChunkRole::PrimaryColor => &contents.primary_color,
            InstructionChunkRole::PrimaryAction => &contents.primary_action,
            InstructionChunkRole::Separator => &contents.separator,
            InstructionChunkRole::SecondaryColor => &contents.secondary_color,
            InstructionChunkRole::SecondaryAction => &contents.secondary_action,
        };
        *text = Text2d::new(content.text.clone());
        *color = TextColor(content.color);
    }
    command.dirty = false;
}

pub(super) fn update_score_texts(
    settings: Res<FlagRaiseSettings>,
    scoreboard: Res<Scoreboard>,
    mut query: Query<(&ScoreText, &mut Text2d, &mut Visibility)>,
) {
    for (slot, mut text, mut visibility) in &mut query {
        if slot.index < settings.player_count {
            *visibility = Visibility::Visible;
            *text = Text2d::new(format!("{}", scoreboard.scores[slot.index]));
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

pub(super) fn update_timer_text(
    command: Res<CommandState>,
    mut query: Query<&mut Text2d, With<TimerText>>,
) {
    let Ok(mut text) = query.single_mut() else {
        return;
    };
    let remaining = command.timer.remaining_secs().max(0.0);
    *text = Text2d::new(format!("{:.1}", remaining));
}

pub(super) fn update_game_timer_text(
    game_timer: Res<GameTimer>,
    mut query: Query<&mut Text2d, With<GameTimerText>>,
) {
    let Ok(mut text) = query.single_mut() else {
        return;
    };
    let remaining = game_timer.timer.remaining_secs().max(0.0);
    *text = Text2d::new(format!("{:.1}", remaining));
}

pub(super) fn update_text_positions(
    window: Query<&Window, With<PrimaryWindow>>,
    settings: Res<FlagRaiseSettings>,
    chunks: Query<(Entity, &InstructionChunk, &TextLayoutInfo)>,
    mut chunk_transforms: Query<&mut Transform, With<InstructionChunk>>,
    mut instruction: Query<
        &mut Transform,
        (
            With<InstructionText>,
            Without<ScoreText>,
            Without<GameTimerText>,
            Without<InstructionChunk>,
        ),
    >,
    mut timer: Query<
        &mut Transform,
        (
            With<TimerText>,
            Without<InstructionText>,
            Without<ScoreText>,
            Without<GameTimerText>,
            Without<InstructionChunk>,
        ),
    >,
    mut game_timer: Query<
        &mut Transform,
        (
            With<GameTimerText>,
            Without<InstructionText>,
            Without<ScoreText>,
            Without<InstructionChunk>,
        ),
    >,
    mut scores: Query<
        (&ScoreText, &mut Transform),
        (Without<InstructionText>, Without<InstructionChunk>),
    >,
) {
    let Ok(window) = window.single() else {
        return;
    };
    let frame_size = Vec2::new(window.resolution.width(), window.resolution.height());
    let half_height = frame_size.y * 0.5;
    let half_width = frame_size.x * 0.5;

    let Ok(mut instruction) = instruction.single_mut() else {
        return;
    };
    instruction.translation = Vec3::new(0.0, half_height - INSTRUCTION_MARGIN_TOP, 10.0);

    update_instruction_chunk_positions(
        &chunks,
        &mut chunk_transforms,
        half_height - INSTRUCTION_MARGIN_TOP,
    );

    if let Ok(mut timer) = timer.single_mut() {
        timer.translation = Vec3::new(0.0, 0.0, 9.5);
    }

    if let Ok(mut game_timer) = game_timer.single_mut() {
        game_timer.translation = Vec3::new(
            half_width - GAME_TIMER_MARGIN_RIGHT,
            half_height - GAME_TIMER_MARGIN_TOP,
            10.0,
        );
    }

    let slot_width = frame_size.x / settings.player_count as f32;
    for (slot, mut transform) in &mut scores {
        let x = -half_width + slot_width * (slot.index as f32 + 0.5);
        transform.translation = Vec3::new(x, half_height - SCORE_MARGIN_TOP, 9.0);
    }
}

struct InstructionChunkContent {
    text: String,
    color: Color,
}

struct InstructionChunkContents {
    primary_color: InstructionChunkContent,
    primary_action: InstructionChunkContent,
    separator: InstructionChunkContent,
    secondary_color: InstructionChunkContent,
    secondary_action: InstructionChunkContent,
}

fn instruction_chunk_contents(
    command: CommandSpec,
    mismatch_color: bool,
) -> InstructionChunkContents {
    let display = command.display();
    let primary_label = display.primary.color.label();
    let primary_action = display.primary.action_text;
    let primary_color = label_color(display.primary.color, mismatch_color);

    let (separator, secondary_label, secondary_action, secondary_color) =
        if let Some(secondary) = display.secondary {
            (
                " / ",
                secondary.color.label(),
                secondary.action_text,
                label_color(secondary.color, mismatch_color),
            )
        } else {
            ("", "", "", INSTRUCTION_NEUTRAL_COLOR)
        };

    InstructionChunkContents {
        primary_color: InstructionChunkContent {
            text: primary_label.to_string(),
            color: primary_color,
        },
        primary_action: InstructionChunkContent {
            text: format!(" {}", primary_action),
            color: INSTRUCTION_NEUTRAL_COLOR,
        },
        separator: InstructionChunkContent {
            text: separator.to_string(),
            color: INSTRUCTION_NEUTRAL_COLOR,
        },
        secondary_color: InstructionChunkContent {
            text: secondary_label.to_string(),
            color: secondary_color,
        },
        secondary_action: InstructionChunkContent {
            text: if secondary_action.is_empty() {
                String::new()
            } else {
                format!(" {}", secondary_action)
            },
            color: INSTRUCTION_NEUTRAL_COLOR,
        },
    }
}

fn label_color(color: FlagColor, mismatch_color: bool) -> Color {
    if mismatch_color {
        color.opposite().text_color()
    } else {
        color.text_color()
    }
}

fn spawn_instruction_chunk(
    commands: &mut Commands,
    ui_font: &UiFont,
    content: InstructionChunkContent,
    role: InstructionChunkRole,
) {
    commands.spawn((
        GameUi,
        InstructionChunk { role },
        Text2d::new(content.text),
        TextFont {
            font: ui_font.0.clone(),
            font_size: INSTRUCTION_FONT_SIZE,
            ..default()
        },
        TextColor(content.color),
        TextLayout::new_with_justify(Justify::Left),
        Anchor::TOP_LEFT,
        Transform::from_xyz(0.0, 0.0, 10.0),
        DespawnOnExit(AppState::FlagRaise),
    ));
}

fn update_instruction_chunk_positions(
    chunks: &Query<(Entity, &InstructionChunk, &TextLayoutInfo)>,
    transforms: &mut Query<&mut Transform, With<InstructionChunk>>,
    y: f32,
) {
    let mut ordered: [Option<(Entity, f32)>; 5] = [None, None, None, None, None];
    for (entity, chunk, layout) in chunks {
        ordered[chunk.role.index()] = Some((entity, layout.size.x));
    }

    let total_width: f32 = ordered
        .iter()
        .filter_map(|item| item.map(|(_, width)| width))
        .sum();

    let mut cursor = -total_width * 0.5;
    for item in ordered.iter().flatten() {
        let (entity, width) = *item;
        if let Ok(mut transform) = transforms.get_mut(entity) {
            transform.translation = Vec3::new(cursor, y, 10.0);
        }
        cursor += width;
    }
}

impl InstructionChunkRole {
    fn index(self) -> usize {
        match self {
            InstructionChunkRole::PrimaryColor => 0,
            InstructionChunkRole::PrimaryAction => 1,
            InstructionChunkRole::Separator => 2,
            InstructionChunkRole::SecondaryColor => 3,
            InstructionChunkRole::SecondaryAction => 4,
        }
    }
}
