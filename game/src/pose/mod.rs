use bevy::prelude::*;
use serde::{Deserialize, Serialize};

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

    match serde_cbor::from_slice::<PeopleData>(&buffer[..size]) {
        Ok(people) => {
            **people_data = people;
        }
        Err(e) => {
            error!("Failed to parse CBOR data: {e}");
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PersonData {
    pub keypoints: Vec<Option<[f64; 2]>>,
    pub right_hand_closed: Option<bool>,
    pub left_hand_closed: Option<bool>,
}

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
