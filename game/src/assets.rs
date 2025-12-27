use bevy::{
    asset::{embedded_asset, load_embedded_asset},
    prelude::*,
};

#[derive(Resource, Clone)]
pub struct UiFont(pub Handle<Font>);

pub struct EmbeddedAssetsPlugin;

impl Plugin for EmbeddedAssetsPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "../assets/fonts/NotoSansMonoCJK-VF.otf.ttc");
        let handle = load_embedded_asset!(app, "../assets/fonts/NotoSansMonoCJK-VF.otf.ttc");
        app.insert_resource(UiFont(handle));
    }
}
