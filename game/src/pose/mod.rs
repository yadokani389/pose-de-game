use bevy::prelude::*;

use receive::*;
use visualize::*;

mod receive;
mod visualize;

use crate::args::Args;

pub struct PosePlugin;

impl Plugin for PosePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(())
            .init_resource::<PeopleDataRes>()
            .add_systems(Startup, create_image.run_if(show_person_enabled))
            .add_systems(
                Update,
                (
                    receive_data,
                    show_right_hand,
                    show_person_image.run_if(show_person_enabled),
                ),
            );
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

fn show_person_enabled(args: Res<Args>) -> bool {
    args.show_person
}
