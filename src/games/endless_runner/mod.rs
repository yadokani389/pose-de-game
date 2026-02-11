mod game;
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
use setup::is_setup;

pub const GAME_WORLD_HEIGHT: f32 = 800.0;
pub const CAMERA_USED_PORTION: f32 = 0.8;
pub const CAMERA_USED_MARGIN: f32 = (1.0 - CAMERA_USED_PORTION) * 0.5;

pub fn game_world_size(window: &Window) -> Vec2 {
    let aspect = window.width() / window.height();
    Vec2::new(GAME_WORLD_HEIGHT * aspect, GAME_WORLD_HEIGHT)
}

pub struct EndlessRunnerPlugin;

impl Plugin for EndlessRunnerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<game::EndlessRunnerPhase>()
            .init_resource::<game::GameSettings>()
            .init_resource::<game::GameSpeed>()
            .init_resource::<game::Scoreboard>()
            .init_resource::<game::ObstacleSpawner>()
            .add_systems(
                OnEnter(AppState::EndlessRunner),
                (enable_pose_runtime, enable_pose_frame_capture, game::setup),
            )
            .add_systems(
                OnExit(AppState::EndlessRunner),
                (
                    game::cleanup,
                    game::cleanup_camera_overlay,
                    disable_pose_frame_capture,
                    disable_pose_runtime,
                ),
            )
            .add_systems(
                Update,
                (
                    setup::handle_escape_to_menu,
                    game::update_camera_overlay,
                    setup::setup_ui_on_enter,
                    setup::handle_setup_input.run_if(is_setup),
                    setup::handle_setup_phase.run_if(is_setup),
                    game::update_player_target_lane.run_if(is_playing),
                    game::update_nose_markers.run_if(is_playing),
                    game::move_players.run_if(is_playing),
                    game::spawn_obstacles.run_if(is_playing),
                    game::move_obstacles.run_if(is_playing),
                    game::check_collisions.run_if(is_playing),
                    game::update_distances.run_if(is_playing),
                    game::check_game_over.run_if(is_playing),
                    ui::update_hud.run_if(is_playing),
                    result::spawn_result_ui.run_if(is_result),
                    result::handle_result_input.run_if(is_result),
                    result::button_system.run_if(is_result),
                )
                    .run_if(in_state(AppState::EndlessRunner)),
            );
    }
}
