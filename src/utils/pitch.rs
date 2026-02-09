use bevy::audio::Volume;
use bevy::prelude::*;
use std::time::Duration;

#[derive(Resource, Clone)]
pub struct BeepPalette {
    pub tick: Handle<Pitch>,
    pub correct: Handle<Pitch>,
    pub wrong: Handle<Pitch>,
}

pub fn setup_beeps(mut commands: Commands, mut pitch_assets: ResMut<Assets<Pitch>>) {
    let tick = pitch_assets.add(Pitch::new(800.0, Duration::from_millis(80)));
    let correct = pitch_assets.add(Pitch::new(1100.0, Duration::from_millis(120)));
    let wrong = pitch_assets.add(Pitch::new(300.0, Duration::from_millis(180)));
    commands.insert_resource(BeepPalette {
        tick,
        correct,
        wrong,
    });
}

pub fn play_beep(commands: &mut Commands, handle: Handle<Pitch>) {
    commands.spawn((
        AudioPlayer(handle),
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.2)),
    ));
}
