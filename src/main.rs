#![allow(clippy::type_complexity)]

use bevy::prelude::*;
use clap::Parser;

mod args;
mod assets;
mod games;
mod infer;
mod menu;
mod pose;

#[derive(States, Clone, Eq, PartialEq, Debug, Hash, Default)]
#[states(scoped_entities)]
pub enum AppState {
    #[default]
    MainMenu,
    Breakout,
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
        pose::PosePlugin,
        assets::EmbeddedAssetsPlugin,
        menu::GameMenuPlugin,
        games::breakout::GamePlugin,
        games::pose_debug::PoseDebugPlugin,
    ))
    .init_state::<AppState>()
    .insert_resource(args)
    .run();
}
