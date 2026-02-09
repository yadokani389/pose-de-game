use bevy::prelude::*;

use crate::{
    pose::PeopleDataRes,
    utils::pitch::{BeepPalette, play_beep},
};

use super::settings::{FlagRaisePhase, FlagRaiseSettings, GameMode, MAX_PLAYERS};

#[derive(Resource, Default)]
pub struct Scoreboard {
    pub scores: [u32; MAX_PLAYERS],
}

#[derive(Resource)]
pub struct CommandRng {
    pub state: u64,
}

impl Default for CommandRng {
    fn default() -> Self {
        Self {
            state: 0x853c_49e6_748f_ea9b,
        }
    }
}

#[derive(Resource)]
pub struct CommandState {
    pub current: CommandSpec,
    pub timer: Timer,
    pub dirty: bool,
    pub beep_state: TurnBeepState,
    pub hand_state: HandState,
    pub mismatch_color: bool,
}

pub const GAME_LIMIT_SECS: f32 = 60.0;
const TURN_MIN_RATIO: f32 = 0.75;
const TURN_MOVE_CHANCE: f32 = 0.7;
const DUAL_COMMAND_CHANCE: f32 = 0.35;
const DUAL_COMMAND_START_SECS: f32 = 15.0;
const COLOR_MISMATCH_CHANCE: f32 = 0.5;
const TURN_BEEP_FIRST: f32 = 1.0 / 3.0;
const TURN_BEEP_SECOND: f32 = 2.0 / 3.0;

