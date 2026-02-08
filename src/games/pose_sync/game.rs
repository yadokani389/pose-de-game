use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::{
    AppState,
    args::Args,
    assets::UiFont,
    pose::PeopleDataRes,
    utils::{
        pitch::{BeepPalette, play_beep},
        spawn_floating_text_popup,
    },
};

use super::settings::{Difficulty, MAX_PLAYERS, PoseSyncPhase, PoseSyncSettings};

pub const GAME_LIMIT_SECS: f32 = 60.0;
pub const SEQUENCE_LEN: usize = 3;
const START_DELAY_SECS: f32 = 1.0;
const REPEAT_JUDGE_DELAY_SECS: f32 = 0.18;
const SLOT_JUDGE_POPUP_FONT_SIZE: f32 = 58.0;
const SLOT_JUDGE_POPUP_Y: f32 = -40.0;
const POINT_SCORE_WEIGHT: f32 = 0.65;
const ANGLE_SCORE_WEIGHT: f32 = 0.35;
const HEAD_POINT_WEIGHT: f32 = 0.12;
const SHOULDER_POINT_SIGMA: f32 = 0.34;
const ELBOW_POINT_SIGMA: f32 = 0.42;
const WRIST_POINT_SIGMA: f32 = 0.72;
const HEAD_POINT_SIGMA: f32 = 0.65;
const ARM_ANGLE_SIGMA: f32 = 0.8;
const LEFT_WRIST_INDEX: usize = 9;
const RIGHT_WRIST_INDEX: usize = 10;
const JUDGE_TEMPLATE_Y_OFFSET: f32 = 0.5;

#[derive(Resource, Default)]
pub struct Scoreboard {
    pub scores: [u32; MAX_PLAYERS],
}

#[derive(Resource)]
pub struct PoseRng {
    pub state: u64,
}

impl Default for PoseRng {
    fn default() -> Self {
        Self {
            state: 0x853c_49e6_748f_ea9b,
        }
    }
}

