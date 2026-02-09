use bevy::prelude::*;
use crate::plugins::audio::GameAudioPlugin;
use crate::plugins::config::ConfigPlugin;
use crate::plugins::enemy::EnemyPlugin;
use crate::plugins::game::GamePlugin;
use crate::plugins::game_state::GameState;
use crate::plugins::ground::GroundPlugin;
use crate::plugins::main_menu::MainMenuPlugin;
use crate::plugins::particle_effects::ParticlePlugin;
use crate::plugins::player::PlayerPlugin;
use crate::plugins::reinforcements::ReinforcementsPlugin;
use crate::plugins::rapier_effects::RapierEffectsPlugin;
use crate::plugins::score::ScorePlugin;
use crate::plugins::timers::TimerPlugin;
use crate::plugins::weapon_effects::WeaponEffectPlugin;
use crate::plugins::weapon_upgrade::UpgradePlugin;
use crate::plugins::weapons::WeaponPlugin;

mod plugins;

fn main() {
    App::new()
        // Oyun Plugin'leri
        .add_plugins((
                ConfigPlugin,
                GamePlugin,
                RapierEffectsPlugin, // Rapier physics + efektler
                ReinforcementsPlugin,
                PlayerPlugin,
                EnemyPlugin,
                WeaponPlugin,
                ParticlePlugin,
                WeaponEffectPlugin,
                UpgradePlugin,
                ScorePlugin,
                GroundPlugin,
                GameAudioPlugin,
                TimerPlugin,
                MainMenuPlugin,
        ))
        .init_state::<GameState>()

        .run();
}

