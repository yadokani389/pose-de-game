use bevy::prelude::*;

pub const MIN_PLAYERS: usize = 1;
pub const MAX_PLAYERS: usize = 4;

#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoseSyncPhase {
    #[default]
    Setup,
    Playing,
    Result,
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct PoseSyncSettings {
    pub player_count: usize,
    pub difficulty: Difficulty,
}

impl Default for PoseSyncSettings {
    fn default() -> Self {
        Self {
            player_count: MIN_PLAYERS,
            difficulty: Difficulty::Normal,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    pub fn label(self) -> &'static str {
        match self {
            Difficulty::Easy => "イージー",
            Difficulty::Normal => "ノーマル",
            Difficulty::Hard => "ハード",
        }
    }

    pub fn turn_seconds(self) -> f32 {
        match self {
            Difficulty::Easy => 1.2,
            Difficulty::Normal => 1.0,
            Difficulty::Hard => 0.85,
        }
    }

    pub fn preview_seconds(self) -> f32 {
        self.turn_seconds()
    }

    pub fn next(self) -> Self {
        match self {
            Difficulty::Easy => Difficulty::Normal,
            Difficulty::Normal => Difficulty::Hard,
            Difficulty::Hard => Difficulty::Easy,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Difficulty::Easy => Difficulty::Hard,
            Difficulty::Normal => Difficulty::Easy,
            Difficulty::Hard => Difficulty::Normal,
        }
    }
}

pub fn is_setup(phase: Option<Res<PoseSyncPhase>>) -> bool {
    matches!(phase.as_deref(), Some(PoseSyncPhase::Setup))
}

pub fn is_playing(phase: Option<Res<PoseSyncPhase>>) -> bool {
    matches!(phase.as_deref(), Some(PoseSyncPhase::Playing))
}

pub fn is_result(phase: Option<Res<PoseSyncPhase>>) -> bool {
    matches!(phase.as_deref(), Some(PoseSyncPhase::Result))
}