#[derive(Resource)]
pub struct GameTimer {
    pub timer: Timer,
    pub elapsed: f32,
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct GameResult {
    pub reason: GameOverReason,
    pub failed_slot: Option<usize>,
}

#[derive(Clone, Copy, Debug)]
pub enum GameOverReason {
    TimeUp,
    Failed,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TurnBeepState {
    first: bool,
    second: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HandState {
    white_raised: bool,
    red_raised: bool,
}

impl HandState {
    fn is_raised(self, color: FlagColor) -> bool {
        match color {
            FlagColor::White => self.white_raised,
            FlagColor::Red => self.red_raised,
        }
    }

    fn set_raised(&mut self, color: FlagColor, raised: bool) {
        match color {
            FlagColor::White => self.white_raised = raised,
            FlagColor::Red => self.red_raised = raised,
        }
    }
}

pub fn start_game(
    commands: &mut Commands,
    time: &Time,
    settings: &FlagRaiseSettings,
    rng: &mut CommandRng,
) -> CommandSpec {
    rng.state = seed_rng(rng.state, time.elapsed_secs_f64());
    let mut hand_state = HandState::default();
    let initial_command = next_command(rng, &mut hand_state, settings, 0.0);
    let initial_turn_seconds = current_turn_seconds(settings, 0.0);
    commands.insert_resource(CommandState {
        current: initial_command,
        timer: Timer::from_seconds(initial_turn_seconds, TimerMode::Once),
        dirty: true,
        beep_state: TurnBeepState::default(),
        hand_state,
        mismatch_color: false,
    });
    commands.insert_resource(Scoreboard {
        scores: [0; MAX_PLAYERS],
    });
    commands.insert_resource(GameTimer {
        timer: Timer::from_seconds(GAME_LIMIT_SECS, TimerMode::Once),
        elapsed: 0.0,
    });
    commands.remove_resource::<GameResult>();

    initial_command
}

pub fn advance_turn_and_score(
    mut commands: Commands,
    time: Res<Time>,
    people: Res<PeopleDataRes>,
    settings: Res<FlagRaiseSettings>,
    beeps: Res<BeepPalette>,
    mut scoreboard: ResMut<Scoreboard>,
    mut command: ResMut<CommandState>,
    mut rng: ResMut<CommandRng>,
    mut game_timer: ResMut<GameTimer>,
    mut phase: ResMut<FlagRaisePhase>,
) {
    let delta = time.delta();
    game_timer.timer.tick(delta);
    game_timer.elapsed += delta.as_secs_f32();
    if game_timer.timer.is_finished() {
        commands.insert_resource(GameResult {
            reason: GameOverReason::TimeUp,
            failed_slot: None,
        });
        *phase = FlagRaisePhase::Result;
        return;
    }

    command.timer.tick(delta);
    let progress = command.timer.fraction();
    if !command.beep_state.first && progress >= TURN_BEEP_FIRST {
        play_beep(&mut commands, beeps.tick.clone());
        command.beep_state.first = true;
    }
    if !command.beep_state.second && progress >= TURN_BEEP_SECOND {
        play_beep(&mut commands, beeps.tick.clone());
        command.beep_state.second = true;
    }
    if !command.timer.just_finished() {
        return;
    }

    let assignments = assign_people_to_slots(&people, settings.player_count);
    let mut failed_slot = None;
    let mut any_correct = false;
    let mut any_wrong = false;

    for (slot_index, person_index) in assignments.iter().enumerate() {
        let Some(person_index) = person_index else {
            continue;
        };
        let Some(person) = people.get(*person_index) else {
            continue;
        };
        let outcome = evaluate_command(command.current, &person.keypoints);
        match outcome {
            Some(true) => {
                scoreboard.scores[slot_index] = scoreboard.scores[slot_index].saturating_add(1);
                any_correct = true;
            }
            Some(false) | None => {
                any_wrong = true;
                if settings.mode == GameMode::SuddenDeath && failed_slot.is_none() {
                    failed_slot = Some(slot_index);
                }
            }
        }
    }

    if any_wrong {
        play_beep(&mut commands, beeps.wrong.clone());
    } else if any_correct {
        play_beep(&mut commands, beeps.correct.clone());
    }

    if settings.mode == GameMode::SuddenDeath
        && let Some(failed_slot) = failed_slot
    {
        commands.insert_resource(GameResult {
            reason: GameOverReason::Failed,
            failed_slot: Some(failed_slot),
        });
        *phase = FlagRaisePhase::Result;
        return;
    }

    command.current = next_command(
        &mut rng,
        &mut command.hand_state,
        &settings,
        game_timer.elapsed,
    );
    command.mismatch_color = should_mismatch_color(&mut rng, &game_timer.timer);
    let next_turn_seconds = current_turn_seconds(&settings, game_timer.elapsed);
    command.timer = Timer::from_seconds(next_turn_seconds, TimerMode::Once);
    command.dirty = true;
    command.beep_state = TurnBeepState::default();
}

fn assign_people_to_slots(people: &PeopleDataRes, player_count: usize) -> Vec<Option<usize>> {
    if player_count == 0 {
        return Vec::new();
    }

    let mut centers: Vec<(usize, f64)> = people
        .iter()
        .enumerate()
        .filter_map(|(index, person)| {
            let center_x = crate::pose::estimate_center_x(&person.keypoints)?;
            if !(0.0..=1.0).contains(&center_x) {
                return None;
            }
            Some((index, center_x))
        })
        .collect();

    centers.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut assignments = vec![None; player_count];
    for slot_index in 0..player_count {
        let slot_min = slot_index as f64 / player_count as f64;
        let slot_max = (slot_index + 1) as f64 / player_count as f64;
        let slot_center = (slot_min + slot_max) * 0.5;

        let mut best: Option<(usize, f64)> = None;
        for (person_index, center_x) in &centers {
            if slot_index + 1 == player_count {
                if *center_x < slot_min || *center_x > slot_max {
                    continue;
                }
            } else if *center_x < slot_min || *center_x >= slot_max {
                continue;
            }
            let distance = (*center_x - slot_center).abs();
            match best {
                Some((_, best_distance)) if distance >= best_distance => {}
                _ => best = Some((*person_index, distance)),
            }
        }
        if let Some((person_index, _)) = best {
            assignments[slot_index] = Some(person_index);
        }
    }

    assignments
}

fn seed_rng(state: u64, time_seconds: f64) -> u64 {
    let time_bits = (time_seconds * 1000.0) as u64;
    state ^ time_bits.wrapping_mul(0x9e37_79b9_7f4a_7c15)
}

fn current_turn_seconds(settings: &FlagRaiseSettings, elapsed: f32) -> f32 {
    let base = settings.difficulty.turn_seconds();
    let min_seconds = base * TURN_MIN_RATIO;
    if GAME_LIMIT_SECS <= 0.0 {
        return base;
    }
    let progress = (elapsed / GAME_LIMIT_SECS).clamp(0.0, 1.0);
    base + (min_seconds - base) * progress
}

fn should_mismatch_color(rng: &mut CommandRng, game_timer: &Timer) -> bool {
    if game_timer.fraction() < 0.5 {
        return false;
    }
    next_f32(&mut rng.state) < COLOR_MISMATCH_CHANCE
}

fn next_command(
    rng: &mut CommandRng,
    hand_state: &mut HandState,
    settings: &FlagRaiseSettings,
    elapsed: f32,
) -> CommandSpec {
    if should_use_dual_command(rng, settings, elapsed) {
        let red = next_action_for_color(rng, hand_state, FlagColor::Red);
        let white = next_action_for_color(rng, hand_state, FlagColor::White);
        return CommandSpec::Dual { red, white };
    }

    let color = if next_index(&mut rng.state, 2) == 0 {
        FlagColor::White
    } else {
        FlagColor::Red
    };
    let action = next_action_for_color(rng, hand_state, color);
    CommandSpec::Single { color, action }
}

fn should_use_dual_command(
    rng: &mut CommandRng,
    settings: &FlagRaiseSettings,
    elapsed: f32,
) -> bool {
    if settings.difficulty != super::settings::Difficulty::Hard {
        return false;
    }
    if elapsed < DUAL_COMMAND_START_SECS {
        return false;
    }
    next_f32(&mut rng.state) < DUAL_COMMAND_CHANCE
}

fn next_action_for_color(
    rng: &mut CommandRng,
    hand_state: &mut HandState,
    color: FlagColor,
) -> FlagAction {
    let was_raised = hand_state.is_raised(color);
    let will_move = next_f32(&mut rng.state) < TURN_MOVE_CHANCE;
    let (action, next_raised) = match (was_raised, will_move) {
        (false, true) => (FlagAction::Raise, true),
        (false, false) => (FlagAction::DontRaise, false),
        (true, true) => (FlagAction::Lower, false),
        (true, false) => (FlagAction::DontLower, true),
    };
    if will_move {
        hand_state.set_raised(color, next_raised);
    }
    action
}

fn next_index(state: &mut u64, len: usize) -> usize {
    (next_u32(state) as usize) % len.max(1)
}

fn next_f32(state: &mut u64) -> f32 {
    next_u32(state) as f32 / u32::MAX as f32
}

fn next_u32(state: &mut u64) -> u32 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    (*state >> 32) as u32
}

fn hand_is_raised(keypoints: &[Option<[f64; 2]>], color: FlagColor) -> Option<bool> {
    let wrist = keypoints.get(color.wrist_index()).and_then(|kp| *kp)?;
    let shoulder = keypoints.get(color.shoulder_index()).and_then(|kp| *kp)?;
    Some(wrist[1] < shoulder[1])
}

#[derive(Clone, Copy, Debug)]
pub enum FlagColor {
    White,
    Red,
}

impl FlagColor {
    fn wrist_index(self) -> usize {
        match self {
            FlagColor::White => 10,
            FlagColor::Red => 9,
        }
    }

