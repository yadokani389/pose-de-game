use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::{AppState, assets::UiFont, pose::PoseRenderSettings};

use super::{
    game::{self, PoseRng},
    render,
    settings::{MAX_PLAYERS, MIN_PLAYERS, PoseSyncPhase, PoseSyncSettings},
    ui,
};

const SETUP_TITLE_SIZE: f32 = 58.0;
const SETUP_LABEL_SIZE: f32 = 36.0;
const SETUP_VALUE_SIZE: f32 = 44.0;
const SETUP_HINT_SIZE: f32 = 28.0;
const SETUP_TITLE_Y: f32 = 220.0;
const SETUP_PLAYERS_LABEL_Y: f32 = 100.0;
const SETUP_PLAYERS_VALUE_Y: f32 = 50.0;
const SETUP_DIFFICULTY_LABEL_Y: f32 = -10.0;
const SETUP_DIFFICULTY_VALUE_Y: f32 = -60.0;
const SETUP_HINT_Y: f32 = -260.0;

#[derive(Component)]
pub(super) struct SetupUi;

#[derive(Component)]
pub(super) struct PlayersValueText;

#[derive(Component)]
pub(super) struct DifficultyValueText;

pub(super) fn enter_pose_sync(
    mut commands: Commands,
    ui_font: Res<UiFont>,
    mut render_settings: ResMut<PoseRenderSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((Camera2d, DespawnOnExit(AppState::PoseSync)));
    commands.insert_resource(PoseSyncPhase::Setup);
    let settings = PoseSyncSettings::default();
    commands.insert_resource(settings);
    commands.insert_resource(PoseRng::default());

    render::setup_render_settings(&mut render_settings);
    render::setup_slot_line_assets(&mut commands, &mut meshes, &mut materials);
    render::setup_pose_preview_assets(&mut commands, &mut meshes, &mut materials);

    spawn_setup_ui(&mut commands, &ui_font, &settings);
}

pub(super) fn exit_pose_sync(mut commands: Commands) {
    commands.remove_resource::<PoseSyncPhase>();
    commands.remove_resource::<PoseSyncSettings>();
    commands.remove_resource::<PoseRng>();
    commands.remove_resource::<game::CommandState>();
    commands.remove_resource::<game::Scoreboard>();
    commands.remove_resource::<game::GameTimer>();
    commands.remove_resource::<game::GameResult>();
    commands.remove_resource::<render::SlotLineAssets>();
    commands.remove_resource::<render::PosePreviewAssets>();
}

pub(super) fn return_to_setup(
    commands: &mut Commands,
    ui_font: &UiFont,
    settings: &PoseSyncSettings,
    phase: &mut PoseSyncPhase,
) {
    commands.remove_resource::<game::CommandState>();
    commands.remove_resource::<game::Scoreboard>();
    commands.remove_resource::<game::GameTimer>();
    commands.remove_resource::<game::GameResult>();
    spawn_setup_ui(commands, ui_font, settings);
    *phase = PoseSyncPhase::Setup;
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
    mut settings: ResMut<PoseSyncSettings>,
    mut phase: ResMut<PoseSyncPhase>,
    mut commands: Commands,
    ui_font: Res<UiFont>,
    time: Res<Time>,
    mut rng: ResMut<PoseRng>,
    setup_entities: Query<Entity, With<SetupUi>>,
) {
    let mut changed = false;
    if input.just_pressed(KeyCode::ArrowUp) && settings.player_count < MAX_PLAYERS {
        settings.player_count += 1;
        changed = true;
    }
    if input.just_pressed(KeyCode::ArrowDown) && MIN_PLAYERS < settings.player_count {
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

    if input.just_pressed(KeyCode::Enter) {
        for entity in &setup_entities {
            commands.entity(entity).despawn();
        }
        *phase = PoseSyncPhase::Playing;
        game::start_game(&mut commands, &time, &mut rng);
        ui::spawn_game_ui(&mut commands, &ui_font);
    }

    if changed {
        settings.player_count = settings.player_count.clamp(MIN_PLAYERS, MAX_PLAYERS);
    }
}

pub(super) fn update_setup_text(
    settings: Res<PoseSyncSettings>,
    mut players_query: Query<&mut Text2d, (With<PlayersValueText>, Without<DifficultyValueText>)>,
    mut difficulty_query: Query<
        &mut Text2d,
        (With<DifficultyValueText>, Without<PlayersValueText>),
    >,
) {
    if !settings.is_changed() {
        return;
    }

    if let Ok(mut text) = players_query.single_mut() {
        *text = Text2d::new(settings.player_count.to_string());
    }

    if let Ok(mut text) = difficulty_query.single_mut() {
        *text = Text2d::new(settings.difficulty.label());
    }
}

pub(super) fn spawn_setup_ui(
    commands: &mut Commands,
    ui_font: &UiFont,
    settings: &PoseSyncSettings,
) {
    commands.spawn((
        SetupUi,
        Text2d::new("ポーズシンクロ"),
        TextFont {
            font: ui_font.0.clone(),
            font_size: SETUP_TITLE_SIZE,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
        TextColor(Color::srgb(0.95, 0.95, 0.98)),
        Anchor::CENTER,
        Transform::from_xyz(0.0, SETUP_TITLE_Y, 10.0),
        DespawnOnExit(AppState::PoseSync),
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
        DespawnOnExit(AppState::PoseSync),
    ));

    commands.spawn((
        SetupUi,
        PlayersValueText,
        Text2d::new(settings.player_count.to_string()),
        TextFont {
            font: ui_font.0.clone(),
            font_size: SETUP_VALUE_SIZE,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
        TextColor(Color::srgb(0.98, 0.78, 0.2)),
        Anchor::CENTER,
        Transform::from_xyz(0.0, SETUP_PLAYERS_VALUE_Y, 10.0),
        DespawnOnExit(AppState::PoseSync),
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
        DespawnOnExit(AppState::PoseSync),
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
        DespawnOnExit(AppState::PoseSync),
    ));

    commands.spawn((
        SetupUi,
        Text2d::new("3つの見本を見てから3連続で再現  ↑↓ 人数  ←→ 難易度  Enter 開始"),
        TextFont {
            font: ui_font.0.clone(),
            font_size: SETUP_HINT_SIZE,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
        TextColor(Color::srgb(0.7, 0.7, 0.8)),
        Anchor::CENTER,
        Transform::from_xyz(0.0, SETUP_HINT_Y, 10.0),
        DespawnOnExit(AppState::PoseSync),
    ));
}