#[derive(Resource)]
pub struct CommandState {
    pub sequence: [PoseTemplateId; SEQUENCE_LEN],
    pub stage: RoundStage,
    pub step_index: usize,
    pub timer: Timer,
    pub repeat_judged: bool,
    pub dirty: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SlotJudgeResult {
    Success,
    Failure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoundStage {
    Intro,
    Show,
    Repeat,
}

#[derive(Resource)]
pub struct GameTimer {
    pub timer: Timer,
}

#[derive(Resource, Clone, Debug)]
pub struct GameResult {
    pub ranked_players: Vec<(usize, u32)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoseTemplateId {
    TPose,
    YPose,
    HandsUp,
    Diagonal,
    LeftUp,
    RightUp,
    GoalPost,
}

impl CommandState {
    pub fn active_pose(self: &Self) -> PoseTemplateId {
        self.sequence[self.step_index]
    }
}

#[derive(Clone, Copy)]
pub struct PoseTarget {
    pub index: usize,
    pub pos: [f32; 2],
}

pub struct PoseTemplate {
    pub targets: &'static [PoseTarget],
    pub head_center: [f32; 2],
}

#[derive(Clone, Copy)]
struct PointMetric {
    index: usize,
    sigma: f32,
    weight: f32,
}

#[derive(Clone, Copy)]
struct SegmentMetric {
    start: usize,
    end: usize,
    sigma: f32,
    weight: f32,
}

const JUDGE_POINTS: &[PointMetric] = &[
    PointMetric {
        index: 5,
        sigma: SHOULDER_POINT_SIGMA,
        weight: 1.05,
    },
    PointMetric {
        index: 6,
        sigma: SHOULDER_POINT_SIGMA,
        weight: 1.05,
    },
    PointMetric {
        index: 7,
        sigma: ELBOW_POINT_SIGMA,
        weight: 1.0,
    },
    PointMetric {
        index: 8,
        sigma: ELBOW_POINT_SIGMA,
        weight: 1.0,
    },
    PointMetric {
        index: 9,
        sigma: WRIST_POINT_SIGMA,
        weight: 1.2,
    },
    PointMetric {
        index: 10,
        sigma: WRIST_POINT_SIGMA,
        weight: 1.2,
    },
];

const JUDGE_ARM_SEGMENTS: &[SegmentMetric] = &[
    SegmentMetric {
        start: 5,
        end: 7,
        sigma: ARM_ANGLE_SIGMA,
        weight: 1.0,
    },
    SegmentMetric {
        start: 7,
        end: 9,
        sigma: ARM_ANGLE_SIGMA,
        weight: 1.15,
    },
    SegmentMetric {
        start: 6,
        end: 8,
        sigma: ARM_ANGLE_SIGMA,
        weight: 1.0,
    },
    SegmentMetric {
        start: 8,
        end: 10,
        sigma: ARM_ANGLE_SIGMA,
        weight: 1.15,
    },
];

pub const HOLE_EDGES: &[(usize, usize)] = &[
    (5, 7),
    (7, 9),
    (6, 8),
    (8, 10),
    (5, 6),
    (5, 11),
    (6, 12),
    (11, 12),
];

pub const SHOW_DRAW_EDGES: &[(usize, usize)] = &[
    (5, 7),
    (7, 9),
    (6, 8),
    (8, 10),
    (5, 6),
    (5, 11),
    (6, 12),
    (11, 12),
    (11, 13),
    (13, 15),
    (12, 14),
    (14, 16),
];

const T_POSE_TARGETS: &[PoseTarget] = &[
    PoseTarget {
        index: 5,
        pos: [-0.5, 0.5],
    },
    PoseTarget {
        index: 6,
        pos: [0.5, 0.5],
    },
    PoseTarget {
        index: 7,
        pos: [-1.0, 0.5],
    },
    PoseTarget {
        index: 8,
        pos: [1.0, 0.5],
    },
    PoseTarget {
        index: 9,
        pos: [-1.5, 0.5],
    },
    PoseTarget {
        index: 10,
        pos: [1.5, 0.5],
    },
    PoseTarget {
        index: 11,
        pos: [-0.4, -0.5],
    },
    PoseTarget {
        index: 12,
        pos: [0.4, -0.5],
    },
    PoseTarget {
        index: 13,
        pos: [-0.4, -1.4],
    },
    PoseTarget {
        index: 14,
        pos: [0.4, -1.4],
    },
    PoseTarget {
        index: 15,
        pos: [-0.4, -2.0],
    },
    PoseTarget {
        index: 16,
        pos: [0.4, -2.0],
    },
];

const Y_POSE_TARGETS: &[PoseTarget] = &[
    PoseTarget {
        index: 5,
        pos: [-0.5, 0.5],
    },
    PoseTarget {
        index: 6,
        pos: [0.5, 0.5],
    },
    PoseTarget {
        index: 7,
        pos: [-0.8, 1.0],
    },
    PoseTarget {
        index: 8,
        pos: [0.8, 1.0],
    },
    PoseTarget {
        index: 9,
        pos: [-1.1, 1.6],
    },
    PoseTarget {
        index: 10,
        pos: [1.1, 1.6],
    },
    PoseTarget {
        index: 11,
        pos: [-0.4, -0.5],
    },
    PoseTarget {
        index: 12,
        pos: [0.4, -0.5],
    },
    PoseTarget {
        index: 13,
        pos: [-0.4, -1.4],
    },
    PoseTarget {
        index: 14,
        pos: [0.4, -1.4],
    },
    PoseTarget {
        index: 15,
        pos: [-0.4, -2.0],
    },
    PoseTarget {
        index: 16,
        pos: [0.4, -2.0],
    },
];

const UP_POSE_TARGETS: &[PoseTarget] = &[
    PoseTarget {
        index: 5,
        pos: [-0.5, 0.5],
    },
    PoseTarget {
        index: 6,
        pos: [0.5, 0.5],
    },
    PoseTarget {
        index: 7,
        pos: [-0.3, 1.1],
    },
    PoseTarget {
        index: 8,
        pos: [0.3, 1.1],
    },
    PoseTarget {
        index: 9,
        pos: [-0.3, 1.8],
    },
    PoseTarget {
        index: 10,
        pos: [0.3, 1.8],
    },
    PoseTarget {
        index: 11,
        pos: [-0.4, -0.5],
    },
    PoseTarget {
        index: 12,
        pos: [0.4, -0.5],
    },
    PoseTarget {
        index: 13,
        pos: [-0.4, -1.4],
    },
    PoseTarget {
        index: 14,
        pos: [0.4, -1.4],
    },
    PoseTarget {
        index: 15,
        pos: [-0.4, -2.0],
    },
    PoseTarget {
        index: 16,
        pos: [0.4, -2.0],
    },
];

const DIAGONAL_POSE_TARGETS: &[PoseTarget] = &[
    PoseTarget {
        index: 5,
        pos: [-0.5, 0.5],
    },
    PoseTarget {
        index: 6,
        pos: [0.5, 0.5],
    },
    PoseTarget {
        index: 7,
        pos: [-0.9, 1.0],
    },
    PoseTarget {
        index: 8,
        pos: [0.9, 1.0],
    },
    PoseTarget {
        index: 9,
        pos: [-0.3, 1.55],
    },
    PoseTarget {
        index: 10,
        pos: [0.3, 1.55],
    },
    PoseTarget {
        index: 11,
        pos: [-0.4, -0.5],
    },
    PoseTarget {
        index: 12,
        pos: [0.4, -0.5],
    },
    PoseTarget {
        index: 13,
        pos: [-0.4, -1.4],
    },
    PoseTarget {
        index: 14,
        pos: [0.4, -1.4],
    },
    PoseTarget {
        index: 15,
        pos: [-0.4, -2.0],
    },
    PoseTarget {
        index: 16,
        pos: [0.4, -2.0],
    },
];

const LEFT_UP_POSE_TARGETS: &[PoseTarget] = &[
    PoseTarget {
        index: 5,
        pos: [-0.5, 0.5],
    },
    PoseTarget {
        index: 6,
        pos: [0.5, 0.5],
    },
    PoseTarget {
        index: 7,
        pos: [-0.8, 1.1],
    },
    PoseTarget {
        index: 8,
        pos: [0.9, 0.2],
    },
    PoseTarget {
        index: 9,
        pos: [-1.0, 1.8],
    },
    PoseTarget {
        index: 10,
        pos: [1.35, 0.2],
    },
    PoseTarget {
        index: 11,
        pos: [-0.4, -0.5],
    },
    PoseTarget {
        index: 12,
        pos: [0.4, -0.5],
    },
    PoseTarget {
        index: 13,
        pos: [-0.4, -1.4],
    },
    PoseTarget {
        index: 14,
        pos: [0.4, -1.4],
    },
    PoseTarget {
        index: 15,
        pos: [-0.4, -2.0],
    },
    PoseTarget {
        index: 16,
        pos: [0.4, -2.0],
    },
];

const RIGHT_UP_POSE_TARGETS: &[PoseTarget] = &[
    PoseTarget {
        index: 5,
        pos: [-0.5, 0.5],
    },
    PoseTarget {
        index: 6,
        pos: [0.5, 0.5],
    },
    PoseTarget {
        index: 7,
        pos: [-0.9, 0.2],
    },
    PoseTarget {
        index: 8,
        pos: [0.8, 1.1],
    },
    PoseTarget {
        index: 9,
        pos: [-1.35, 0.2],
    },
    PoseTarget {
        index: 10,
        pos: [1.0, 1.8],
    },
    PoseTarget {
        index: 11,
        pos: [-0.4, -0.5],
    },
    PoseTarget {
        index: 12,
        pos: [0.4, -0.5],
    },
    PoseTarget {
        index: 13,
        pos: [-0.4, -1.4],
    },
    PoseTarget {
        index: 14,
        pos: [0.4, -1.4],
    },
    PoseTarget {
        index: 15,
        pos: [-0.4, -2.0],
    },
    PoseTarget {
        index: 16,
        pos: [0.4, -2.0],
    },
];

const GOALPOST_POSE_TARGETS: &[PoseTarget] = &[
    PoseTarget {
        index: 5,
        pos: [-0.5, 0.5],
    },
    PoseTarget {
        index: 6,
        pos: [0.5, 0.5],
    },
    PoseTarget {
        index: 7,
        pos: [-1.05, 0.8],
    },
    PoseTarget {
        index: 8,
        pos: [1.05, 0.8],
    },
    PoseTarget {
        index: 9,
        pos: [-1.05, 1.55],
    },
    PoseTarget {
        index: 10,
        pos: [1.05, 1.55],
    },
    PoseTarget {
        index: 11,
        pos: [-0.4, -0.5],
    },
    PoseTarget {
        index: 12,
        pos: [0.4, -0.5],
    },
    PoseTarget {
        index: 13,
        pos: [-0.4, -1.4],
    },
    PoseTarget {
        index: 14,
        pos: [0.4, -1.4],
    },
    PoseTarget {
        index: 15,
        pos: [-0.4, -2.0],
    },
    PoseTarget {
        index: 16,
        pos: [0.4, -2.0],
    },
];

const T_TEMPLATE: PoseTemplate = PoseTemplate {
    targets: T_POSE_TARGETS,
    head_center: [0.0, 1.3],
};
const Y_TEMPLATE: PoseTemplate = PoseTemplate {
    targets: Y_POSE_TARGETS,
    head_center: [0.0, 1.4],
};
const UP_TEMPLATE: PoseTemplate = PoseTemplate {
    targets: UP_POSE_TARGETS,
    head_center: [0.0, 1.5],
};
const DIAGONAL_TEMPLATE: PoseTemplate = PoseTemplate {
    targets: DIAGONAL_POSE_TARGETS,
    head_center: [0.0, 1.4],
};
const LEFT_UP_TEMPLATE: PoseTemplate = PoseTemplate {
    targets: LEFT_UP_POSE_TARGETS,
    head_center: [0.0, 1.4],
};
const RIGHT_UP_TEMPLATE: PoseTemplate = PoseTemplate {
    targets: RIGHT_UP_POSE_TARGETS,
    head_center: [0.0, 1.4],
};
const GOALPOST_TEMPLATE: PoseTemplate = PoseTemplate {
    targets: GOALPOST_POSE_TARGETS,
    head_center: [0.0, 1.35],
};

pub fn template(id: PoseTemplateId) -> &'static PoseTemplate {
    match id {
        PoseTemplateId::TPose => &T_TEMPLATE,
        PoseTemplateId::YPose => &Y_TEMPLATE,
        PoseTemplateId::HandsUp => &UP_TEMPLATE,
        PoseTemplateId::Diagonal => &DIAGONAL_TEMPLATE,
        PoseTemplateId::LeftUp => &LEFT_UP_TEMPLATE,
        PoseTemplateId::RightUp => &RIGHT_UP_TEMPLATE,
        PoseTemplateId::GoalPost => &GOALPOST_TEMPLATE,
    }
}

pub fn start_game(commands: &mut Commands, time: &Time, rng: &mut PoseRng) {
    rng.state = seed_rng(rng.state, time.elapsed_secs_f64());
    let sequence = next_sequence(rng);
    commands.insert_resource(CommandState {
        sequence,
        stage: RoundStage::Intro,
        step_index: 0,
        timer: Timer::from_seconds(START_DELAY_SECS, TimerMode::Once),
        repeat_judged: false,
        dirty: true,
    });
    commands.insert_resource(Scoreboard {
        scores: [0; MAX_PLAYERS],
    });
    commands.insert_resource(GameTimer {
        timer: Timer::from_seconds(GAME_LIMIT_SECS, TimerMode::Once),
    });
    commands.remove_resource::<GameResult>();
}

pub fn advance_turn_and_score(
    mut commands: Commands,
    time: Res<Time>,
    args: Res<Args>,
    people: Res<PeopleDataRes>,
    settings: Res<PoseSyncSettings>,
    beeps: Res<BeepPalette>,
    ui_font: Res<UiFont>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut scoreboard: ResMut<Scoreboard>,
    mut command: ResMut<CommandState>,
    mut rng: ResMut<PoseRng>,
    mut game_timer: ResMut<GameTimer>,
    mut phase: ResMut<PoseSyncPhase>,
) {
    let delta = time.delta();
    game_timer.timer.tick(delta);
    if game_timer.timer.is_finished() {
        let mut ranked_players: Vec<(usize, u32)> = (0..settings.player_count)
            .map(|index| (index, scoreboard.scores[index]))
            .collect();
        ranked_players.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        commands.insert_resource(GameResult { ranked_players });
        *phase = PoseSyncPhase::Result;
        return;
    }

    command.timer.tick(delta);

    if command.stage == RoundStage::Repeat && !command.repeat_judged {
        let duration_secs = command.timer.duration().as_secs_f32();
        if duration_secs <= 0.0 {
            let pose = command.active_pose();
            let slot_results = judge_repeat_step(
                &people,
                args.mirror_camera,
                &settings,
                &mut scoreboard,
                pose,
            );
            spawn_slot_judge_popups(&mut commands, &window, &settings, &ui_font, &slot_results);
            command.repeat_judged = true;
        } else {
            let judge_progress = (REPEAT_JUDGE_DELAY_SECS / duration_secs).clamp(0.0, 1.0);
            if judge_progress <= command.timer.fraction() {
                let pose = command.active_pose();
                let slot_results = judge_repeat_step(
                    &people,
                    args.mirror_camera,
                    &settings,
                    &mut scoreboard,
                    pose,
                );
                spawn_slot_judge_popups(&mut commands, &window, &settings, &ui_font, &slot_results);
                command.repeat_judged = true;
            }
        }
    }

    if !command.timer.just_finished() {
        return;
    }

    match command.stage {
        RoundStage::Intro => {
            command.stage = RoundStage::Show;
            command.step_index = 0;
            command.timer =
                Timer::from_seconds(settings.difficulty.preview_seconds(), TimerMode::Once);
            command.repeat_judged = false;
            command.dirty = true;
            play_beep(&mut commands, beeps.tick.clone());
        }
        RoundStage::Show => {
            if command.step_index + 1 < SEQUENCE_LEN {
                command.step_index += 1;
                command.timer =
                    Timer::from_seconds(settings.difficulty.preview_seconds(), TimerMode::Once);
                command.repeat_judged = false;
                command.dirty = true;
                play_beep(&mut commands, beeps.tick.clone());
                return;
            }

            command.stage = RoundStage::Repeat;
            command.step_index = 0;
            command.timer =
                Timer::from_seconds(settings.difficulty.turn_seconds(), TimerMode::Once);
            command.repeat_judged = false;
            command.dirty = true;
            play_beep(&mut commands, beeps.tick.clone());
        }
        RoundStage::Repeat => {
            if !command.repeat_judged {
                let pose = command.active_pose();
                let slot_results = judge_repeat_step(
                    &people,
                    args.mirror_camera,
                    &settings,
                    &mut scoreboard,
                    pose,
                );
                spawn_slot_judge_popups(&mut commands, &window, &settings, &ui_font, &slot_results);
                command.repeat_judged = true;
            }

            if command.step_index + 1 < SEQUENCE_LEN {
                command.step_index += 1;
                command.timer =
                    Timer::from_seconds(settings.difficulty.turn_seconds(), TimerMode::Once);
                command.repeat_judged = false;
                command.dirty = true;
                play_beep(&mut commands, beeps.tick.clone());
            } else {
                command.sequence = next_sequence(&mut rng);
                command.stage = RoundStage::Show;
                command.step_index = 0;
                command.timer =
                    Timer::from_seconds(settings.difficulty.preview_seconds(), TimerMode::Once);
                command.repeat_judged = false;
                command.dirty = true;
                play_beep(&mut commands, beeps.tick.clone());
            }
        }
    }
}

fn judge_repeat_step(
    people: &PeopleDataRes,
    mirror_camera: bool,
    settings: &PoseSyncSettings,
    scoreboard: &mut Scoreboard,
    pose: PoseTemplateId,
) -> [Option<SlotJudgeResult>; MAX_PLAYERS] {
    let assignments = assign_people_to_slots(people, settings.player_count);
    let mut slot_results = [None; MAX_PLAYERS];

    for slot_index in 0..settings.player_count {
        let person_index = assignments.get(slot_index).copied().flatten();
        let Some(person_index) = person_index else {
            continue;
        };
        let Some(person) = people.get(person_index) else {
            continue;
        };
        if evaluate_pose(pose, &person.keypoints, mirror_camera, settings.difficulty) {
            scoreboard.scores[slot_index] = scoreboard.scores[slot_index].saturating_add(1);
            slot_results[slot_index] = Some(SlotJudgeResult::Success);
        } else {
            slot_results[slot_index] = Some(SlotJudgeResult::Failure);
        }
    }

    slot_results
}

fn spawn_slot_judge_popups(
    commands: &mut Commands,
    window: &Query<&Window, With<PrimaryWindow>>,
    settings: &PoseSyncSettings,
    ui_font: &UiFont,
    slot_results: &[Option<SlotJudgeResult>; MAX_PLAYERS],
) {
    if settings.player_count == 0 {
        return;
    }
    let Ok(window) = window.single() else {
        return;
    };

    let frame_size = Vec2::new(window.resolution.width(), window.resolution.height());
    let half_width = frame_size.x * 0.5;
    let slot_width = frame_size.x / settings.player_count as f32;

    for slot_index in 0..settings.player_count {
        let Some(slot_result) = slot_results[slot_index] else {
            continue;
        };

        let (label, color) = match slot_result {
            SlotJudgeResult::Success => ("成功!", Color::srgb(0.42, 1.0, 0.62)),
            SlotJudgeResult::Failure => ("ミス!", Color::srgb(1.0, 0.46, 0.46)),
        };

        let x = -half_width + slot_width * (slot_index as f32 + 0.5);
        let popup_entity = spawn_floating_text_popup(
            commands,
            Vec2::new(x, SLOT_JUDGE_POPUP_Y),
            label,
            color,
            Some(ui_font.0.clone()),
            SLOT_JUDGE_POPUP_FONT_SIZE,
        );
        commands
            .entity(popup_entity)
            .insert(DespawnOnExit(AppState::PoseSync));
    }
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
                if *center_x < slot_min || slot_max < *center_x {
                    continue;
                }
            } else if *center_x < slot_min || slot_max <= *center_x {
                continue;
            }

            let distance = (*center_x - slot_center).abs();
            match best {
                Some((_, best_distance)) if best_distance <= distance => {}
                _ => best = Some((*person_index, distance)),
            }
        }

