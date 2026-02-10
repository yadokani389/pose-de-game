use std::collections::VecDeque;

use bevy::prelude::*;

use crate::utils::spawn_floating_text_popup;

use super::{game_world_size, hand_tracker::HandTrackers};

pub const GAME_DURATION: f32 = 60.0;
pub const GRAVITY: f32 = 600.0;
const SLICE_MIN_VELOCITY: f32 = 0.0;
pub const MIN_PLAYERS: usize = 1;
pub const MAX_PLAYERS: usize = 2;

#[derive(Component, Copy, Clone, Eq, PartialEq, Debug)]
pub enum PlayerSide {
    Left,
    Right,
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct FruitCutSettings {
    pub player_count: usize,
}

impl Default for FruitCutSettings {
    fn default() -> Self {
        Self {
            player_count: MIN_PLAYERS,
        }
    }
}

#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum FruitCutPhase {
    #[default]
    Setup,
    Playing,
    Result,
}

pub fn is_playing(phase: Res<FruitCutPhase>) -> bool {
    *phase == FruitCutPhase::Playing
}

pub fn is_result(phase: Res<FruitCutPhase>) -> bool {
    *phase == FruitCutPhase::Result
}

#[derive(Resource, Default)]
pub struct Scoreboard {
    pub left_score: u32,
    pub right_score: u32,
    pub left_total_sliced: u32,
    pub right_total_sliced: u32,
    pub left_total_missed: u32,
    pub right_total_missed: u32,
    pub left_bombs_hit: u32,
    pub right_bombs_hit: u32,
}

#[derive(Clone, Copy, Default)]
pub struct PlayerCombo {
    pub current_combo: u32,
    pub max_combo: u32,
    pub last_slice_time: f32,
}

impl PlayerCombo {
    pub fn get_multiplier(&self) -> f32 {
        match self.current_combo {
            0..=4 => 1.0,
            5..=9 => 1.5,
            10..=14 => 2.0,
            _ => 2.5,
        }
    }

    pub fn increment(&mut self, time: f32) {
        self.current_combo += 1;
        self.last_slice_time = time;
        if self.current_combo > self.max_combo {
            self.max_combo = self.current_combo;
        }
    }

    pub fn reset(&mut self) {
        self.current_combo = 0;
    }
}

#[derive(Resource, Default)]
pub struct ComboState {
    pub left: PlayerCombo,
    pub right: PlayerCombo,
}

impl ComboState {
    pub fn get_mut(&mut self, side: PlayerSide) -> &mut PlayerCombo {
        match side {
            PlayerSide::Left => &mut self.left,
            PlayerSide::Right => &mut self.right,
        }
    }
}

#[derive(Resource)]
pub struct GameTimer {
    pub elapsed: f32,
    pub timer: Timer,
}

impl Default for GameTimer {
    fn default() -> Self {
        Self {
            elapsed: 0.0,
            timer: Timer::from_seconds(GAME_DURATION, TimerMode::Once),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FruitType {
    Apple,
    Orange,
    Watermelon,
    Banana,
}

impl FruitType {
    pub fn radius(&self) -> f32 {
        match self {
            FruitType::Apple => 50.0,
            FruitType::Orange => 48.0,
            FruitType::Watermelon => 75.0,
            FruitType::Banana => 40.0,
        }
    }

    pub fn score(&self) -> u32 {
        match self {
            FruitType::Apple => 10,
            FruitType::Orange => 10,
            FruitType::Watermelon => 15,
            FruitType::Banana => 20,
        }
    }

    pub fn color(&self) -> Color {
        match self {
            FruitType::Apple => Color::srgb(0.9, 0.2, 0.2),
            FruitType::Orange => Color::srgb(1.0, 0.6, 0.1),
            FruitType::Watermelon => Color::srgb(0.2, 0.8, 0.3),
            FruitType::Banana => Color::srgb(1.0, 0.9, 0.2),
        }
    }

    fn random(rng_state: &mut u64) -> Self {
        let r = lcg(rng_state) % 100;
        if r < 25 {
            FruitType::Apple
        } else if r < 50 {
            FruitType::Orange
        } else if r < 70 {
            FruitType::Watermelon
        } else {
            FruitType::Banana
        }
    }
}

#[derive(Component)]
pub struct Fruit {
    pub fruit_type: FruitType,
    pub velocity: Vec2,
    pub angular_velocity: f32,
}

#[derive(Component)]
pub struct Bomb {
    pub velocity: Vec2,
    pub angular_velocity: f32,
}

#[derive(Component)]
pub struct FruitCutEntity;

#[derive(Resource)]
pub struct FruitSpawner {
    pub timer: Timer,
    pub rng_state: u64,
    pub next_spawn_side: PlayerSide,
}

impl Default for FruitSpawner {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.8, TimerMode::Repeating),
            rng_state: 0x853c_49e6_748f_ea9b,
            next_spawn_side: PlayerSide::Left,
        }
    }
}

