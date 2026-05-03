use std::collections::HashMap;
use bevy::asset::Handle;
use bevy::image::Image;
use bevy::prelude::{AssetServer, FromWorld, Resource, World};
use strum::{EnumCount, FromRepr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumCount, FromRepr)]
pub enum TextureType {
    Body,
    Shield,
    Zombie,
    Knight,
    Wizard,
    Elf,
    Robot,
    Sword,
    Vampire,
}

#[derive(Resource)]
pub struct TextureAssets {
    pub textures: HashMap<TextureType, Handle<Image>>,
}
impl FromWorld for TextureAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world
            .get_resource::<AssetServer>()
            .expect("AssetServer resource not found.");
        let mut textures = HashMap::new();
        textures.insert(TextureType::Body, asset_server.load("sprites/BODY_skeleton.png"));
        textures.insert(TextureType::Shield, asset_server.load("sprites/WEAPON_shield_cutout_body.png"));
        textures.insert(TextureType::Zombie, asset_server.load("sprites/zombie.png"));
        textures.insert(TextureType::Knight, asset_server.load("sprites/knight.png"));
        textures.insert(TextureType::Wizard, asset_server.load("sprites/wizard.png"));
        textures.insert(TextureType::Elf, asset_server.load("sprites/elf.png"));
        textures.insert(TextureType::Robot, asset_server.load("sprites/robot1.png"));
        textures.insert(TextureType::Sword, asset_server.load("sprites/sword.png"));
        textures.insert(TextureType::Vampire, asset_server.load("sprites/vampire.png"));
        Self { textures }
    }
}