        if let Some((person_index, _)) = best {
            assignments[slot_index] = Some(person_index);
        }
    }

    assignments
}

fn evaluate_pose(
    template_id: PoseTemplateId,
    keypoints: &[Option<[f64; 2]>],
    mirror_camera: bool,
    difficulty: Difficulty,
) -> bool {
    let threshold = pose_pass_threshold(difficulty);
    let min_wrist = min_wrist_guard_score(difficulty);

    let Some((points, head)) = normalize_keypoints(keypoints, mirror_camera) else {
        return false;
    };

    let pose = template(template_id);
    let left_wrist_score = point_match_score(pose, &points, LEFT_WRIST_INDEX, WRIST_POINT_SIGMA);
    let right_wrist_score = point_match_score(pose, &points, RIGHT_WRIST_INDEX, WRIST_POINT_SIGMA);
    let point_score = pose_point_score(pose, &points, head);
    let angle_score = pose_angle_score(pose, &points);
    let final_score = POINT_SCORE_WEIGHT * point_score + ANGLE_SCORE_WEIGHT * angle_score;
    if left_wrist_score < min_wrist || right_wrist_score < min_wrist {
        return false;
    }

    threshold <= final_score
}

fn pose_pass_threshold(difficulty: Difficulty) -> f32 {
    match difficulty {
        Difficulty::Easy => 0.46,
        Difficulty::Normal => 0.52,
        Difficulty::Hard => 0.60,
    }
}