fn lcg(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    *state
}

fn random_f32(state: &mut u64) -> f32 {
    (lcg(state) as f32) / (u64::MAX as f32)
}

pub fn spawn_fruits(
    mut commands: Commands,
    time: Res<Time>,
    mut spawner: ResMut<FruitSpawner>,
    game_timer: Res<GameTimer>,
    settings: Res<FruitCutSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    window: Single<&Window>,
) {
    spawner.timer.tick(time.delta());

    let base_interval = if game_timer.elapsed < 20.0 {
        0.8
    } else if game_timer.elapsed < 40.0 {
        0.65
    } else {
        0.5
    };
    let interval = if settings.player_count == 2 {
        base_interval * 0.6
    } else {
        base_interval
    };
    let current_duration = spawner.timer.duration().as_secs_f32();
    if (current_duration - interval).abs() > 0.001 {
        spawner
            .timer
            .set_duration(std::time::Duration::from_secs_f32(interval));
    }

    if !spawner.timer.just_finished() {
        return;
    }

    let bomb_chance = if game_timer.elapsed < 30.0 {
        0.15
    } else {
        0.25
    };
    let is_bomb = random_f32(&mut spawner.rng_state) < bomb_chance;

    let game_size = game_world_size(&window);

    let x = if settings.player_count == 1 {
        let x_min = -game_size.x * 0.4;
        let x_max = game_size.x * 0.4;
        x_min + random_f32(&mut spawner.rng_state) * (x_max - x_min)
    } else {
        let (x_min, x_max) = match spawner.next_spawn_side {
            PlayerSide::Left => (-game_size.x * 0.45, -game_size.x * 0.05),
            PlayerSide::Right => (game_size.x * 0.05, game_size.x * 0.45),
        };
        spawner.next_spawn_side = match spawner.next_spawn_side {
            PlayerSide::Left => PlayerSide::Right,
            PlayerSide::Right => PlayerSide::Left,
        };

        x_min + random_f32(&mut spawner.rng_state) * (x_max - x_min)
    };

    let y = game_size.y * 0.5 + 100.0;

    let vx = (random_f32(&mut spawner.rng_state) - 0.5) * 200.0;
    let vy = -200.0 - random_f32(&mut spawner.rng_state) * 200.0;
    let velocity = Vec2::new(vx, vy);

    let angular_velocity = (random_f32(&mut spawner.rng_state) - 0.5) * 4.0;

    if is_bomb {
        let radius = 45.0;
        commands.spawn((
            Bomb {
                velocity,
                angular_velocity,
            },
            FruitCutEntity,
            Mesh2d(meshes.add(Circle::new(radius))),
            MeshMaterial2d(materials.add(Color::srgb(0.15, 0.15, 0.15))),
            Transform::from_xyz(x, y, 3.0),
        ));
    } else {
        let fruit_type = FruitType::random(&mut spawner.rng_state);
        let radius = fruit_type.radius();
        let color = fruit_type.color();

        commands.spawn((
            Fruit {
                fruit_type,
                velocity,
                angular_velocity,
            },
            FruitCutEntity,
            Mesh2d(meshes.add(Circle::new(radius))),
            MeshMaterial2d(materials.add(color)),
            Transform::from_xyz(x, y, 3.0),
        ));
    }
}

