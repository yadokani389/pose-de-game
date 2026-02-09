use bevy::prelude::*;
use std::collections::VecDeque;

use super::{game::FruitCutSettings, game::PlayerSide, game_world_size, map_pose_to_world};
use crate::pose::PeopleDataRes;

const HAND_TRAIL_DURATION: f32 = 0.3;
const VELOCITY_SAMPLES: usize = 3;
const LEFT_HAND_KEYPOINT: usize = 9;
const RIGHT_HAND_KEYPOINT: usize = 10;
const HAND_TIMEOUT: f32 = 0.5;
const HAND_MATCH_DISTANCE: f32 = 100.0;

#[derive(Debug, Clone)]
pub struct HandTrail {
    pub trail: VecDeque<(Vec2, f32)>,
    pub color: Color,
    pub owner: PlayerSide,
    velocity_samples: [Vec2; VELOCITY_SAMPLES],
    sample_index: usize,
    sample_count: usize,
    prev_pos: Option<Vec2>,
    last_update: f32,
}

impl HandTrail {
    fn new(color: Color, owner: PlayerSide, timestamp: f32) -> Self {
        Self {
            trail: VecDeque::new(),
            color,
            owner,
            velocity_samples: [Vec2::ZERO; VELOCITY_SAMPLES],
            sample_index: 0,
            sample_count: 0,
            prev_pos: None,
            last_update: timestamp,
        }
    }

    pub fn velocity(&self) -> Option<Vec2> {
        if self.sample_count == 0 {
            return None;
        }

        let mut sum = Vec2::ZERO;
        for i in 0..self.sample_count {
            sum += self.velocity_samples[i];
        }
        Some(sum / self.sample_count as f32)
    }

    fn update(&mut self, position: Vec2, timestamp: f32, dt: f32) {
        self.trail.push_back((position, timestamp));
        self.last_update = timestamp;

        while let Some(&(_, t)) = self.trail.front() {
            if timestamp - t > HAND_TRAIL_DURATION {
                self.trail.pop_front();
            } else {
                break;
            }
        }

        if let Some(prev) = self.prev_pos {
            let velocity = if dt > 0.0 {
                (position - prev) / dt
            } else {
                Vec2::ZERO
            };

            self.velocity_samples[self.sample_index] = velocity;
            self.sample_index = (self.sample_index + 1) % VELOCITY_SAMPLES;
            if self.sample_count < VELOCITY_SAMPLES {
                self.sample_count += 1;
            }
        }

        self.prev_pos = Some(position);
    }
}

#[derive(Resource)]
pub struct HandTrackers {
    pub hands: Vec<HandTrail>,
    pub left_player_id: Option<u64>,
    pub right_player_id: Option<u64>,
}

impl Default for HandTrackers {
    fn default() -> Self {
        Self {
            hands: Vec::new(),
            left_player_id: None,
            right_player_id: None,
        }
    }
}

