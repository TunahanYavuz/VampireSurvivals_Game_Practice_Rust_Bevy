use bevy::prelude::*;
use crate::plugins::audio::GameAudioPlugin;
use crate::plugins::boss::BossPlugin;
use crate::plugins::config::ConfigPlugin;
use crate::plugins::enemy::EnemyPlugin;
use crate::plugins::game::GamePlugin;
use crate::plugins::game_state::GameState;
use crate::plugins::ground::GroundPlugin;
use crate::plugins::lobby::LobbyPlugin;
use crate::plugins::main_menu::MainMenuPlugin;
use crate::plugins::network::NetworkPlugin;
use crate::plugins::particle_effects::ParticlePlugin;
use crate::plugins::player::PlayerPlugin;
use crate::plugins::reinforcements::ReinforcementsPlugin;
use crate::plugins::score::ScorePlugin;
use crate::plugins::settings::SettingsPlugin;
use crate::plugins::timers::TimerPlugin;
use crate::plugins::weapons::{UpgradePlugin, WeaponEffectPlugin, WeaponPlugin};

mod plugins;

fn main() {
    App::new()
        // Oyun Plugin'leri
        .add_plugins((
                ConfigPlugin,
                NetworkPlugin,
                GamePlugin,
                ReinforcementsPlugin,
                PlayerPlugin,
                EnemyPlugin,
                BossPlugin,
                WeaponPlugin,
                ParticlePlugin,
                WeaponEffectPlugin,
                UpgradePlugin,
                ScorePlugin,
                GroundPlugin,
                GameAudioPlugin,
                TimerPlugin,
        ))
        .add_plugins((
            MainMenuPlugin,
                      LobbyPlugin,
                      SettingsPlugin,
        ))
        .init_state::<GameState>()

        .run();
}
