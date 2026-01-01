use bevy::{camera::ScalingMode, prelude::*};
use bevy_egui::PrimaryEguiContext;
use bevy_ggrs::prelude::*;
use components::Team;
use matchbox_socket::PeerId;

use crate::AppState;

mod ball;
mod components;
pub mod field;
mod item;
mod menu;
mod online;
mod paddle;
mod pose_visualize;
mod timer;

type Config = bevy_ggrs::GgrsConfig<f32, PeerId>;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            GgrsPlugin::<Config>::default(),
            menu::MenuPlugin,
            ball::BallPlugin,
            field::FieldPlugin,
            paddle::PaddlePlugin,
            online::OnlinePlugin,
            item::ItemPlugin,
            timer::TimerPlugin,
        ))
        .init_state::<GameState>()
        .add_systems(OnEnter(AppState::Breakout), enter_breakout)
        .add_systems(OnEnter(AppState::Breakout), setup_graphics)
        .add_systems(
            OnEnter(AppState::Breakout),
            pose_visualize::create_image.run_if(pose_visualize::show_person_enabled),
        )
        .add_systems(
            Update,
            (
                pose_visualize::show_right_hand,
                pose_visualize::show_person_image.run_if(pose_visualize::show_person_enabled),
            )
                .run_if(in_state(AppState::Breakout)),
        )
        .add_systems(
            GgrsSchedule,
            despawn_out_of_bounds_entities
                .after(field::toggle_cell)
                .run_if(in_state(AppState::Breakout)),
        )
        .rollback_component_with_clone::<Transform>()
        .rollback_component_with_copy::<Team>();
    }
}

#[derive(States, Clone, Eq, PartialEq, Debug, Hash, Default)]
#[states(scoped_entities)]
enum GameState {
    #[default]
    Boot,
    Lobby,
    Matchmaking,
    InGame,
    GameOver,
}

fn enter_breakout(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::Lobby);
}

fn setup_graphics(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        PrimaryEguiContext,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 1100.,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}

fn despawn_out_of_bounds_entities(mut commands: Commands, query: Query<(Entity, &Transform)>) {
    for (entity, transform) in query {
        if 1200. < transform.translation.x.abs() || 2000. < transform.translation.y.abs() {
            commands.entity(entity).despawn();
        }
    }
}
