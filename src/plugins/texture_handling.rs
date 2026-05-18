use std::collections::HashMap;
use bevy::asset::Handle;
use bevy::image::Image;
use bevy::prelude::{AssetServer, FromWorld, Resource, World};
use strum::{EnumCount, FromRepr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumCount, FromRepr)]
pub enum TextureType {
    //character body
    Body,
    Shield,
    //enemys
    Zombie,
    Knight,
    Wizard,
    Elf,
    Robot,
    Vampire,
    //weapons
    Sword,
    Rocket,
    Laser,
    //collectibles
    Magnet,
    HealthPack,
    AtomBomb,
    XPGem,
    // effects
    Spark,
    Smoke,
    Flame,
    Electric,
    Sparkle,
    ExplosionCore,
    MuzzleFlash,
    Particle,
    // ground
    GroundTile,
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
        textures.insert(TextureType::HealthPack, asset_server.load("collectibles/health_pack.png"));
        textures.insert(TextureType::AtomBomb, asset_server.load("collectibles/atom_bomb.png"));
        textures.insert(TextureType::Magnet, asset_server.load("collectibles/magnet.png"));
        textures.insert(TextureType::XPGem, asset_server.load("collectibles/xp_gem.png"));
        textures.insert(TextureType::Laser, asset_server.load("sprites/laser.png"));
        textures.insert(TextureType::Rocket, asset_server.load("sprites/rocket.png"));
        // effects
        textures.insert(TextureType::Spark, asset_server.load("effects/spark.png"));
        textures.insert(TextureType::Smoke, asset_server.load("effects/smoke.png"));
        textures.insert(TextureType::Flame, asset_server.load("effects/flame.png"));
        textures.insert(TextureType::Electric, asset_server.load("effects/electric.png"));
        textures.insert(TextureType::Sparkle, asset_server.load("effects/sparkle.png"));
        textures.insert(TextureType::ExplosionCore, asset_server.load("effects/explosion_core.png"));
        textures.insert(TextureType::MuzzleFlash, asset_server.load("effects/muzzle_flash.png"));
        textures.insert(TextureType::Particle, asset_server.load("effects/particle.png"));
        // ground
        textures.insert(TextureType::GroundTile, asset_server.load("textures/rpg/tiles/generic-rpg-tile01.png"));
        Self { textures }
    }
}
