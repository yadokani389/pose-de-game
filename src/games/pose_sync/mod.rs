mod game;
mod render;
mod result;
mod settings;
mod setup;
mod ui;

use bevy::prelude::*;

use crate::{
    AppState,
    pose::{disable_pose_render, disable_pose_runtime, enable_pose_render, enable_pose_runtime},
    utils::update_floating_text_popups,
};

use settings::{is_playing, is_result, is_setup};

pub struct PoseSyncPlugin;

impl Plugin for PoseSyncPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(AppState::PoseSync),
            (
                enable_pose_runtime,
                enable_pose_render,
                setup::enter_pose_sync,
            ),
        )
        .add_systems(
            OnExit(AppState::PoseSync),
            (
                setup::exit_pose_sync,
                disable_pose_render,
                disable_pose_runtime,
            ),
        )
        .add_systems(
            Update,
            (
                setup::handle_escape_to_menu,
                setup::setup_input.run_if(is_setup),
                setup::update_setup_text.run_if(is_setup),
                render::sync_slot_lines,
                game::advance_turn_and_score
                    .run_if(is_playing)
                    .run_if(resource_exists::<game::CommandState>)
                    .run_if(resource_exists::<game::GameTimer>)
                    .run_if(resource_exists::<game::Scoreboard>)
                    .run_if(resource_exists::<game::PlayerSlotAssignments>),
                render::draw_pose_preview
                    .run_if(is_playing)
                    .run_if(resource_exists::<game::CommandState>),
                ui::update_score_texts
                    .run_if(is_playing)
                    .run_if(resource_exists::<game::Scoreboard>),
                ui::update_hint_text
                    .run_if(is_playing)
                    .run_if(resource_exists::<game::CommandState>),
                ui::update_turn_timer_text
                    .run_if(is_playing)
                    .run_if(resource_exists::<game::CommandState>),
                ui::update_game_timer_text
                    .run_if(is_playing)
                    .run_if(resource_exists::<game::GameTimer>),
                ui::update_text_positions.run_if(is_playing),
                update_floating_text_popups,
                result::spawn_result_ui.run_if(is_result),
                result::result_input.run_if(is_result),
                result::button_system.run_if(is_result),
            )
                .run_if(in_state(AppState::PoseSync)),
        );
    }
}