pub fn update_fruits(
    mut commands: Commands,
    time: Res<Time>,
    mut fruit_query: Query<(Entity, &Fruit, &mut Transform)>,
    mut bomb_query: Query<(Entity, &Bomb, &mut Transform), Without<Fruit>>,
    mut scoreboard: ResMut<Scoreboard>,
    mut combo: ResMut<ComboState>,
    window: Single<&Window>,
) {
    let dt = time.delta_secs();

    let game_size = game_world_size(&window);
    let despawn_y = -game_size.y * 0.5 - 100.0;

    for (entity, fruit, mut transform) in fruit_query.iter_mut() {
        let mut velocity = fruit.velocity;
        velocity.y -= GRAVITY * dt;
        transform.translation.x += velocity.x * dt;
        transform.translation.y += velocity.y * dt;
        transform.rotation *= Quat::from_rotation_z(fruit.angular_velocity * dt);

        if transform.translation.y < despawn_y {
            let side = if transform.translation.x >= 0.0 {
                PlayerSide::Right
            } else {
                PlayerSide::Left
            };

            match side {
                PlayerSide::Left => {
                    scoreboard.left_total_missed += 1;
                    combo.left.reset();
                }
                PlayerSide::Right => {
                    scoreboard.right_total_missed += 1;
                    combo.right.reset();
                }
            }

            commands.entity(entity).despawn();
        }
    }

    for (entity, bomb, mut transform) in bomb_query.iter_mut() {
        let mut velocity = bomb.velocity;
        velocity.y -= GRAVITY * dt;
        transform.translation.x += velocity.x * dt;
        transform.translation.y += velocity.y * dt;
        transform.rotation *= Quat::from_rotation_z(bomb.angular_velocity * dt);

        if transform.translation.y < despawn_y {
            commands.entity(entity).despawn();
        }
    }
}

pub fn check_fruit_slicing(
    mut commands: Commands,
    hand_trackers: Res<HandTrackers>,
    fruit_query: Query<(Entity, &Fruit, &Transform)>,
    bomb_query: Query<(Entity, &Bomb, &Transform)>,
    mut scoreboard: ResMut<Scoreboard>,
    mut combo: ResMut<ComboState>,
    time: Res<Time>,
    settings: Res<FruitCutSettings>,
) {
    let elapsed = time.elapsed_secs();

    for (entity, fruit, transform) in fruit_query.iter() {
        let fruit_pos = transform.translation.truncate();
        let radius = fruit.fruit_type.radius();

        let mut sliced = false;
        let mut slicing_owner = None;

        let center_side = if fruit_pos.x >= 0.0 {
            PlayerSide::Right
        } else {
            PlayerSide::Left
        };

        for hand_trail in &hand_trackers.hands {
            if settings.player_count == 2 && hand_trail.owner != center_side {
                continue;
            }

            if let Some(velocity) = hand_trail.velocity()
                && velocity.length() >= SLICE_MIN_VELOCITY
                && check_trail_intersection(&hand_trail.trail, fruit_pos, radius)
            {
                sliced = true;
                slicing_owner = Some(hand_trail.owner);
                break;
            }
        }

        if sliced && let Some(owner) = slicing_owner {
            let player_combo = combo.get_mut(owner);
            let base_score = fruit.fruit_type.score();
            let multiplier = player_combo.get_multiplier();
            let final_score = (base_score as f32 * multiplier) as u32;

            match owner {
                PlayerSide::Left => {
                    scoreboard.left_score += final_score;
                    scoreboard.left_total_sliced += 1;
                }
                PlayerSide::Right => {
                    scoreboard.right_score += final_score;
                    scoreboard.right_total_sliced += 1;
                }
            }

            player_combo.increment(elapsed);
            let popup_entity = spawn_floating_text_popup(
                &mut commands,
                fruit_pos,
                format!("+{}", final_score),
                fruit.fruit_type.color(),
                None,
                64.0,
            );
            commands.entity(popup_entity).insert(FruitCutEntity);

            commands.entity(entity).despawn();
        }
    }

    for (entity, _bomb, transform) in bomb_query.iter() {
        let bomb_pos = transform.translation.truncate();
        let radius = 45.0;

        let mut hit = false;
        let mut hitting_owner = None;

        let bomb_center_side = if bomb_pos.x >= 0.0 {
            PlayerSide::Right
        } else {
            PlayerSide::Left
        };

        for hand_trail in &hand_trackers.hands {
            if settings.player_count == 2 && hand_trail.owner != bomb_center_side {
                continue;
            }

            if let Some(velocity) = hand_trail.velocity()
                && velocity.length() >= SLICE_MIN_VELOCITY
                && check_trail_intersection(&hand_trail.trail, bomb_pos, radius)
            {
                hit = true;
                hitting_owner = Some(hand_trail.owner);
                break;
            }
        }

        if hit && let Some(owner) = hitting_owner {
            match owner {
                PlayerSide::Left => {
                    scoreboard.left_score = scoreboard.left_score.saturating_sub(50);
                    scoreboard.left_bombs_hit += 1;
                    combo.left.reset();
                }
                PlayerSide::Right => {
                    scoreboard.right_score = scoreboard.right_score.saturating_sub(50);
                    scoreboard.right_bombs_hit += 1;
                    combo.right.reset();
                }
            }
            spawn_floating_text_popup(
                &mut commands,
                bomb_pos,
                "×",
                Color::srgb(1.0, 0.1, 0.1),
                None,
                128.0,
            );
            commands.entity(entity).despawn();
        }
    }
}

