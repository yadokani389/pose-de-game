use bevy::prelude::*;
use std::collections::VecDeque;

use super::{game_world_size, hand_tracker::HandTrackers};

// Hand preference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandPreference {
    Left,
    Right,
    Both,
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct HandSelection {
    pub preference: HandPreference,
}

impl Default for HandSelection {
    fn default() -> Self {
        Self {
            preference: HandPreference::Both,
        }
    }
}

// Constants
pub const GAME_DURATION: f32 = 60.0;
pub const GRAVITY: f32 = 600.0;
const SLICE_MIN_VELOCITY: f32 = 400.0;

// Game phases
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

// Scoreboard
#[derive(Resource, Default)]
pub struct Scoreboard {
    pub score: u32,
    pub total_sliced: u32,
    pub total_missed: u32,
    pub bombs_hit: u32,
}

// Combo state
#[derive(Resource, Default)]
pub struct ComboState {
    pub current_combo: u32,
    pub max_combo: u32,
    pub last_slice_time: f32,
}

impl ComboState {
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

// Game timer
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

// Fruit types
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

// Components
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

// Fruit spawner
#[derive(Resource)]
pub struct FruitSpawner {
    pub timer: Timer,
    pub rng_state: u64,
}

impl Default for FruitSpawner {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.8, TimerMode::Repeating),
            rng_state: 0x853c_49e6_748f_ea9b,
        }
    }
}

// Simple LCG for random numbers
fn lcg(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    *state
}

fn random_f32(state: &mut u64) -> f32 {
    (lcg(state) as f32) / (u64::MAX as f32)
}

// Systems
pub fn spawn_fruits(
    mut commands: Commands,
    time: Res<Time>,
    mut spawner: ResMut<FruitSpawner>,
    game_timer: Res<GameTimer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    window: Single<&Window>,
) {
    spawner.timer.tick(time.delta());

    // Adjust spawn rate based on time
    let interval = if game_timer.elapsed < 20.0 {
        0.8
    } else if game_timer.elapsed < 40.0 {
        0.65
    } else {
        0.5
    };

    if spawner.timer.duration().as_secs_f32() != interval {
        spawner.timer = Timer::from_seconds(interval, TimerMode::Repeating);
    }

    if !spawner.timer.just_finished() {
        return;
    }

    // Determine if bomb or fruit
    let bomb_chance = if game_timer.elapsed < 30.0 {
        0.15
    } else {
        0.25
    };
    let is_bomb = random_f32(&mut spawner.rng_state) < bomb_chance;

    // Calculate game world size (same as in setup)
    let game_size = game_world_size(&window);

    // Random spawn position
    let x = (random_f32(&mut spawner.rng_state) - 0.5) * game_size.x * 0.8;
    let y = game_size.y * 0.5 + 100.0;

    // Random initial velocity
    let vx = (random_f32(&mut spawner.rng_state) - 0.5) * 200.0;
    let vy = -200.0 - random_f32(&mut spawner.rng_state) * 200.0;
    let velocity = Vec2::new(vx, vy);

    // Random angular velocity
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
    mut fruit_query: Query<(Entity, &mut Fruit, &mut Transform)>,
    mut bomb_query: Query<(Entity, &mut Bomb, &mut Transform), Without<Fruit>>,
    mut scoreboard: ResMut<Scoreboard>,
    mut combo: ResMut<ComboState>,
    window: Single<&Window>,
) {
    let dt = time.delta_secs();

    // Calculate game world size
    let game_size = game_world_size(&window);
    let despawn_y = -game_size.y * 0.5 - 100.0;

    // Update fruits
    for (entity, mut fruit, mut transform) in fruit_query.iter_mut() {
        fruit.velocity.y -= GRAVITY * dt;
        transform.translation.x += fruit.velocity.x * dt;
        transform.translation.y += fruit.velocity.y * dt;
        transform.rotation *= Quat::from_rotation_z(fruit.angular_velocity * dt);

        // Despawn if off screen
        if transform.translation.y < despawn_y {
            commands.entity(entity).despawn();
            scoreboard.total_missed += 1;
            combo.reset();
        }
    }

    // Update bombs
    for (entity, mut bomb, mut transform) in bomb_query.iter_mut() {
        bomb.velocity.y -= GRAVITY * dt;
        transform.translation.x += bomb.velocity.x * dt;
        transform.translation.y += bomb.velocity.y * dt;
        transform.rotation *= Quat::from_rotation_z(bomb.angular_velocity * dt);

        // Despawn if off screen
        if transform.translation.y < despawn_y {
            commands.entity(entity).despawn();
        }
    }
}

pub fn check_fruit_slicing(
    mut commands: Commands,
    hand_trackers: Res<HandTrackers>,
    fruit_query: Query<(Entity, &Fruit, &Transform)>,
    bomb_query: Query<(Entity, &Transform), With<Bomb>>,
    mut scoreboard: ResMut<Scoreboard>,
    mut combo: ResMut<ComboState>,
    time: Res<Time>,
) {
    let elapsed = time.elapsed_secs();

    for (entity, fruit, transform) in fruit_query.iter() {
        let fruit_pos = transform.translation.truncate();
        let radius = fruit.fruit_type.radius();

        let mut sliced = false;

        if let Some(velocity) = hand_trackers.left_velocity() {
            if velocity.length() >= SLICE_MIN_VELOCITY {
                if check_trail_intersection(&hand_trackers.left_trail, fruit_pos, radius) {
                    sliced = true;
                }
            }
        }

        if !sliced {
            if let Some(velocity) = hand_trackers.right_velocity() {
                if velocity.length() >= SLICE_MIN_VELOCITY {
                    if check_trail_intersection(&hand_trackers.right_trail, fruit_pos, radius) {
                        sliced = true;
                    }
                }
            }
        }

        if sliced {
            let base_score = fruit.fruit_type.score();
            let multiplier = combo.get_multiplier();
            let final_score = (base_score as f32 * multiplier) as u32;

            scoreboard.score += final_score;
            scoreboard.total_sliced += 1;
            combo.increment(elapsed);

            commands.entity(entity).despawn();
        }
    }

    for (entity, transform) in bomb_query.iter() {
        let bomb_pos = transform.translation.truncate();
        let radius = 35.0;

        let mut hit = false;

        if let Some(velocity) = hand_trackers.left_velocity() {
            if velocity.length() >= SLICE_MIN_VELOCITY {
                if check_trail_intersection(&hand_trackers.left_trail, bomb_pos, radius) {
                    hit = true;
                }
            }
        }

        if !hit {
            if let Some(velocity) = hand_trackers.right_velocity() {
                if velocity.length() >= SLICE_MIN_VELOCITY {
                    if check_trail_intersection(&hand_trackers.right_trail, bomb_pos, radius) {
                        hit = true;
                    }
                }
            }
        }

        if hit {
            scoreboard.score = scoreboard.score.saturating_sub(50);
            scoreboard.bombs_hit += 1;
            combo.reset();
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
) {
    game_timer.timer.tick(time.delta());
    game_timer.elapsed = game_timer.timer.elapsed_secs();

    if game_timer.timer.just_finished() {
        *phase = FruitCutPhase::Result;
        commands.insert_resource(GameResult {
            final_score: scoreboard.score,
            total_sliced: scoreboard.total_sliced,
            total_missed: scoreboard.total_missed,
            bombs_hit: scoreboard.bombs_hit,
            max_combo: combo.max_combo,
        });
    }
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct GameResult {
    pub final_score: u32,
    pub total_sliced: u32,
    pub total_missed: u32,
    pub bombs_hit: u32,
    pub max_combo: u32,
}
