use bevy::asset::Handle;
use bevy::image::Image;
use bevy::prelude::{AssetServer, Resource};
use bevy_ecs::prelude::World;
use bevy_ecs::world::FromWorld;

#[derive(Resource)]
pub struct TextureAssets {
    pub body: Handle<Image>,
    pub shield: Handle<Image>,
}
impl FromWorld for TextureAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>()
            .expect("AssetServer resource not found.");
        Self{
            body: asset_server.load("BODY_skeleton.png"),
            shield: asset_server.load("WEAPON_shield_cutout_body.png")
        }
    }
}