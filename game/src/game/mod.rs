use bevy::camera::ScalingMode;
use bevy::prelude::*;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(())
            .init_resource::<PeopleDataResource>()
            .init_resource::<UdpBuffer>()
            .add_systems(Startup, (setup_graphics, setup_figures))
            .add_systems(Update, (receive_data, update_figures));
    }
}

fn setup_graphics(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 1000.,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}

fn setup_figures(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for person_id in 0..4 {
        for keypoint_type in [
            KeypointType::Head,
            KeypointType::LeftHand,
            KeypointType::RightHand,
        ] {
            let circle = StickFigureCircle::new(person_id, keypoint_type);

            let (color, radius) = match keypoint_type {
                KeypointType::Head => (Color::srgb(1.0, 1.0, 0.0), 80.0), // Yellow head
                KeypointType::LeftHand => (Color::srgb(1.0, 0.0, 0.0), 20.0), // Red left hand
                KeypointType::RightHand => (Color::srgb(0.0, 0.0, 1.0), 20.0), // Blue right hand
            };

            commands.spawn((
                Mesh2d(meshes.add(Circle::new(radius))),
                MeshMaterial2d(materials.add(ColorMaterial::from(color))),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                Visibility::Hidden, // Hide initially until we have data
                circle,
            ));
        }
    }
}

fn receive_data(
    socket: Res<super::UdpSocketResource>,
    mut people_data: ResMut<PeopleDataResource>,
    mut buffer: ResMut<UdpBuffer>,
) {
    let Ok(size) = socket.0.recv(&mut buffer.0) else {
        return;
    };

    match serde_cbor::from_slice::<PeopleData>(&buffer.0[..size]) {
        Ok(people) => {
            people_data.0 = people;
        }
        Err(e) => {
            error!("Failed to parse CBOR data: {e}");
        }
    }
}

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct PersonData {
    keypoints: Vec<Option<[f64; 2]>>,
    right_hand_closed: Option<bool>,
    left_hand_closed: Option<bool>,
}

type PeopleData = Vec<PersonData>;
#[derive(Resource, Default)]
struct PeopleDataResource(PeopleData);

#[derive(Resource)]
struct UdpBuffer(Vec<u8>);

impl Default for UdpBuffer {
    fn default() -> Self {
        Self(vec![0; 65536])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum KeypointType {
    Head,
    LeftHand,
    RightHand,
}

#[derive(Component)]
struct StickFigureCircle {
    person_id: usize,
    keypoint_indices: Vec<usize>,
}

impl StickFigureCircle {
    fn new(person_id: usize, keypoint_type: KeypointType) -> Self {
        let keypoint_indices = match keypoint_type {
            KeypointType::Head => vec![0, 1, 2, 3, 4], // nose only
            KeypointType::LeftHand => vec![9],         // left wrist
            KeypointType::RightHand => vec![10],       // right wrist
        };

        Self {
            person_id,
            keypoint_indices,
        }
    }

    fn calculate_average_position(&self, keypoints: &[Option<[f64; 2]>]) -> Option<Vec2> {
        let mut valid_points = Vec::new();

        for &idx in &self.keypoint_indices {
            if let Some(Some(point)) = keypoints.get(idx) {
                valid_points.push(*point);
            }
        }

        if valid_points.is_empty() {
            return None;
        }

        let sum_x: f64 = valid_points.iter().map(|p| p[0]).sum();
        let sum_y: f64 = valid_points.iter().map(|p| p[1]).sum();
        let count = valid_points.len() as f64;

        let avg_point = [sum_x / count, sum_y / count];
        Some(normalize_to_screen_coords(avg_point))
    }
}

fn normalize_to_screen_coords(keypoint: [f64; 2]) -> Vec2 {
    Vec2::new(
        (keypoint[0] as f32 - 0.5) * 1000.0,
        (0.5 - keypoint[1] as f32) * 1000.0,
    )
}

fn update_figures(
    people_data: Res<PeopleDataResource>,
    mut circles: Query<(&mut Transform, &mut Visibility, &StickFigureCircle)>,
) {
    if !people_data.is_changed() {
        return;
    }

    for (mut transform, mut visibility, circle) in circles.iter_mut() {
        if let Some(person) = people_data.0.get(circle.person_id) {
            let keypoints = &person.keypoints;
            if 17 <= keypoints.len()
                && let Some(new_position) = circle.calculate_average_position(keypoints)
            {
                transform.translation =
                    0.5 * (transform.translation + Vec3::new(new_position.x, new_position.y, 0.0));
                *visibility = Visibility::Visible;
            }
        }
    }
}
