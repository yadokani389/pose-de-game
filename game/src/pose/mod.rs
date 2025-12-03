use bevy::prelude::*;
use image::ImageFormat;
use serde::Deserialize;
use serde_bytes::ByteBuf;

pub struct PosePlugin;

impl Plugin for PosePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(())
            .init_resource::<PeopleDataRes>()
            .add_systems(Update, receive_data);
    }
}

fn receive_data(
    socket: Res<super::UdpSocketResource>,
    mut people_data: ResMut<PeopleDataRes>,
    mut buffer: Local<UdpBuffer>,
) {
    let Ok(size) = socket.0.recv(&mut buffer) else {
        return;
    };

    match serde_cbor::from_slice::<PeoplePayload>(&buffer[..size]) {
        Ok(people) => {
            let converted: PeopleData = people
                .into_iter()
                .enumerate()
                .map(|(idx, person)| person.into_person_data(idx))
                .collect();

            **people_data = converted;
        }
        Err(e) => {
            error!("Failed to parse CBOR data: {e}");
        }
    }
}

#[derive(Deserialize, Debug)]
struct PersonPayload {
    pub keypoints: Vec<Option<[f64; 2]>>,
    pub right_hand_closed: Option<bool>,
    pub left_hand_closed: Option<bool>,
    #[serde(default)]
    pub person_png: Option<ByteBuf>,
}

#[derive(Debug, Clone)]
pub struct PersonImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PersonData {
    pub keypoints: Vec<Option<[f64; 2]>>,
    pub right_hand_closed: Option<bool>,
    pub left_hand_closed: Option<bool>,
    pub person_image: Option<PersonImage>,
}

type PeoplePayload = Vec<PersonPayload>;
type PeopleData = Vec<PersonData>;

#[derive(Resource, Default, Deref, DerefMut)]
pub struct PeopleDataRes(PeopleData);

#[derive(Deref, DerefMut)]
struct UdpBuffer(Vec<u8>);

impl Default for UdpBuffer {
    fn default() -> Self {
        Self(vec![0; 65536])
    }
}

impl PersonPayload {
    fn into_person_data(self, idx: usize) -> PersonData {
        let PersonPayload {
            keypoints,
            right_hand_closed,
            left_hand_closed,
            person_png,
        } = self;

        let person_image = match person_png {
            Some(bytes) => match decode_person_png(bytes.as_ref()) {
                Ok(image) => Some(image),
                Err(err) => {
                    error!("Failed to decode PNG for person {idx}: {err}");
                    None
                }
            },
            None => None,
        };

        PersonData {
            keypoints,
            right_hand_closed,
            left_hand_closed,
            person_image,
        }
    }
}

fn decode_person_png(bytes: &[u8]) -> Result<PersonImage, image::ImageError> {
    let image = image::load_from_memory_with_format(bytes, ImageFormat::Png)?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();

    Ok(PersonImage {
        width,
        height,
        rgba: rgba.into_raw(),
    })
}
