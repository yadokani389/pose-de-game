use bevy::prelude::*;
use std::collections::VecDeque;

use super::{
    game::{HandPreference, HandSelection},
    game_world_size, map_pose_to_world,
};
use crate::pose::{LatestFrameRes, PeopleDataRes};

const HAND_TRAIL_DURATION: f32 = 0.3;
const VELOCITY_SAMPLES: usize = 3;
const LEFT_HAND_KEYPOINT: usize = 9;
const RIGHT_HAND_KEYPOINT: usize = 10;

#[derive(Resource, Default)]
pub struct HandTrackers {
    pub left_trail: VecDeque<(Vec2, f32)>,
    pub right_trail: VecDeque<(Vec2, f32)>,
    left_velocity_samples: [Vec2; VELOCITY_SAMPLES],
    right_velocity_samples: [Vec2; VELOCITY_SAMPLES],
    left_sample_index: usize,
    right_sample_index: usize,
    left_sample_count: usize,
    right_sample_count: usize,
    left_prev_pos: Option<Vec2>,
    right_prev_pos: Option<Vec2>,
}

impl HandTrackers {
    pub fn left_velocity(&self) -> Option<Vec2> {
        if self.left_sample_count == 0 {
            return None;
        }

        let mut sum = Vec2::ZERO;
        for i in 0..self.left_sample_count {
            sum += self.left_velocity_samples[i];
        }
        Some(sum / self.left_sample_count as f32)
    }

    pub fn right_velocity(&self) -> Option<Vec2> {
        if self.right_sample_count == 0 {
            return None;
        }

        let mut sum = Vec2::ZERO;
        for i in 0..self.right_sample_count {
            sum += self.right_velocity_samples[i];
        }
        Some(sum / self.right_sample_count as f32)
    }

    fn update_hand(&mut self, is_left: bool, position: Vec2, timestamp: f32, dt: f32) {
        let (trail, velocity_samples, sample_index, sample_count, prev_pos) = if is_left {
            (
                &mut self.left_trail,
                &mut self.left_velocity_samples,
                &mut self.left_sample_index,
                &mut self.left_sample_count,
                &mut self.left_prev_pos,
            )
        } else {
            (
                &mut self.right_trail,
                &mut self.right_velocity_samples,
                &mut self.right_sample_index,
                &mut self.right_sample_count,
                &mut self.right_prev_pos,
            )
        };

        trail.push_back((position, timestamp));

        while let Some(&(_, t)) = trail.front() {
            if timestamp - t > HAND_TRAIL_DURATION {
                trail.pop_front();
            } else {
                break;
            }
        }

        // Calculate velocity
        if let Some(prev) = *prev_pos {
            let velocity = if dt > 0.0 {
                (position - prev) / dt
            } else {
                Vec2::ZERO
            };

            velocity_samples[*sample_index] = velocity;
            *sample_index = (*sample_index + 1) % VELOCITY_SAMPLES;
            if *sample_count < VELOCITY_SAMPLES {
                *sample_count += 1;
            }
        }

        *prev_pos = Some(position);
    }

    fn clear_hand(&mut self, is_left: bool) {
        if is_left {
            self.left_trail.clear();
            self.left_velocity_samples = [Vec2::ZERO; VELOCITY_SAMPLES];
            self.left_sample_index = 0;
            self.left_sample_count = 0;
            self.left_prev_pos = None;
        } else {
            self.right_trail.clear();
            self.right_velocity_samples = [Vec2::ZERO; VELOCITY_SAMPLES];
            self.right_sample_index = 0;
            self.right_sample_count = 0;
            self.right_prev_pos = None;
        }
    }
}

pub fn update_hand_trackers(
    mut trackers: ResMut<HandTrackers>,
    people: Res<PeopleDataRes>,
    latest_frame: Res<LatestFrameRes>,
    time: Res<Time>,
    window: Single<&Window>,
    hand_selection: Res<HandSelection>,
) {
    let mapped_size = game_world_size(&window);

    let timestamp = time.elapsed_secs();
    let dt = time.delta_secs();

    let person = people.iter().next();

    if let Some(person) = person {
        if hand_selection.preference == HandPreference::Left
            || hand_selection.preference == HandPreference::Both
        {
            if let Some(Some(left_hand)) = person.keypoints.get(LEFT_HAND_KEYPOINT) {
                let position = map_pose_to_world(
                    Vec2::new(left_hand[0] as f32, left_hand[1] as f32),
                    mapped_size,
                );
                trackers.update_hand(true, position, timestamp, dt);
            } else {
                trackers.clear_hand(true);
            }
        } else {
            trackers.clear_hand(true);
        }

        if hand_selection.preference == HandPreference::Right
            || hand_selection.preference == HandPreference::Both
        {
            if let Some(Some(right_hand)) = person.keypoints.get(RIGHT_HAND_KEYPOINT) {
                let position = map_pose_to_world(
                    Vec2::new(right_hand[0] as f32, right_hand[1] as f32),
                    mapped_size,
                );
                trackers.update_hand(false, position, timestamp, dt);
            } else {
                trackers.clear_hand(false);
            }
        } else {
            trackers.clear_hand(false);
        }
    } else {
        trackers.clear_hand(true);
        trackers.clear_hand(false);
    }
}
