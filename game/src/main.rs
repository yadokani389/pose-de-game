use std::{net::UdpSocket, time::Duration};

use bevy::prelude::*;

mod game;

const LISTEN_ADDRESS: &str = "127.0.0.1:45233";

#[derive(Resource)]
struct UdpSocketResource(UdpSocket);

fn main() {
    let socket = UdpSocket::bind(LISTEN_ADDRESS).expect("could not bind socket");
    socket
        .set_nonblocking(true)
        .expect("could not set socket to be nonblocking");
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("could not set read timeout");

    info!("Server now listening on {}", LISTEN_ADDRESS);

    App::new()
        .add_plugins((
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
            game::GamePlugin,
        ))
        .insert_resource(UdpSocketResource(socket))
        .run();
}
