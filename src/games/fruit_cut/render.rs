use bevy::prelude::*;

use super::game::FruitCutPhase;
use super::hand_tracker::HandTrackers;

pub fn render_hand_trails(
    mut gizmos: Gizmos,
    hand_trackers: Res<HandTrackers>,
    phase: Res<FruitCutPhase>,
) {
    if *phase != FruitCutPhase::Playing {
        return;
    }

    if hand_trackers.left_trail.len() >= 2 {
        for i in 0..hand_trackers.left_trail.len() - 1 {
            let p1 = hand_trackers.left_trail[i].0;
            let p2 = hand_trackers.left_trail[i + 1].0;

            for offset in -2..=2 {
                let offset_vec = Vec2::new(offset as f32 * 4.0, 0.0);
                gizmos.line_2d(
                    p1 + offset_vec,
                    p2 + offset_vec,
                    Color::srgba(0.3, 0.6, 1.0, 0.6),
                );
            }
            for offset in -2..=2 {
                let offset_vec = Vec2::new(0.0, offset as f32 * 4.0);
                gizmos.line_2d(
                    p1 + offset_vec,
                    p2 + offset_vec,
                    Color::srgba(0.3, 0.6, 1.0, 0.6),
                );
            }
        }
    }

    if hand_trackers.right_trail.len() >= 2 {
        for i in 0..hand_trackers.right_trail.len() - 1 {
            let p1 = hand_trackers.right_trail[i].0;
            let p2 = hand_trackers.right_trail[i + 1].0;

            for offset in -2..=2 {
                let offset_vec = Vec2::new(offset as f32 * 4.0, 0.0);
                gizmos.line_2d(
                    p1 + offset_vec,
                    p2 + offset_vec,
                    Color::srgba(1.0, 0.3, 0.4, 0.6),
                );
            }
            for offset in -2..=2 {
                let offset_vec = Vec2::new(0.0, offset as f32 * 4.0);
                gizmos.line_2d(
                    p1 + offset_vec,
                    p2 + offset_vec,
                    Color::srgba(1.0, 0.3, 0.4, 0.6),
                );
            }
        }
    }
}