fn min_wrist_guard_score(difficulty: Difficulty) -> f32 {
    match difficulty {
        Difficulty::Easy => 0.05,
        Difficulty::Normal => 0.08,
        Difficulty::Hard => 0.12,
    }
}

fn pose_point_score(pose: &PoseTemplate, points: &[Option<Vec2>], head: Option<Vec2>) -> f32 {
    let mut weighted_sum = 0.0;
    let mut weight_total = 0.0;

    for metric in JUDGE_POINTS {
        weight_total += metric.weight;
        weighted_sum += metric.weight * point_match_score(pose, points, metric.index, metric.sigma);
    }

    if let Some(actual_head) = head {
        let expected_head = judge_head_center(pose);
        weighted_sum += HEAD_POINT_WEIGHT
            * gaussian_score(actual_head.distance(expected_head), HEAD_POINT_SIGMA);
        weight_total += HEAD_POINT_WEIGHT;
    }

    if weight_total <= f32::EPSILON {
        return 0.0;
    }

    (weighted_sum / weight_total).clamp(0.0, 1.0)
}

fn pose_angle_score(pose: &PoseTemplate, points: &[Option<Vec2>]) -> f32 {
    let mut weighted_sum = 0.0;
    let mut weight_total = 0.0;

    for metric in JUDGE_ARM_SEGMENTS {
        weight_total += metric.weight;

        let Some(actual_start) = points.get(metric.start).and_then(|point| *point) else {
            continue;
        };
        let Some(actual_end) = points.get(metric.end).and_then(|point| *point) else {
            continue;
        };
        let Some(expected_start) = judge_pose_point(pose, metric.start) else {
            continue;
        };
        let Some(expected_end) = judge_pose_point(pose, metric.end) else {
            continue;
        };

        let actual_segment = actual_end - actual_start;
        let expected_segment = expected_end - expected_start;
        if actual_segment.length_squared() <= f32::EPSILON
            || expected_segment.length_squared() <= f32::EPSILON
        {
            continue;
        }

        let actual_theta = actual_segment.y.atan2(actual_segment.x);
        let expected_theta = expected_segment.y.atan2(expected_segment.x);
        let delta_theta = smallest_angle_delta(actual_theta, expected_theta);
        weighted_sum += metric.weight * gaussian_score(delta_theta.abs(), metric.sigma);
    }

    if weight_total <= f32::EPSILON {
        return 0.0;
    }

    (weighted_sum / weight_total).clamp(0.0, 1.0)
}

