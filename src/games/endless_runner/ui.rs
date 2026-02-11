use bevy::prelude::*;

use super::game::{GameSettings, Scoreboard};

#[derive(Component)]
pub struct DistanceText;

#[derive(Component)]
pub struct Player1DistanceText;

#[derive(Component)]
pub struct Player2DistanceText;

pub fn update_hud(
    scoreboard: Res<Scoreboard>,
    settings: Res<GameSettings>,
    mut p1_query: Query<&mut Text, (With<Player1DistanceText>, Without<Player2DistanceText>)>,
    mut p2_query: Query<&mut Text, (With<Player2DistanceText>, Without<Player1DistanceText>)>,
) {
    if !scoreboard.is_changed() {
        return;
    }

    let distance1 = scoreboard.player1_distance as u32;

    for mut text in p1_query.iter_mut() {
        if settings.num_players == 1 {
            *text = Text::new(format!("{} m", distance1));
        } else {
            *text = Text::new(format!("P1: {} m", distance1));
        }
    }

    if settings.num_players == 2 {
        let distance2 = scoreboard.player2_distance as u32;
        for mut text in p2_query.iter_mut() {
            *text = Text::new(format!("P2: {} m", distance2));
        }
    }
}
