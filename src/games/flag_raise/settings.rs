use bevy::prelude::*;

pub const MIN_PLAYERS: usize = 1;
pub const MAX_PLAYERS: usize = 4;

#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlagRaisePhase {
    #[default]
    Setup,
    Playing,
    Result,
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct FlagRaiseSettings {
    pub player_count: usize,
    pub difficulty: Difficulty,
    pub mode: GameMode,
}

impl Default for FlagRaiseSettings {
    fn default() -> Self {
        Self {
            player_count: MIN_PLAYERS,
            difficulty: Difficulty::Normal,
            mode: GameMode::Normal,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameMode {
    Normal,
    SuddenDeath,
}

impl GameMode {
    pub fn label(self) -> &'static str {
        match self {
            GameMode::Normal => "ノーマル",
            GameMode::SuddenDeath => "サドンデス",
        }
    }

    pub fn next(self) -> Self {
        match self {
            GameMode::Normal => GameMode::SuddenDeath,
            GameMode::SuddenDeath => GameMode::Normal,
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
            Difficulty::Easy => 4.0,
            Difficulty::Normal => 3.0,
            Difficulty::Hard => 2.0,
        }
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

pub fn is_setup(phase: Option<Res<FlagRaisePhase>>) -> bool {
    matches!(phase.as_deref(), Some(FlagRaisePhase::Setup))
}

pub fn is_playing(phase: Option<Res<FlagRaisePhase>>) -> bool {
    matches!(phase.as_deref(), Some(FlagRaisePhase::Playing))
}

pub fn is_result(phase: Option<Res<FlagRaisePhase>>) -> bool {
    matches!(phase.as_deref(), Some(FlagRaisePhase::Result))
}
