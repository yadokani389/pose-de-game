use bevy::prelude::*;

const DEFAULT_LIFETIME: f32 = 1.0;

#[derive(Component)]
pub struct FloatingTextPopup {
    pub velocity: f32,
    pub timer: Timer,
    pub initial_alpha: f32,
}

pub fn spawn_floating_text_popup(
    commands: &mut Commands,
    position: Vec2,
    text: impl Into<String>,
    color: Color,
    font: Option<Handle<Font>>,
    font_size: f32,
) -> Entity {
    let mut text_font = TextFont {
        font_size,
        ..default()
    };
    if let Some(font) = font {
        text_font.font = font;
    }

    commands
        .spawn((
            FloatingTextPopup {
                velocity: 200.0,
                timer: Timer::from_seconds(DEFAULT_LIFETIME, TimerMode::Once),
                initial_alpha: 0.9,
            },
            Text2d::new(text.into()),
            text_font,
            TextColor(color),
            Transform::from_xyz(position.x, position.y, 5.0),
        ))
        .id()
}

pub fn update_floating_text_popups(
    mut commands: Commands,
    time: Res<Time>,
    mut popup_query: Query<(
        Entity,
        &mut FloatingTextPopup,
        &mut Transform,
        &mut TextColor,
    )>,
) {
    let dt = time.delta_secs();

    for (entity, mut popup, mut transform, mut text_color) in popup_query.iter_mut() {
        popup.timer.tick(time.delta());

        transform.translation.y += popup.velocity * dt;

        let elapsed = popup.timer.elapsed_secs();
        let duration = popup.timer.duration().as_secs_f32();
        let alpha = popup.initial_alpha * (1.0 - (elapsed / duration)).max(0.0);
        text_color.0 = text_color.0.with_alpha(alpha);

        if popup.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}
