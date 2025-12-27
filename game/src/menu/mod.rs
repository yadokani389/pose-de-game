use bevy::prelude::*;

use crate::{AppState, assets::UiFont};

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.18);
const HOVERED_BUTTON: Color = Color::srgb(0.22, 0.22, 0.26);
const PRESSED_BUTTON: Color = Color::srgb(0.28, 0.28, 0.33);
const DISABLED_BUTTON: Color = Color::srgb(0.12, 0.12, 0.14);
const PANEL_BACKGROUND: Color = Color::srgb(0.06, 0.06, 0.08);

pub struct GameMenuPlugin;

impl Plugin for GameMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::MainMenu), setup_menu)
            .add_systems(Update, button_system.run_if(in_state(AppState::MainMenu)));
    }
}

#[derive(Component)]
struct MenuButton(MenuAction);

#[derive(Clone, Copy)]
enum MenuAction {
    StartBreakout,
}

fn setup_menu(mut commands: Commands, ui_font: Res<UiFont>) {
    commands.spawn((Camera2d, DespawnOnExit(AppState::MainMenu)));

    commands
        .spawn((
            DespawnOnExit(AppState::MainMenu),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(24.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("ゲーム選択"),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: 56.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            parent.spawn((
                Text::new("クリックしてゲームを開始"),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.85)),
            ));

            spawn_active_button(
                parent,
                &ui_font,
                "ブロック崩し",
                "（利用可）",
                MenuAction::StartBreakout,
            );
            spawn_disabled_button(parent, &ui_font, "Coming Soon", "（準備中）");
            spawn_disabled_button(parent, &ui_font, "Coming Soon", "（準備中）");
        });
}

fn spawn_active_button(
    parent: &mut ChildSpawnerCommands,
    ui_font: &UiFont,
    title: &str,
    status: &str,
    action: MenuAction,
) {
    parent
        .spawn((
            Button,
            MenuButton(action),
            Node {
                width: Val::Px(420.0),
                height: Val::Px(96.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(6.0),
                border: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            BorderColor::all(Color::BLACK),
            BorderRadius::MAX,
            BackgroundColor(NORMAL_BUTTON),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(title),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: 30.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            parent.spawn((
                Text::new(status),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.85)),
            ));
        });
}

fn spawn_disabled_button(
    parent: &mut ChildSpawnerCommands,
    ui_font: &UiFont,
    title: &str,
    status: &str,
) {
    parent
        .spawn((
            Node {
                width: Val::Px(420.0),
                height: Val::Px(96.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(6.0),
                border: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            BorderColor::all(Color::srgb(0.1, 0.1, 0.1)),
            BorderRadius::MAX,
            BackgroundColor(DISABLED_BUTTON),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(title),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: 30.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.65)),
            ));
            parent.spawn((
                Text::new(status),
                TextFont {
                    font: ui_font.0.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.55)),
            ));
        });
}

fn button_system(
    mut query: Query<
        (
            &Interaction,
            &MenuButton,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, button, mut color, mut border_color) in &mut query {
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
                border_color.set_all(Color::WHITE);
                match button.0 {
                    MenuAction::StartBreakout => next_state.set(AppState::Breakout),
                }
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
                border_color.set_all(Color::srgb(0.7, 0.7, 0.8));
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
                border_color.set_all(Color::BLACK);
            }
        }
    }
}
