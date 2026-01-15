use bevy::asset::Handle;
use bevy::image::Image;
use bevy::prelude::{AssetServer, FromWorld, Resource, World};

#[derive(Resource)]
pub struct TextureAssets {
    pub body: Handle<Image>,
    pub shield: Handle<Image>,
    pub zombie: Handle<Image>,
    pub knight: Handle<Image>,
    pub wizard: Handle<Image>,
    pub elf: Handle<Image>,
    pub robot: Handle<Image>,
    pub sword: Handle<Image>,
}
impl FromWorld for TextureAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world
            .get_resource::<AssetServer>()
            .expect("AssetServer resource not found.");
        Self {
            body: asset_server.load("BODY_skeleton.png"),
            shield: asset_server.load("WEAPON_shield_cutout_body.png"),
            zombie: asset_server.load("zombie.png"),
            knight: asset_server.load("knight.png"),
            wizard: asset_server.load("wizard.png"),
            elf: asset_server.load("elf.png"),
            robot: asset_server.load("robot1.png"),
            sword: asset_server.load("sword.png"),
        }
    }
}