fn check_trail_intersection(trail: &VecDeque<(Vec2, f32)>, center: Vec2, radius: f32) -> bool {
    for i in 0..trail.len().saturating_sub(1) {
        let p1 = trail[i].0;
        let p2 = trail[i + 1].0;

        if line_circle_intersect(p1, p2, center, radius) {
            return true;
        }
    }
    false
}

fn line_circle_intersect(p1: Vec2, p2: Vec2, center: Vec2, radius: f32) -> bool {
    let d = p2 - p1;
    let f = p1 - center;

    let a = d.dot(d);
    if a < 1e-6 {
        return false;
    }

    let b = 2.0 * f.dot(d);
    let c = f.dot(f) - radius * radius;

    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return false;
    }

    let sqrt_disc = discriminant.sqrt();
    let inv = 0.5 / a;
    let t1 = (-b - sqrt_disc) * inv;
    let t2 = (-b + sqrt_disc) * inv;

    (0.0..=1.0).contains(&t1) || (0.0..=1.0).contains(&t2)
}

pub fn update_game_timer(
    mut game_timer: ResMut<GameTimer>,
    time: Res<Time>,
    mut phase: ResMut<FruitCutPhase>,
    mut commands: Commands,
    scoreboard: Res<Scoreboard>,
    combo: Res<ComboState>,
    settings: Res<FruitCutSettings>,
) {
    game_timer.timer.tick(time.delta());
    game_timer.elapsed = game_timer.timer.elapsed_secs();

    if game_timer.timer.just_finished() {
        *phase = FruitCutPhase::Result;

        let winner = if settings.player_count == 2 {
            if scoreboard.left_score > scoreboard.right_score {
                Some(PlayerSide::Left)
            } else if scoreboard.right_score > scoreboard.left_score {
                Some(PlayerSide::Right)
            } else {
                None
            }
        } else {
            None
        };

        commands.insert_resource(GameResult {
            player_count: settings.player_count,
            winner,
            left_score: scoreboard.left_score,
            left_total_sliced: scoreboard.left_total_sliced,
            left_total_missed: scoreboard.left_total_missed,
            left_bombs_hit: scoreboard.left_bombs_hit,
            left_max_combo: combo.left.max_combo,
            right_score: scoreboard.right_score,
            right_total_sliced: scoreboard.right_total_sliced,
            right_total_missed: scoreboard.right_total_missed,
            right_bombs_hit: scoreboard.right_bombs_hit,
            right_max_combo: combo.right.max_combo,
        });
    }
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct GameResult {
    pub player_count: usize,
    pub winner: Option<PlayerSide>,
    pub left_score: u32,
    pub left_total_sliced: u32,
    pub left_total_missed: u32,
    pub left_bombs_hit: u32,
    pub left_max_combo: u32,
    pub right_score: u32,
    pub right_total_sliced: u32,
    pub right_total_missed: u32,
    pub right_bombs_hit: u32,
    pub right_max_combo: u32,
}