fn point_match_score(
    pose: &PoseTemplate,
    points: &[Option<Vec2>],
    index: usize,
    sigma: f32,
) -> f32 {
    let Some(actual) = points.get(index).and_then(|point| *point) else {
        return 0.0;
    };
    let Some(expected) = judge_pose_point(pose, index) else {
        return 0.0;
    };

    gaussian_score(actual.distance(expected), sigma)
}

fn gaussian_score(distance: f32, sigma: f32) -> f32 {
    if sigma <= f32::EPSILON {
        return 0.0;
    }

    (-((distance / sigma).powi(2))).exp()
}

fn smallest_angle_delta(lhs: f32, rhs: f32) -> f32 {
    let delta = lhs - rhs;
    delta.sin().atan2(delta.cos())
}

fn pose_point(pose: &PoseTemplate, index: usize) -> Option<Vec2> {
    pose.targets
        .iter()
        .find(|target| target.index == index)
        .map(|target| Vec2::new(target.pos[0], target.pos[1]))
}

fn judge_pose_point(pose: &PoseTemplate, index: usize) -> Option<Vec2> {
    pose_point(pose, index).map(|point| Vec2::new(point.x, point.y - JUDGE_TEMPLATE_Y_OFFSET))
}

fn judge_head_center(pose: &PoseTemplate) -> Vec2 {
    Vec2::new(
        pose.head_center[0],
        pose.head_center[1] - JUDGE_TEMPLATE_Y_OFFSET,
    )
}

