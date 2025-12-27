use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use crate::{
    games::breakout::field::{CELL_SIZE, FIELD_WIDTH},
    pose::PeopleDataRes,
};

#[derive(Resource)]
pub(super) struct PersonImageHandle {
    handle: Handle<Image>,
}

#[derive(Component)]
pub(super) struct PersonImage;

pub fn create_image(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
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
        RenderAssetUsages::default(),
    );
    let image_handle = images.add(image);

    commands.spawn((
        Sprite {
            image: image_handle.clone(),
            flip_x: true,
            ..default()
        },
        Transform::from_xyz(0., 0., 15.),
        PersonImage,
    ));

    commands.insert_resource(PersonImageHandle {
        handle: image_handle,
    });
}

pub fn show_right_hand(people: Res<PeopleDataRes>, mut gizmos: Gizmos) {
    let Some(mut pos) = get_right_hand_pos(&people) else {
        return;
    };

    pos[0] -= 0.5;
    pos[1] -= 0.5;
    pos[0] *= -2. * FIELD_WIDTH as f64 * CELL_SIZE as f64;
    pos[1] *= -2. * FIELD_WIDTH as f64 * CELL_SIZE as f64;

    gizmos.circle_2d(Vec2::new(pos[0] as f32, pos[1] as f32), 10.0, Color::BLACK);
}

fn get_right_hand_pos(people: &PeopleDataRes) -> Option<[f64; 2]> {
    *people.first()?.keypoints.get(10)?
}

pub fn show_person_image(
    people: Res<PeopleDataRes>,
    mut images: ResMut<Assets<Image>>,
    mut sprite: Single<&mut Sprite, With<PersonImage>>,
    debug_image: Res<PersonImageHandle>,
) {
    if !people.is_changed() {
        return;
    }

    let Some(person) = people.first() else {
        return;
    };

    let Some(person_image) = &person.person_image else {
        return;
    };

    if let Some(image) = images.get_mut(&debug_image.handle) {
        let mut img = person_image.clone();
        dim_alpha(&mut img);
        *image = img;
        let size = image.size_f32() / (image.width() as f32) * 2. * FIELD_WIDTH as f32 * CELL_SIZE;
        if sprite.custom_size != Some(size) {
            sprite.custom_size.replace(size);
        }
    }
}

fn dim_alpha(image: &mut Image) {
    if image.texture_descriptor.format != TextureFormat::Rgba8UnormSrgb {
        return;
    }

    let width = image.texture_descriptor.size.width as usize;
    let height = image.texture_descriptor.size.height as usize;
    let stride = width * 4;

    let Some(layer) = image.data.as_mut() else {
        return;
    };
    let data: &mut [u8] = layer.as_mut_slice();

    for y in 0..height {
        let row_offset = y * stride;

        // Halve alpha
        for x in 0..width {
            let alpha_idx = row_offset + x * 4 + 3;
            data[alpha_idx] >>= 1;
        }
    }
}
