use bevy::prelude::*;
use bevy_flair::prelude::*;

use crate::{AppState, pose::disable_pose_runtime};

const GRID_COLUMNS: usize = 3;

pub struct GameMenuPlugin;

impl Plugin for GameMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(AppState::MainMenu),
            (disable_pose_runtime, setup_menu),
        )
        .add_systems(
            Update,
            (
                menu_keyboard_navigation,
                menu_keyboard_activate,
                menu_pointer_interaction,
                apply_selection_classes,
            )
                .chain()
                .run_if(in_state(AppState::MainMenu)),
        );
    }
}

#[derive(Component)]
struct MenuCard {
    index: usize,
}

#[derive(Resource)]
struct MenuSelection {
    index: usize,
}

#[derive(Clone, Copy)]
enum MenuAction {
    StartAirHockey,
    StartEndlessRunner,
    StartFlagRaise,
    StartPoseSync,
    StartFruitCut,
    StartPoseDebug,
}

struct MenuEntry {
    title: &'static str,
    badge: &'static str,
    summary: &'static str,
    status: &'static str,
    action: Option<MenuAction>,
}

const MENU_ENTRIES: &[MenuEntry] = &[
    MenuEntry {
        title: "エアホッケー",
        badge: "PLAY",
        summary: "右手でマレット操作",
        status: "利用可",
        action: Some(MenuAction::StartAirHockey),
    },
    MenuEntry {
        title: "旗上げゲーム",
        badge: "PLAY",
        summary: "命令に合わせて旗を操作",
        status: "利用可",
        action: Some(MenuAction::StartFlagRaise),
    },
    MenuEntry {
        title: "カメラデバッグ",
        badge: "DEBUG",
        summary: "姿勢推定の表示確認",
        status: "デバッグ",
        action: Some(MenuAction::StartPoseDebug),
    },
    MenuEntry {
        title: "ポーズシンクロ",
        badge: "PLAY",
        summary: "見本ポーズを覚えて再現",
        status: "利用可",
        action: Some(MenuAction::StartPoseSync),
    },
    MenuEntry {
        title: "フルーツカット",
        badge: "PLAY",
        summary: "手を振ってフルーツを切る",
        status: "利用可",
        action: Some(MenuAction::StartFruitCut),
    },
    MenuEntry {
        title: "エンドレスランナー",
        badge: "PLAY",
        summary: "体を左右に動かして障害物を回避",
        status: "利用可",
        action: Some(MenuAction::StartEndlessRunner),
    },
    MenuEntry {
        title: "Coming Soon 04",
        badge: "SOON",
        summary: "新しいゲーム準備中",
        status: "準備中",
        action: None,
    },
    MenuEntry {
        title: "Coming Soon 05",
        badge: "SOON",
        summary: "新しいゲーム準備中",
        status: "準備中",
        action: None,
    },
    MenuEntry {
        title: "Coming Soon 06",
        badge: "SOON",
        summary: "新しいゲーム準備中",
        status: "準備中",
        action: None,
    },
    MenuEntry {
        title: "Coming Soon 07",
        badge: "SOON",
        summary: "新しいゲーム準備中",
        status: "準備中",
        action: None,
    },
    MenuEntry {
        title: "Coming Soon 08",
        badge: "SOON",
        summary: "新しいゲーム準備中",
        status: "準備中",
        action: None,
    },
    MenuEntry {
        title: "Coming Soon 09",
        badge: "SOON",
        summary: "新しいゲーム準備中",
        status: "準備中",
        action: None,
    },
    MenuEntry {
        title: "Coming Soon 10",
        badge: "SOON",
        summary: "新しいゲーム準備中",
        status: "準備中",
        action: None,
    },
];

fn setup_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((Camera2d, DespawnOnExit(AppState::MainMenu)));

    let selected = first_enabled_index();
    commands.insert_resource(MenuSelection { index: selected });

    let total = MENU_ENTRIES.len();
    let available = MENU_ENTRIES
        .iter()
        .filter(|entry| entry.action.is_some())
        .count();
    let hint = format!("利用可能: {available}/{total} ・ ↑↓←→で選択 / Enterで開始");

    commands
        .spawn((
            DespawnOnExit(AppState::MainMenu),
            Name::new("menu_root"),
            Node {
                display: Display::None,
                ..default()
            },
            NodeStyleSheet::new(asset_server.load("game_select.css")),
        ))
        .with_children(|parent| {
            parent
                .spawn((Node::default(), ClassList::new("menu-header")))
                .with_children(|header| {
                    header.spawn((Text::new("ゲーム選択"), ClassList::new("menu-title")));
                    header.spawn((
                        Text::new("キーボードでゲームを選択"),
                        ClassList::new("menu-subtitle"),
                    ));
                    header.spawn((Node::default(), ClassList::new("menu-divider")));
                });

            parent
                .spawn((Node::default(), ClassList::new("menu-grid")))
                .with_children(|grid| {
                    for (index, entry) in MENU_ENTRIES.iter().enumerate() {
                        spawn_menu_card(grid, entry, index, selected);
                    }
                });

            parent
                .spawn((Node::default(), ClassList::new("menu-footer")))
                .with_children(|footer| {
                    footer.spawn((Text::new(hint), ClassList::new("menu-hint")));
                });
        });
}

