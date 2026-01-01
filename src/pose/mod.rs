use anyhow::Result;
use bevy::prelude::*;

use runtime::infer_from_camera;

mod runtime;

use crate::args::Args;

pub struct PosePlugin;

impl Plugin for PosePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(())
            .init_resource::<PeopleDataRes>()
            .init_resource::<LatestFrameRes>()
            .add_systems(Update, (infer_from_camera,));
    }
}

pub fn setup_runtime(app: &mut App, args: &Args) -> Result<()> {
    runtime::setup_runtime(app, args)
}

pub fn list_cameras() {
    runtime::list_cameras();
}

#[derive(Debug, Clone)]
pub struct PersonData {
    pub keypoints: Vec<Option<[f64; 2]>>,
    pub person_image: Option<Image>,
}

type PeopleData = Vec<PersonData>;

#[derive(Debug, Clone)]
pub struct FrameImage {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

#[derive(Resource, Default)]
pub struct LatestFrameRes {
    pub frame: Option<FrameImage>,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct PeopleDataRes(PeopleData);
