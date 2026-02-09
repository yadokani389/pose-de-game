use bevy::prelude::*;

#[derive(Component)]
pub struct ScorePopup {
    pub velocity: f32,
    pub lifetime: f32,
    pub initial_alpha: f32,
}

pub fn spawn_score_popup(
    commands: &mut Commands,
    position: Vec2,
    score: u32,
    color: Color,
    font_size: f32,
) {
    commands.spawn((
        ScorePopup {
            velocity: 200.0,
            lifetime: 0.0,
            initial_alpha: 0.9,
        },
        Text2d::new(format!("+{}", score)),
        TextFont {
            font_size,
            ..default()
        },
        TextColor(color),
        Transform::from_xyz(position.x, position.y, 5.0),
    ));
}
