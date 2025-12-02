use bevy::prelude::*;

use crate::{
    breakout::field::{CELL_SIZE, FIELD_WIDTH},
    pose::PeopleDataRes,
};

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, show_right_hand);
    }
}

fn show_right_hand(people: Res<PeopleDataRes>, mut gizmos: Gizmos) {
    let Some(mut pos) = get_right_hand_pos(&people) else {
        return;
    };

    pos[0] -= 0.5;
    pos[0] *= -2. * FIELD_WIDTH as f64 * CELL_SIZE as f64;
    pos[1] *= -2. * FIELD_WIDTH as f64 * CELL_SIZE as f64;
    println!("{pos:?}");
    gizmos.circle_2d(Vec2::new(pos[0] as f32, pos[1] as f32), 10.0, Color::BLACK);
}

fn get_right_hand_pos(people: &PeopleDataRes) -> Option<[f64; 2]> {
    *people.first()?.keypoints.get(10)?
}
