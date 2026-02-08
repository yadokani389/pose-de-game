#![allow(clippy::type_complexity, clippy::too_many_arguments)]

use bevy::prelude::*;
use bevy_flair::prelude::FlairPlugin;
use clap::Parser;

mod args;
mod assets;
mod games;
mod infer;
mod menu;
mod pose;
mod utils;

#[derive(States, Clone, Eq, PartialEq, Debug, Hash, Default)]
#[states(scoped_entities)]
pub enum AppState {
    #[default]
    MainMenu,
    AirHockey,
    FlagRaise,
    PoseSync,
    PoseDebug,
}

fn main() {
    let args = args::Args::parse();

    if args.list_cameras {
        pose::list_cameras();
        return;
    }

    let mut app = App::new();
    pose::setup_runtime(&mut app, &args).expect("failed to initialize camera inference");

    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                // fill the entire browser window
                fit_canvas_to_parent: true,
                // don't hijack keyboard shortcuts like F5, F6, F12, Ctrl+R etc.
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }),
        FlairPlugin,
        pose::PosePlugin,
        assets::EmbeddedAssetsPlugin,
        menu::GameMenuPlugin,
        games::air_hockey::AirHockeyPlugin,
        games::flag_raise::FlagRaisePlugin,
        games::pose_sync::PoseSyncPlugin,
        games::pose_debug::PoseDebugPlugin,
    ))
    .add_systems(Startup, utils::pitch::setup_beeps)
    .init_state::<AppState>()
    .insert_resource(args)
    .run();
}
