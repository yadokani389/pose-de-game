use bevy::prelude::*;

use super::game::{HandPreference, HandSelection};

pub fn handle_hand_toggle(input: Res<ButtonInput<KeyCode>>, mut selection: ResMut<HandSelection>) {
    if input.just_pressed(KeyCode::ArrowLeft) {
        selection.preference = match selection.preference {
            HandPreference::Right => HandPreference::Both,
            HandPreference::Both => HandPreference::Left,
            HandPreference::Left => HandPreference::Right,
        };
    }
    if input.just_pressed(KeyCode::ArrowRight) {
        selection.preference = match selection.preference {
            HandPreference::Left => HandPreference::Both,
            HandPreference::Both => HandPreference::Right,
            HandPreference::Right => HandPreference::Left,
        };
    }
}
