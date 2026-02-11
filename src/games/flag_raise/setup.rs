use crate::{AppState, assets::UiFont, pose::PoseRenderSettings};
use bevy::prelude::*;
use bevy::sprite::Anchor;

use super::game::{CommandRng, start_game};
use super::render::{setup_flag_render_assets, setup_render_settings, setup_slot_line_assets};
use super::settings::{FlagRaisePhase, FlagRaiseSettings, MAX_PLAYERS, MIN_PLAYERS};
use super::ui;

const SETUP_TITLE_SIZE: f32 = 58.0;
const SETUP_LABEL_SIZE: f32 = 36.0;
const SETUP_VALUE_SIZE: f32 = 44.0;
const SETUP_HINT_SIZE: f32 = 28.0;
const SETUP_TITLE_Y: f32 = 220.0;
const SETUP_PLAYERS_LABEL_Y: f32 = 100.0;
const SETUP_PLAYERS_VALUE_Y: f32 = 50.0;
const SETUP_DIFFICULTY_LABEL_Y: f32 = -10.0;
const SETUP_DIFFICULTY_VALUE_Y: f32 = -60.0;
const SETUP_MODE_LABEL_Y: f32 = -130.0;
const SETUP_MODE_VALUE_Y: f32 = -180.0;
const SETUP_HINT_Y: f32 = -260.0;

#[derive(Component)]
pub(super) struct SetupUi;

#[derive(Component)]
pub(super) struct PlayersValueText;

#[derive(Component)]
pub(super) struct DifficultyValueText;

#[derive(Component)]
pub(super) struct ModeValueText;

pub(super) fn enter_flag_raise(
    mut commands: Commands,
    ui_font: Res<UiFont>,
    mut render_settings: ResMut<PoseRenderSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((Camera2d, DespawnOnExit(AppState::FlagRaise)));
    commands.insert_resource(FlagRaisePhase::Setup);
    let settings = FlagRaiseSettings::default();
    commands.insert_resource(settings);
    commands.insert_resource(CommandRng::default());

    setup_render_settings(&mut render_settings);
    setup_slot_line_assets(&mut commands, &mut meshes, &mut materials);
    setup_flag_render_assets(&mut commands, &mut meshes, &mut materials);

    spawn_setup_ui(&mut commands, &ui_font, &settings);
}

pub(super) fn exit_flag_raise(mut commands: Commands) {
    commands.remove_resource::<FlagRaisePhase>();
    commands.remove_resource::<FlagRaiseSettings>();
    commands.remove_resource::<CommandRng>();
    commands.remove_resource::<super::game::CommandState>();
    commands.remove_resource::<super::game::Scoreboard>();
    commands.remove_resource::<super::game::GameTimer>();
    commands.remove_resource::<super::game::GameResult>();
    commands.remove_resource::<super::render::SlotLineAssets>();
    commands.remove_resource::<super::render::FlagRenderAssets>();
}

pub(super) fn return_to_setup(
    commands: &mut Commands,
    ui_font: &UiFont,
    settings: &FlagRaiseSettings,
    phase: &mut FlagRaisePhase,
) {
    commands.remove_resource::<super::game::CommandState>();
    commands.remove_resource::<super::game::Scoreboard>();
    commands.remove_resource::<super::game::GameTimer>();
    commands.remove_resource::<super::game::GameResult>();
    spawn_setup_ui(commands, ui_font, settings);
    *phase = FlagRaisePhase::Setup;
}

pub(super) fn handle_escape_to_menu(
    input: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if input.just_pressed(KeyCode::Escape) || mouse_buttons.just_pressed(MouseButton::Back) {
        next_state.set(AppState::MainMenu);
    }
}

pub(super) fn setup_input(
    input: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<FlagRaiseSettings>,
    mut phase: ResMut<FlagRaisePhase>,
    mut commands: Commands,
    ui_font: Res<UiFont>,
    time: Res<Time>,
    mut rng: ResMut<CommandRng>,
    setup_entities: Query<Entity, With<SetupUi>>,
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
    if input.just_pressed(KeyCode::ArrowRight) {
        settings.difficulty = settings.difficulty.next();
        changed = true;
    }
    if input.just_pressed(KeyCode::ArrowLeft) {
        settings.difficulty = settings.difficulty.prev();
        changed = true;
    }
    if input.just_pressed(KeyCode::KeyM) {
        settings.mode = settings.mode.next();
        changed = true;
    }

    if input.just_pressed(KeyCode::Enter) {
        for entity in &setup_entities {
            commands.entity(entity).despawn();
        }
        *phase = FlagRaisePhase::Playing;
        let initial_command = start_game(&mut commands, &time, &settings, &mut rng);
        ui::spawn_game_ui(&mut commands, &ui_font, &initial_command, false);
    }

    if changed {
        settings.player_count = settings.player_count.clamp(MIN_PLAYERS, MAX_PLAYERS);
    }
}

