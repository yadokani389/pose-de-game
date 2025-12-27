use bevy::{color::palettes::css::GRAY, prelude::*};
use bevy_ggrs::LocalPlayers;

use crate::{
    AppState,
    assets::UiFont,
    games::breakout::{GameState, components::Team, timer::GameResult},
};

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.35, 0.35);

pub struct ResultPlugin;

impl Plugin for ResultPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::GameOver),
            setup_result_screen.run_if(in_state(AppState::Breakout)),
        )
        .add_systems(
            Update,
            button_system
                .run_if(in_state(GameState::GameOver))
                .run_if(in_state(AppState::Breakout)),
        );
    }
}

#[derive(Component)]
struct ResultMenuButton;

fn setup_result_screen(
    mut commands: Commands,
    ui_font: Res<UiFont>,
    game_result: Option<Res<GameResult>>,
    local_players: Res<LocalPlayers>,
) {
    let Some(result) = game_result else {
        return;
    };

    // Get local player's team
    let local_team = local_players.0.first().copied().unwrap_or(0);

    // Victory message
    let (winner_text, winner_color, score_text) = match result.winner {
        Some(Team(0)) => {
            let personal_message = if local_team == 0 {
                "You Win!"
            } else {
                "You Lose!"
            };
            (
                personal_message,
                if local_team == 0 {
                    Color::hsl(Team(0).hue(), 0.8, 0.7)
                } else {
                    Color::srgb(0.8, 0.3, 0.3)
                },
                format!(
                    "Player 1: {} blocks vs Player 2: {} blocks",
                    result.team0_blocks, result.team1_blocks
                ),
            )
        }
        Some(Team(1)) => {
            let personal_message = if local_team == 1 {
                "You Win!"
            } else {
                "You Lose!"
            };
            (
                personal_message,
                if local_team == 1 {
                    Color::hsl(Team(1).hue(), 0.8, 0.7)
                } else {
                    Color::srgb(0.8, 0.3, 0.3)
                },
                format!(
                    "Player 2: {} blocks vs Player 1: {} blocks",
                    result.team1_blocks, result.team0_blocks
                ),
            )
        }
        None => (
            "It's a Draw!",
            Color::srgb(0.7, 0.7, 0.7),
            format!("Both players: {} blocks", result.team0_blocks),
        ),
        Some(_) => ("Game Over", Color::WHITE, "".to_string()),
    };

    // Main UI container
    commands.spawn((
        DespawnOnExit(GameState::GameOver),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
        children![
            (
                Text::new("GAME OVER"),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(30.0)),
                    ..default()
                },
            ),
            (
                // Winner announcement
                Text::new(winner_text),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: 36.0,
                    ..default()
                },
                TextColor(winner_color),
                Node {
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                },
            ),
            (
                // Score display
                Text::new(score_text),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                Node {
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..default()
                },
            ),
            (
                Button,
                ResultMenuButton,
                Node {
                    width: Val::Px(360.0),
                    height: Val::Px(84.0),
                    border: UiRect::all(Val::Px(4.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BorderColor::all(Color::BLACK),
                BorderRadius::MAX,
                BackgroundColor(NORMAL_BUTTON),
                children![(
                    Text::new("メニューへ戻る"),
                    TextFont {
                        font: ui_font.0.clone(),
                        font_size: 28.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                )],
            ),
        ],
    ));
}

fn button_system(
    mut query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<Button>, With<ResultMenuButton>),
    >,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut next_game_state: ResMut<NextState<GameState>>,
) {
    for (interaction, mut color, mut border_color) in &mut query {
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
                border_color.set_all(GRAY);
                next_game_state.set(GameState::Lobby);
                next_app_state.set(AppState::MainMenu);
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
