#![allow(clippy::type_complexity)]

use std::{env, fs, net::UdpSocket, path::PathBuf, time::Duration};

#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};

use bevy::prelude::*;
use clap::Parser;

mod args;
mod breakout;
mod person_image;
mod pose;

const READ_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Resource)]
enum SocketResource {
    Udp(UdpSocket),
    #[cfg(unix)]
    UnixStream {
        listener: UnixListener,
        stream: Option<UnixStream>,
    },
}

fn main() {
    let args = args::Args::parse();

    let (socket, transport_label) =
        build_socket(&args).expect("failed to initialize listening socket");

    info!("Server listening on {transport_label}");

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
            pose::PosePlugin,
            breakout::GamePlugin,
            person_image::PersonImagePlugin,
        ))
        .insert_resource(socket)
        .insert_resource(args)
        .run();
}

fn resolve_transport(args: &args::Args) -> args::Transport {
    match args.transport {
        args::Transport::Auto => {
            if cfg!(unix) {
                args::Transport::Unix
            } else {
                args::Transport::Udp
            }
        }
        other => other,
    }
}

fn build_socket(args: &args::Args) -> std::io::Result<(SocketResource, String)> {
    let transport = resolve_transport(args);
    match transport {
        args::Transport::Udp | args::Transport::Auto => build_udp_socket(args),
        args::Transport::Unix => build_unix_socket(args),
    }
}

fn build_udp_socket(args: &args::Args) -> std::io::Result<(SocketResource, String)> {
    let socket = UdpSocket::bind(&args.udp_addr)?;
    socket.set_nonblocking(true)?;
    socket.set_read_timeout(Some(READ_TIMEOUT))?;
    Ok((
        SocketResource::Udp(socket),
        format!("udp://{}", &args.udp_addr),
    ))
}

#[cfg(unix)]
fn build_unix_socket(args: &args::Args) -> std::io::Result<(SocketResource, String)> {
    let path = args
        .unix_path
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(default_unix_path);

    if path.exists() {
        let _ = fs::remove_file(&path);
    }

    let listener = UnixListener::bind(&path)?;
    listener.set_nonblocking(true)?;
    Ok((
        SocketResource::UnixStream {
            listener,
            stream: None,
        },
        format!("unix://{}", path.display()),
    ))
}

#[cfg(not(unix))]
fn build_unix_socket(_: &args::Args) -> std::io::Result<(SocketResource, String)> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Unix domain sockets are not supported on this platform",
    ))
}

#[cfg(unix)]
fn default_unix_path() -> PathBuf {
    if let Ok(runtime_dir) = env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(runtime_dir).join("pose-de-game.sock");
    }
    PathBuf::from("/tmp/pose-de-game.sock")
}
