use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use crate::{
    breakout::field::{CELL_SIZE, FIELD_WIDTH},
    pose::PeopleDataRes,
};

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, create_image)
            .add_systems(Update, (show_right_hand, show_person_image));
    }
}

#[derive(Resource)]
struct PersonImageDebug {
    handle: Handle<Image>,
}

fn create_image(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let extent = Extent3d {
        width: 1000,
        height: 1000,
        depth_or_array_layers: 1,
    };

    let image = Image::new_fill(
        extent,
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let image_handle = images.add(image);

    commands.spawn((
        Sprite {
            image: image_handle.clone(),
            ..default()
        },
        Transform::default(),
    ));

    commands.insert_resource(PersonImageDebug {
        handle: image_handle,
    });
}

fn show_right_hand(people: Res<PeopleDataRes>, mut gizmos: Gizmos) {
    let Some(mut pos) = get_right_hand_pos(&people) else {
        return;
    };

    pos[0] -= 0.5;
    pos[0] *= -2. * FIELD_WIDTH as f64 * CELL_SIZE as f64;
    pos[1] *= -2. * FIELD_WIDTH as f64 * CELL_SIZE as f64;
    println!("{pos:?}");
    gizmos.circle_2d(Vec2::new(pos[0] as f32, pos[1] as f32), 10.0, Color::BLACK);
}

fn get_right_hand_pos(people: &PeopleDataRes) -> Option<[f64; 2]> {
    *people.first()?.keypoints.get(10)?
}

fn show_person_image(
    people: Res<PeopleDataRes>,
    mut images: ResMut<Assets<Image>>,
    debug_image: Res<PersonImageDebug>,
) {
    let Some(person) = people.first() else {
        return;
    };

    let Some(person_image) = &person.person_image else {
        return;
    };

    if let Some(image) = images.get_mut(&debug_image.handle) {
        *image = person_image.0.clone();
    }
}
