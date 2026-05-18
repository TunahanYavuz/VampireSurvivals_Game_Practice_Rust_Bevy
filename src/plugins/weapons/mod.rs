// Weapons feature module: core logic, stats, effects, upgrades, and UI.

pub mod core;
pub mod effects;
pub mod stats;
pub mod upgrade_screen;
pub mod upgrades;
pub mod upgrades_net;

pub use core::WeaponPlugin;
pub use effects::{
    WeaponEffectPlugin, attach_trail_effect, raygun_spark_config, spawn_explosion_effect,
    spawn_muzzle_flash,
};
pub use stats::spawn_weapons_for_player;
pub use upgrades::{LevelUpEvent, UpgradePlugin, WeaponType};
