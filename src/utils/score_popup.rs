use bevy::prelude::*;

const DEFAULT_LIFETIME: f32 = 1.0;

#[derive(Component)]
pub struct ScorePopup {
    pub velocity: f32,
    pub timer: Timer,
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
            timer: Timer::from_seconds(DEFAULT_LIFETIME, TimerMode::Once),
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

pub fn update_score_popups(
    mut commands: Commands,
    time: Res<Time>,
    mut popup_query: Query<(Entity, &mut ScorePopup, &mut Transform, &mut TextColor)>,
) {
    let dt = time.delta_secs();

    for (entity, mut popup, mut transform, mut text_color) in popup_query.iter_mut() {
        popup.timer.tick(time.delta());

        transform.translation.y += popup.velocity * dt;

        let elapsed = popup.timer.elapsed_secs();
        let duration = popup.timer.duration().as_secs_f32();
        let alpha = popup.initial_alpha * (1.0 - (elapsed / duration)).max(0.0);
        text_color.0 = text_color.0.with_alpha(alpha);

        if popup.timer.finished() {
            commands.entity(entity).despawn();
        }
    }
}
