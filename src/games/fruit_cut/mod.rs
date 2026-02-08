mod game;
mod hand_tracker;
mod input;
mod render;
mod result;
mod setup;
mod ui;

use bevy::prelude::*;

use crate::{
    AppState,
    pose::{
        disable_pose_frame_capture, disable_pose_runtime, enable_pose_frame_capture,
        enable_pose_runtime,
    },
};

use game::{is_playing, is_result};

pub const GAME_WORLD_HEIGHT: f32 = 800.0;
pub const CAMERA_USED_PORTION: f32 = 0.8;
pub const CAMERA_USED_MARGIN: f32 = (1.0 - CAMERA_USED_PORTION) * 0.5;

pub fn game_world_size(window: &Window) -> Vec2 {
    let aspect = window.width() / window.height();
    Vec2::new(GAME_WORLD_HEIGHT * aspect, GAME_WORLD_HEIGHT)
}

pub fn camera_display_size(window: &Window, frame_size: Option<Vec2>) -> Vec2 {
    let world = game_world_size(window);
    let world_aspect = world.x / world.y;
    let frame_aspect = frame_size.map(|s| s.x / s.y).unwrap_or(world_aspect);

    if frame_aspect > world_aspect {
        let width = world.x;
        Vec2::new(width, width / frame_aspect)
    } else {
        let height = world.y;
        Vec2::new(height * frame_aspect, height)
    }
}

pub fn map_pose_to_world(normalized: Vec2, mapped_size: Vec2) -> Vec2 {
    let nx = normalized
        .x
        .clamp(CAMERA_USED_MARGIN, 1.0 - CAMERA_USED_MARGIN);
    let ny = normalized
        .y
        .clamp(CAMERA_USED_MARGIN, 1.0 - CAMERA_USED_MARGIN);
    let x = ((nx - 0.5) / CAMERA_USED_PORTION) * mapped_size.x;
    let y = ((0.5 - ny) / CAMERA_USED_PORTION) * mapped_size.y;
    Vec2::new(x, y)
}

pub struct FruitCutPlugin;

impl Plugin for FruitCutPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<game::FruitCutPhase>()
            .init_resource::<game::Scoreboard>()
            .init_resource::<game::GameTimer>()
            .init_resource::<game::ComboState>()
            .init_resource::<game::FruitSpawner>()
            .init_resource::<game::HandSelection>()
            .init_resource::<hand_tracker::HandTrackers>()
            .add_systems(
                OnEnter(AppState::FruitCut),
                (enable_pose_runtime, enable_pose_frame_capture, setup::setup),
            )
            .add_systems(
                OnExit(AppState::FruitCut),
                (
                    setup::cleanup,
                    setup::cleanup_camera_overlay,
                    disable_pose_frame_capture,
                    disable_pose_runtime,
                ),
            )
            .add_systems(
                Update,
                (
                    setup::handle_escape_to_menu,
                    input::handle_hand_toggle,
                    setup::update_camera_overlay,
                    hand_tracker::update_hand_trackers,
                    setup::handle_setup_phase.run_if(setup::is_setup),
                    game::spawn_fruits.run_if(is_playing),
                    game::update_fruits.run_if(is_playing),
                    game::check_fruit_slicing.run_if(is_playing),
                    game::update_game_timer.run_if(is_playing),
                    render::render_hand_trails,
                    ui::update_hud.run_if(is_playing),
                    result::spawn_result_ui.run_if(is_result),
                    result::handle_result_input.run_if(is_result),
                    result::button_system.run_if(is_result),
                )
                    .run_if(in_state(AppState::FruitCut)),
            );
    }
}
