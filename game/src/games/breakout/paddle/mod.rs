use bevy::{
    math::bounding::{Aabb2d, IntersectsVolume},
    platform::collections::HashMap,
    prelude::*,
};
use bevy_ggrs::{LocalInputs, LocalPlayers, PlayerInputs, prelude::*};

use crate::{AppState, pose::PeopleDataRes};

use super::field::{CELL_SIZE, FIELD_WIDTH, SideWall};
use super::{Config, components::Team};
use super::{GameState, field::Wall};

pub const PADDLE_WIDTH: f32 = 100.0;
pub const PADDLE_HEIGHT: f32 = 10.0;
const PADDLE_Y_POSITION: f32 = 450.0;

pub struct PaddlePlugin;

impl Plugin for PaddlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::InGame),
            setup_paddle.run_if(in_state(AppState::Breakout)),
        )
        .add_systems(
            ReadInputs,
            read_local_inputs.run_if(in_state(AppState::Breakout)),
        )
        .add_systems(
            GgrsSchedule,
            move_paddles.run_if(in_state(AppState::Breakout)),
        );
    }
}

#[derive(Component)]
pub struct Paddle {
    pub half_size: Vec2,
}

fn setup_paddle(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let half_size = Vec2::new(PADDLE_WIDTH / 2.0, PADDLE_HEIGHT / 2.0);
    commands
        .spawn((
            Paddle { half_size },
            Team(0),
            Mesh2d(meshes.add(Rectangle::new(PADDLE_WIDTH, PADDLE_HEIGHT))),
            MeshMaterial2d(materials.add(Color::WHITE)),
            Transform::from_xyz(0.0, -PADDLE_Y_POSITION, 7.0),
        ))
        .add_rollback();
    commands
        .spawn((
            Paddle { half_size },
            Team(1),
            Mesh2d(meshes.add(Rectangle::new(PADDLE_WIDTH, PADDLE_HEIGHT))),
            MeshMaterial2d(materials.add(Color::WHITE)),
            Transform::from_xyz(0.0, PADDLE_Y_POSITION, 7.0),
        ))
        .add_rollback();
}

fn read_local_inputs(
    mut commands: Commands,
    people: Res<PeopleDataRes>,
    local_players: Res<LocalPlayers>,
) {
    let mut local_inputs = HashMap::new();

    for handle in &local_players.0 {
        let input = get_right_hand_pos(&people).map(|p| p[0]).unwrap_or(0.) as f32;
        local_inputs.insert(*handle, input);
    }

    commands.insert_resource(LocalInputs::<Config>(local_inputs));
}

fn get_right_hand_pos(people: &PeopleDataRes) -> Option<[f64; 2]> {
    *people.first()?.keypoints.get(10)?
}

pub fn move_paddles(
    inputs: Res<PlayerInputs<Config>>,
    query: Query<(&Paddle, &Team, &mut Transform)>,
    query_walls: Query<(&Wall, &Transform), (With<SideWall>, Without<Paddle>)>,
) {
    for (paddle, team, mut paddle_transform) in query {
        let (mut input, _) = inputs[team.0];

        input -= 0.5;
        input *= -2. * FIELD_WIDTH as f32 * CELL_SIZE;

        // Check wall collision
        for (wall, wall_transform) in query_walls.iter() {
            let paddle_aabb = Aabb2d::new(
                paddle_transform.translation.truncate().with_x(input),
                paddle.half_size,
            );
            let wall_aabb = Aabb2d::new(wall_transform.translation.truncate(), wall.half_size);

            if wall_transform.translation.x.abs() < input.abs()
                || paddle_aabb.intersects(&wall_aabb)
            {
                input = if wall_transform.translation.x.is_sign_positive() {
                    wall_aabb.min.x - paddle.half_size.x
                } else {
                    wall_aabb.max.x + paddle.half_size.x
                };
            }
        }

        paddle_transform.translation.x = input;
    }
}
