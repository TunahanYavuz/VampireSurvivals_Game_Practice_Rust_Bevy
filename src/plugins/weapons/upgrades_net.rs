// Network handlers for upgrade flow.

use crate::plugins::game_state::GameState;
use crate::plugins::locale::Locale;
use crate::plugins::network::{
    NetworkRole, PendingClientUpgradeChoice, PendingUpgradeApplied, PendingUpgradeOptions, UpgradeMode,
};
use bevy::prelude::*;

use super::upgrade_screen::{populate_upgrade_table, populate_upgrade_table_entity, spawn_upgrade_table_ui};
use super::upgrades::{
    upgrade_option_desc, upgrade_option_name, UpgradeChoices, UpgradeOption, UpgradeSelectedEvent,
    WeaponType, WeaponTable,
};

// ─────────────────────────── Network Receive ────────────────────────────

pub fn receive_net_upgrade_options(
    role: Res<NetworkRole>,
    mut pending: ResMut<PendingUpgradeOptions>,
    mut upgrade_choices: ResMut<UpgradeChoices>,
    locale: Res<Locale>,
    mut commands: Commands,
    table: Query<Entity, With<WeaponTable>>,
    asset_server: Res<AssetServer>,
    upgrade_mode: Res<UpgradeMode>,
    mut next_state: ResMut<NextState<GameState>>,
    c_state: Res<State<GameState>>,
) {
    if *role != NetworkRole::Client {
        return;
    }
    let Some((opts_u8, for_player)) = pending.0.take() else {
        return;
    };

    let for_player_opt = if for_player == 255 {
        None
    } else {
        Some(for_player)
    };
    upgrade_choices.for_player = for_player_opt;

    let options: Vec<UpgradeOption> = opts_u8
        .iter()
        .filter_map(|&idx| WeaponType::from_u8(idx))
        .map(|wt| UpgradeOption {
            weapon_type: wt,
            name: upgrade_option_name(wt, &locale).to_string(),
            description: upgrade_option_desc(wt, &locale).to_string(),
            icon: None,
        })
        .collect();

    upgrade_choices.options = options.clone();
    upgrade_choices.waiting_for_choice = true;
    println!("Received upgrade options {:?} for player {:?}", upgrade_choices.options, upgrade_choices.for_player);
    if *upgrade_mode == UpgradeMode::Independent && *c_state.get() != GameState::RemoteUpgrade {
        next_state.set(GameState::RemoteUpgrade);
    }

    if table.iter().next().is_none() {
        let table_entity = spawn_upgrade_table_ui(&mut commands);
        populate_upgrade_table_entity(&mut commands, table_entity, &asset_server, &locale, &options);
        return;
    }

    populate_upgrade_table(&mut commands, &table, &asset_server, &locale, &options);
}

pub fn receive_net_upgrade_applied(
    role: Res<NetworkRole>,
    mut pending: ResMut<PendingUpgradeApplied>,
    mut upgrade_events: MessageWriter<UpgradeSelectedEvent>,
) {
    if *role != NetworkRole::Client {
        return;
    }
    let Some((weapon_idx, _for_player)) = pending.0.take() else {
        return;
    };
    if let Some(wt) = WeaponType::from_u8(weapon_idx) {
        upgrade_events.write(UpgradeSelectedEvent { weapon_type: wt });
    }
}

pub fn receive_client_upgrade_choice(
    role: Res<NetworkRole>,
    mut pending: ResMut<PendingClientUpgradeChoice>,
    mut upgrade_events: MessageWriter<UpgradeSelectedEvent>,
    mut upgrade_choices: ResMut<UpgradeChoices>,
) {
    if *role != NetworkRole::Host {
        return;
    }
    let Some(weapon_idx) = pending.0.take() else {
        return;
    };
    if let Some(wt) = WeaponType::from_u8(weapon_idx) {
        upgrade_choices.for_player = Some(1);
        upgrade_events.write(UpgradeSelectedEvent { weapon_type: wt });
    }
}