pub fn update_hand_trackers(
    mut trackers: ResMut<HandTrackers>,
    people: Res<PeopleDataRes>,
    settings: Res<FruitCutSettings>,
    time: Res<Time>,
    window: Single<&Window>,
) {
    let mapped_size = game_world_size(&window);
    let timestamp = time.elapsed_secs();
    let dt = time.delta_secs();

    let left_color = Color::srgba(0.3, 0.6, 1.0, 0.6);
    let right_color = Color::srgba(1.0, 0.3, 0.4, 0.6);

    struct DetectedHand {
        position: Vec2,
        owner: PlayerSide,
    }

    let mut detected_hands = Vec::new();

    if settings.player_count == 1 {
        for person in people.iter() {
            if let Some(Some(left_hand)) = person.keypoints.get(LEFT_HAND_KEYPOINT) {
                let position = map_pose_to_world(
                    Vec2::new(left_hand[0] as f32, left_hand[1] as f32),
                    mapped_size,
                );
                detected_hands.push(DetectedHand {
                    position,
                    owner: PlayerSide::Left,
                });
            }

            if let Some(Some(right_hand)) = person.keypoints.get(RIGHT_HAND_KEYPOINT) {
                let position = map_pose_to_world(
                    Vec2::new(right_hand[0] as f32, right_hand[1] as f32),
                    mapped_size,
                );
                detected_hands.push(DetectedHand {
                    position,
                    owner: PlayerSide::Left,
                });
            }
        }
    } else {
        if trackers.left_player_id.is_none() || trackers.right_player_id.is_none() {
            let mut people_with_centers: Vec<(u64, f32)> = Vec::new();

            for person in people.iter() {
                let center_x = estimate_person_center(&person.keypoints);
                if let Some(center) = center_x {
                    people_with_centers.push((person.id, center));
                }
            }

            people_with_centers.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            if let Some((id, _)) = people_with_centers.get(0) {
                trackers.left_player_id = Some(*id);
            }
            if let Some((id, _)) = people_with_centers.get(1) {
                trackers.right_player_id = Some(*id);
            }
        }

        for person in people.iter() {
            let owner = if Some(person.id) == trackers.left_player_id {
                PlayerSide::Left
            } else if Some(person.id) == trackers.right_player_id {
                PlayerSide::Right
            } else {
                continue;
            };

            if let Some(Some(left_hand)) = person.keypoints.get(LEFT_HAND_KEYPOINT) {
                let position = map_pose_to_world(
                    Vec2::new(left_hand[0] as f32, left_hand[1] as f32),
                    mapped_size,
                );
                detected_hands.push(DetectedHand { position, owner });
            }

            if let Some(Some(right_hand)) = person.keypoints.get(RIGHT_HAND_KEYPOINT) {
                let position = map_pose_to_world(
                    Vec2::new(right_hand[0] as f32, right_hand[1] as f32),
                    mapped_size,
                );
                detected_hands.push(DetectedHand { position, owner });
            }
        }
    }

    let mut matched = vec![false; detected_hands.len()];

    for hand_trail in &mut trackers.hands {
        if let Some(last_pos) = hand_trail.prev_pos {
            let mut best_match = None;
            let mut best_distance = HAND_MATCH_DISTANCE;

            for (i, detected) in detected_hands.iter().enumerate() {
                if !matched[i] && detected.owner == hand_trail.owner {
                    let distance = last_pos.distance(detected.position);
                    if distance < best_distance {
                        best_distance = distance;
                        best_match = Some(i);
                    }
                }
            }

            if let Some(match_idx) = best_match {
                matched[match_idx] = true;
                hand_trail.update(detected_hands[match_idx].position, timestamp, dt);
            }
        }
    }

    for (i, detected) in detected_hands.iter().enumerate() {
        if !matched[i] {
            let color = match detected.owner {
                PlayerSide::Left => left_color,
                PlayerSide::Right => right_color,
            };
            let mut trail = HandTrail::new(color, detected.owner, timestamp);
            trail.update(detected.position, timestamp, dt);
            trackers.hands.push(trail);
        }
    }

    trackers
        .hands
        .retain(|hand| timestamp - hand.last_update < HAND_TIMEOUT);
}

fn estimate_person_center(keypoints: &[Option<[f64; 2]>]) -> Option<f32> {
    const LEFT_SHOULDER: usize = 5;
    const RIGHT_SHOULDER: usize = 6;
    const LEFT_HIP: usize = 11;
    const RIGHT_HIP: usize = 12;

    if let (Some(Some(left)), Some(Some(right))) =
        (keypoints.get(LEFT_SHOULDER), keypoints.get(RIGHT_SHOULDER))
    {
        return Some(((left[0] + right[0]) / 2.0) as f32);
    }

    if let (Some(Some(left)), Some(Some(right))) =
        (keypoints.get(LEFT_HIP), keypoints.get(RIGHT_HIP))
    {
        return Some(((left[0] + right[0]) / 2.0) as f32);
    }

    None
}