pub(super) fn update_setup_text(
    settings: Res<FlagRaiseSettings>,
    mut players_query: Query<
        &mut Text2d,
        (
            With<PlayersValueText>,
            Without<DifficultyValueText>,
            Without<ModeValueText>,
        ),
    >,
    mut difficulty_query: Query<
        &mut Text2d,
        (
            With<DifficultyValueText>,
            Without<PlayersValueText>,
            Without<ModeValueText>,
        ),
    >,
    mut mode_query: Query<
        &mut Text2d,
        (
            With<ModeValueText>,
            Without<PlayersValueText>,
            Without<DifficultyValueText>,
        ),
    >,
) {
    if !settings.is_changed() {
        return;
    }

    if let Ok(mut text) = players_query.single_mut() {
        *text = Text2d::new(format!("{}", settings.player_count));
    }

    if let Ok(mut text) = difficulty_query.single_mut() {
        *text = Text2d::new(settings.difficulty.label());
    }

    if let Ok(mut text) = mode_query.single_mut() {
        *text = Text2d::new(settings.mode.label());
    }
}

pub(super) fn spawn_setup_ui(
    commands: &mut Commands,
    ui_font: &UiFont,
    settings: &FlagRaiseSettings,
) {
    commands.spawn((
        SetupUi,
        Text2d::new("旗上げゲーム"),
        TextFont {
            font: ui_font.0.clone(),
            font_size: SETUP_TITLE_SIZE,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
        TextColor(Color::srgb(0.95, 0.95, 0.98)),
        Anchor::CENTER,
        Transform::from_xyz(0.0, SETUP_TITLE_Y, 10.0),
        DespawnOnExit(AppState::FlagRaise),
    ));

    commands.spawn((
        SetupUi,
        Text2d::new("人数"),
        TextFont {
            font: ui_font.0.clone(),
            font_size: SETUP_LABEL_SIZE,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
        TextColor(Color::srgb(0.92, 0.92, 0.96)),
        Anchor::CENTER,
        Transform::from_xyz(0.0, SETUP_PLAYERS_LABEL_Y, 10.0),
        DespawnOnExit(AppState::FlagRaise),
    ));

    commands.spawn((
        SetupUi,
        PlayersValueText,
        Text2d::new(format!("{}", settings.player_count)),
        TextFont {
            font: ui_font.0.clone(),
            font_size: SETUP_VALUE_SIZE,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
        TextColor(Color::srgb(0.98, 0.78, 0.2)),
        Anchor::CENTER,
        Transform::from_xyz(0.0, SETUP_PLAYERS_VALUE_Y, 10.0),
        DespawnOnExit(AppState::FlagRaise),
    ));

    commands.spawn((
        SetupUi,
        Text2d::new("難易度"),
        TextFont {
            font: ui_font.0.clone(),
            font_size: SETUP_LABEL_SIZE,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
        TextColor(Color::srgb(0.92, 0.92, 0.96)),
        Anchor::CENTER,
        Transform::from_xyz(0.0, SETUP_DIFFICULTY_LABEL_Y, 10.0),
        DespawnOnExit(AppState::FlagRaise),
    ));

    commands.spawn((
        SetupUi,
        DifficultyValueText,
        Text2d::new(settings.difficulty.label()),
        TextFont {
            font: ui_font.0.clone(),
            font_size: SETUP_VALUE_SIZE,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
        TextColor(Color::srgb(0.2, 0.88, 0.95)),
        Anchor::CENTER,
        Transform::from_xyz(0.0, SETUP_DIFFICULTY_VALUE_Y, 10.0),
        DespawnOnExit(AppState::FlagRaise),
    ));

    commands.spawn((
        SetupUi,
        Text2d::new("モード"),
        TextFont {
            font: ui_font.0.clone(),
            font_size: SETUP_LABEL_SIZE,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
        TextColor(Color::srgb(0.92, 0.92, 0.96)),
        Anchor::CENTER,
        Transform::from_xyz(0.0, SETUP_MODE_LABEL_Y, 10.0),
        DespawnOnExit(AppState::FlagRaise),
    ));

    commands.spawn((
        SetupUi,
        ModeValueText,
        Text2d::new(settings.mode.label()),
        TextFont {
            font: ui_font.0.clone(),
            font_size: SETUP_VALUE_SIZE,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
        TextColor(Color::srgb(0.8, 0.9, 0.5)),
        Anchor::CENTER,
        Transform::from_xyz(0.0, SETUP_MODE_VALUE_Y, 10.0),
        DespawnOnExit(AppState::FlagRaise),
    ));

    commands.spawn((
        SetupUi,
        Text2d::new("↑↓ 人数  ←→ 難易度  M モード  Enter 開始  Esc/MouseBack 戻る"),
        TextFont {
            font: ui_font.0.clone(),
            font_size: SETUP_HINT_SIZE,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
        TextColor(Color::srgb(0.7, 0.7, 0.8)),
        Anchor::CENTER,
        Transform::from_xyz(0.0, SETUP_HINT_Y, 10.0),
        DespawnOnExit(AppState::FlagRaise),
    ));
}