fn spawn_menu_card(
    parent: &mut ChildSpawnerCommands,
    entry: &MenuEntry,
    index: usize,
    selected_index: usize,
) {
    let mut classes = ClassList::new("game-card");
    if entry.action.is_none() {
        classes.add("is-disabled");
    }
    if index == selected_index {
        classes.add("is-selected");
    }

    parent
        .spawn((Button, MenuCard { index }, classes))
        .with_children(|card| {
            card.spawn((Node::default(), ClassList::new("card-header")))
                .with_children(|header| {
                    header.spawn((Text::new(entry.title), ClassList::new("card-title")));
                    header.spawn((Text::new(entry.badge), ClassList::new("card-badge")));
                });

            card.spawn((Text::new(entry.summary), ClassList::new("card-summary")));

            card.spawn((Node::default(), ClassList::new("card-footer")))
                .with_children(|footer| {
                    footer.spawn((Text::new(entry.status), ClassList::new("card-status")));
                });
        });
}

fn menu_keyboard_navigation(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut selection: ResMut<MenuSelection>,
) {
    let delta = if pressed_any(&keyboard_input, &[KeyCode::ArrowLeft, KeyCode::KeyA]) {
        Some(-1)
    } else if pressed_any(&keyboard_input, &[KeyCode::ArrowRight, KeyCode::KeyD]) {
        Some(1)
    } else if pressed_any(&keyboard_input, &[KeyCode::ArrowUp, KeyCode::KeyW]) {
        Some(-(GRID_COLUMNS as isize))
    } else if pressed_any(&keyboard_input, &[KeyCode::ArrowDown, KeyCode::KeyS]) {
        Some(GRID_COLUMNS as isize)
    } else {
        None
    };

    if let Some(delta) = delta {
        let next = move_selection(selection.index, delta);
        if next != selection.index {
            selection.index = next;
        }
    }
}

fn menu_keyboard_activate(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    selection: Res<MenuSelection>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if (keyboard_input.just_pressed(KeyCode::Enter) || keyboard_input.just_pressed(KeyCode::Space))
        && let Some(action) = action_for(selection.index)
    {
        apply_action(action, &mut next_state);
    }
}

fn menu_pointer_interaction(
    mut interactions: Query<(&Interaction, &MenuCard), Changed<Interaction>>,
    mut selection: ResMut<MenuSelection>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, card) in &mut interactions {
        if !is_enabled(card.index) {
            continue;
        }

        match *interaction {
            Interaction::Pressed => {
                selection.index = card.index;
                if let Some(action) = action_for(card.index) {
                    apply_action(action, &mut next_state);
                }
            }
            Interaction::Hovered => {
                selection.index = card.index;
            }
            Interaction::None => {}
        }
    }
}

fn apply_selection_classes(
    selection: Res<MenuSelection>,
    mut cards: Query<(&MenuCard, &mut ClassList)>,
) {
    if !selection.is_changed() {
        return;
    }

    for (card, mut classes) in &mut cards {
        if card.index == selection.index {
            classes.add("is-selected");
        } else {
            classes.remove("is-selected");
        }
    }
}

fn apply_action(action: MenuAction, next_state: &mut NextState<AppState>) {
    match action {
        MenuAction::StartAirHockey => next_state.set(AppState::AirHockey),
        MenuAction::StartEndlessRunner => next_state.set(AppState::EndlessRunner),
        MenuAction::StartFlagRaise => next_state.set(AppState::FlagRaise),
        MenuAction::StartPoseSync => next_state.set(AppState::PoseSync),
        MenuAction::StartFruitCut => next_state.set(AppState::FruitCut),
        MenuAction::StartPoseDebug => next_state.set(AppState::PoseDebug),
    }
}

fn move_selection(current: usize, delta: isize) -> usize {
    let mut next = current as isize;
    loop {
        next += delta;
        if next < 0 || next >= MENU_ENTRIES.len() as isize {
            return current;
        }
        let candidate = next as usize;
        if is_enabled(candidate) {
            return candidate;
        }
    }
}

fn first_enabled_index() -> usize {
    MENU_ENTRIES
        .iter()
        .position(|entry| entry.action.is_some())
        .unwrap_or(0)
}

fn is_enabled(index: usize) -> bool {
    MENU_ENTRIES
        .get(index)
        .is_some_and(|entry| entry.action.is_some())
}

fn action_for(index: usize) -> Option<MenuAction> {
    MENU_ENTRIES.get(index).and_then(|entry| entry.action)
}

fn pressed_any(keyboard_input: &ButtonInput<KeyCode>, keys: &[KeyCode]) -> bool {
    keys.iter().any(|key| keyboard_input.just_pressed(*key))
}
