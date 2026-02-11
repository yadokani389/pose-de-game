use bevy::{
    asset::{embedded_asset, load_embedded_asset},
    audio::AudioSource,
    prelude::*,
};

#[derive(Resource, Clone)]
pub struct UiFont(pub Handle<Font>);

#[derive(Resource, Clone)]
pub struct AppBgm(pub Handle<AudioSource>);

pub struct EmbeddedAssetsPlugin;

impl Plugin for EmbeddedAssetsPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "../assets/fonts/NotoSansMonoCJK-VF.otf.ttc");
        embedded_asset!(app, "../assets/bgm.mp3");

        let handle = load_embedded_asset!(app, "../assets/fonts/NotoSansMonoCJK-VF.otf.ttc");
        let bgm = load_embedded_asset!(app, "../assets/bgm.mp3");

        app.insert_resource(UiFont(handle));
        app.insert_resource(AppBgm(bgm));
    }
}