fn normalize_keypoints(
    keypoints: &[Option<[f64; 2]>],
    mirror_camera: bool,
) -> Option<(Vec<Option<Vec2>>, Option<Vec2>)> {
    let left_shoulder = keypoints.get(5).and_then(|point| *point)?;
    let right_shoulder = keypoints.get(6).and_then(|point| *point)?;
    let center = Vec2::new(
        ((left_shoulder[0] + right_shoulder[0]) * 0.5) as f32,
        ((left_shoulder[1] + right_shoulder[1]) * 0.5) as f32,
    );

    let shoulder_vec = Vec2::new(
        (right_shoulder[0] - left_shoulder[0]) as f32,
        (right_shoulder[1] - left_shoulder[1]) as f32,
    );
    let scale = shoulder_vec.length();
    if scale <= f32::EPSILON {
        return None;
    }

    let mut points: Vec<Option<Vec2>> = keypoints
        .iter()
        .map(|kp| {
            kp.map(|point| {
                let mut normalized = normalize_point(point, center, scale);
                if mirror_camera {
                    normalized.x = -normalized.x;
                }
                normalized
            })
        })
        .collect();

    let Some(left) = points.get(5).and_then(|point| *point) else {
        return None;
    };
    let Some(right) = points.get(6).and_then(|point| *point) else {
        return None;
    };

    let shoulder_direction = right - left;
    let shoulder_angle = shoulder_direction.y.atan2(shoulder_direction.x);
    let rotation = Mat2::from_angle(-shoulder_angle);

    for point in &mut points {
        if let Some(value) = point.as_mut() {
            *value = rotation * *value;
        }
    }

    let head = head_point(keypoints).map(|point| {
        let mut normalized = normalize_point(point, center, scale);
        if mirror_camera {
            normalized.x = -normalized.x;
        }
        rotation * normalized
    });

    Some((points, head))
}

