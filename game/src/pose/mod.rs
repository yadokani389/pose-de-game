use bevy::prelude::*;

use receive::*;
use visualize::*;

mod receive;
mod visualize;

pub struct PosePlugin;

impl Plugin for PosePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(())
            .init_resource::<PeopleDataRes>()
            .add_systems(Startup, create_image)
            .add_systems(Update, (receive_data, show_right_hand, show_person_image));
    }
}

#[derive(Debug, Clone)]
pub struct PersonData {
    pub keypoints: Vec<Option<[f64; 2]>>,
    pub right_hand_closed: Option<bool>,
    pub left_hand_closed: Option<bool>,
    pub person_image: Option<Image>,
}

type PeopleData = Vec<PersonData>;

#[derive(Resource, Default, Deref, DerefMut)]
pub struct PeopleDataRes(PeopleData);
