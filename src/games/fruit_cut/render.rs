use bevy::prelude::*;

use super::game::FruitCutPhase;
use super::hand_tracker::HandTrackers;

const HAND_TRAIL_THICKNESS: f32 = 33.75;

pub fn render_hand_trails(
    mut gizmos: Gizmos,
    hand_trackers: Res<HandTrackers>,
    phase: Res<FruitCutPhase>,
) {
    if *phase != FruitCutPhase::Playing {
        return;
    }

    for hand_trail in &hand_trackers.hands {
        if hand_trail.trail.len() >= 2 {
            for i in 0..hand_trail.trail.len() - 1 {
                let p1 = hand_trail.trail[i].0;
                let p2 = hand_trail.trail[i + 1].0;

                let num_lines = 5;
                let half = num_lines / 2;

                for offset in -half..=half {
                    let offset_f = offset as f32 * (HAND_TRAIL_THICKNESS / num_lines as f32);
                    let offset_vec = Vec2::new(offset_f, 0.0);
                    gizmos.line_2d(p1 + offset_vec, p2 + offset_vec, hand_trail.color);
                }
                for offset in -half..=half {
                    let offset_f = offset as f32 * (HAND_TRAIL_THICKNESS / num_lines as f32);
                    let offset_vec = Vec2::new(0.0, offset_f);
                    gizmos.line_2d(p1 + offset_vec, p2 + offset_vec, hand_trail.color);
                }
            }
        }
    }
}