fn normalize_point(point: [f64; 2], center: Vec2, scale: f32) -> Vec2 {
    Vec2::new(
        (point[0] as f32 - center.x) / scale,
        (center.y - point[1] as f32) / scale,
    )
}

fn head_point(keypoints: &[Option<[f64; 2]>]) -> Option<[f64; 2]> {
    let mut sum = Vec2::ZERO;
    let mut count = 0.0;
    for index in 0..=4 {
        let Some(point) = keypoints.get(index).and_then(|kp| *kp) else {
            continue;
        };
        sum += Vec2::new(point[0] as f32, point[1] as f32);
        count += 1.0;
    }
    if count > 0.0 {
        let avg = sum / count;
        return Some([avg.x as f64, avg.y as f64]);
    }

    None
}

fn next_sequence(rng: &mut PoseRng) -> [PoseTemplateId; SEQUENCE_LEN] {
    let mut sequence = [PoseTemplateId::TPose; SEQUENCE_LEN];
    for index in 0..SEQUENCE_LEN {
        let current = if index == 0 {
            None
        } else {
            Some(sequence[index - 1])
        };
        sequence[index] = next_template(rng, current);
    }
    sequence
}

fn next_template(rng: &mut PoseRng, current: Option<PoseTemplateId>) -> PoseTemplateId {
    let all = [
        PoseTemplateId::TPose,
        PoseTemplateId::YPose,
        PoseTemplateId::HandsUp,
        PoseTemplateId::Diagonal,
        PoseTemplateId::LeftUp,
        PoseTemplateId::RightUp,
        PoseTemplateId::GoalPost,
    ];
    let mut next = all[next_index(&mut rng.state, all.len())];
    if let Some(current) = current
        && next == current
    {
        next = all[(next_index(&mut rng.state, all.len()) + 1) % all.len()];
    }
    next
}

fn seed_rng(state: u64, time_seconds: f64) -> u64 {
    let time_bits = (time_seconds * 1000.0) as u64;
    state ^ time_bits.wrapping_mul(0x9e37_79b9_7f4a_7c15)
}

fn next_index(state: &mut u64, len: usize) -> usize {
    (next_u32(state) as usize) % len.max(1)
}

fn next_u32(state: &mut u64) -> u32 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    (*state >> 32) as u32
}