    fn shoulder_index(self) -> usize {
        match self {
            FlagColor::White => 6,
            FlagColor::Red => 5,
        }
    }

    pub fn text_color(self) -> Color {
        match self {
            FlagColor::White => Color::srgb(0.95, 0.95, 1.0),
            FlagColor::Red => Color::srgb(1.0, 0.6, 0.6),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            FlagColor::White => "白",
            FlagColor::Red => "赤",
        }
    }

    pub(crate) fn opposite(self) -> Self {
        match self {
            FlagColor::White => FlagColor::Red,
            FlagColor::Red => FlagColor::White,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) enum FlagAction {
    Raise,
    Lower,
    DontRaise,
    DontLower,
}

impl FlagAction {
    fn expects_raised(self) -> bool {
        matches!(self, FlagAction::Raise | FlagAction::DontLower)
    }

    fn text(self) -> &'static str {
        match self {
            FlagAction::Raise => "上げて",
            FlagAction::Lower => "下げて",
            FlagAction::DontRaise => "上げないで",
            FlagAction::DontLower => "下げないで",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CommandSpec {
    Single {
        color: FlagColor,
        action: FlagAction,
    },
    Dual {
        red: FlagAction,
        white: FlagAction,
    },
}

impl CommandSpec {
    pub fn display(self) -> CommandDisplay {
        match self {
            CommandSpec::Single { color, action } => CommandDisplay {
                primary: CommandDisplayPart {
                    color,
                    action_text: action.text(),
                },
                secondary: None,
            },
            CommandSpec::Dual { red, white } => CommandDisplay {
                primary: CommandDisplayPart {
                    color: FlagColor::Red,
                    action_text: red.text(),
                },
                secondary: Some(CommandDisplayPart {
                    color: FlagColor::White,
                    action_text: white.text(),
                }),
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CommandDisplayPart {
    pub color: FlagColor,
    pub action_text: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct CommandDisplay {
    pub primary: CommandDisplayPart,
    pub secondary: Option<CommandDisplayPart>,
}

fn evaluate_command(command: CommandSpec, keypoints: &[Option<[f64; 2]>]) -> Option<bool> {
    match command {
        CommandSpec::Single { color, action } => {
            let actual = hand_is_raised(keypoints, color)?;
            Some(actual == action.expects_raised())
        }
        CommandSpec::Dual { red, white } => {
            let actual_red = hand_is_raised(keypoints, FlagColor::Red)?;
            let actual_white = hand_is_raised(keypoints, FlagColor::White)?;
            Some(actual_red == red.expects_raised() && actual_white == white.expects_raised())
        }
    }
}
