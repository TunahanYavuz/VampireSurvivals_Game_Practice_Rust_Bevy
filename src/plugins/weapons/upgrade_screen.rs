// Upgrade selection UI systems and input handling.

use crate::plugins::audio::GameAudioEntity;
use crate::plugins::game_state::GameState;
use crate::plugins::locale::Locale;
use crate::plugins::network::{
    encode, NetOutbox, NetworkRole, NetworkedGameState, S2C, UpgradeMode,
};
use bevy::prelude::*;
use bevy::ui::Val::Auto;

use super::upgrades::{
    LevelUpEvent, UpgradeButton, UpgradeChoices, UpgradeOption, UpgradeSelectedEvent, WeaponTable,
};

// ─────────────────────────── UI Setup ───────────────────────────────────

#[derive(Component)]
pub struct RemoteUpgradeNotice;

pub fn spawn_upgrade_table_ui(commands: &mut Commands) -> Entity {
    commands.spawn((
        WeaponTable,
        Node {
            width: Val::Percent(40.0),
            height: Val::Percent(50.0),
            margin: UiRect {
                left: Auto,
                right: Auto,
                top: Auto,
                bottom: Auto,
            },
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_wrap: FlexWrap::Wrap,
            ..default()
        },
        BackgroundColor(Color::srgba(0.7137, 0.7137, 0.7137, 0.92)),
    )).id()
}

pub fn spawn_remote_upgrade_notice(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    table: Query<Entity, With<WeaponTable>>,
    notice: Query<Entity, With<RemoteUpgradeNotice>>,
) {
    if table.iter().next().is_some() || notice.iter().next().is_some() {
        return;
    }
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");
    commands.spawn((
        RemoteUpgradeNotice,
        Node {
            width: Val::Percent(40.0),
            height: Val::Percent(20.0),
            margin: UiRect::all(Val::Auto),
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.85)),
    ))
    .with_children(|parent| {
        parent.spawn((
            Text::new("Diger oyuncu silah yukseltmesi yapiyor"),
            TextFont {
                font,
                font_size: 22.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ));
    });
}

pub fn cleanup_upgrade_ui_on_choice(
    table: Query<Entity, With<WeaponTable>>,
    notice: Query<Entity, With<RemoteUpgradeNotice>>,
    audio_entity: Query<Entity, With<GameAudioEntity>>,
    mut commands: Commands,
) {
    for table_entity in table.iter() {
        commands.entity(table_entity).try_despawn();
    }
    for notice_entity in notice.iter() {
        commands.entity(notice_entity).despawn();
    }
    for audio_entity in audio_entity.iter() {
        commands.entity(audio_entity).despawn();
    }
}

pub(crate) fn populate_upgrade_table(
    commands: &mut Commands,
    table: &Query<Entity, With<WeaponTable>>,
    asset_server: &Res<AssetServer>,
    locale: &Locale,
    options: &[UpgradeOption],
) {
    let Ok(table_entity) = table.single() else {
        return;
    };
    populate_upgrade_table_entity(commands, table_entity, asset_server, locale, options);
}

pub(crate) fn populate_upgrade_table_entity(
    commands: &mut Commands,
    table_entity: Entity,
    asset_server: &Res<AssetServer>,
    locale: &Locale,
    options: &[UpgradeOption],
) {
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");
    let options_len = options.len() as f32;

    for (i, option) in options.iter().enumerate() {
        commands.entity(table_entity).with_children(|parent| {
            parent
                .spawn((
                    Button::default(),
                    UpgradeButton(option.weapon_type),
                    Text::new(format!(
                        "{} {} {} - {}",
                        locale.t("option_prefix"),
                        i + 1,
                        option.name,
                        option.description
                    )),
                    TextFont {
                        font: font.clone(),
                        font_size: 20.0,
                        ..default()
                    },
                    Node {
                        height: Val::Percent(100.0 / options_len),
                        width: Val::Percent(100.0),
                        ..default()
                    },
                    Outline {
                        width: Val::Px(2.0),
                        offset: Val::Px(0.0),
                        color: Color::srgba(0.0, 0.1, 0.2, 0.8),
                    },
                ))
                .with_children(|parent| {
                    if let Some(icon_handle) = &option.icon {
                        parent.spawn((
                            ImageNode::new(icon_handle.clone()),
                            Node {
                                left: Val::Percent(75.0),
                                top: Val::Percent(45.0),
                                width: Val::Px(64.0),
                                height: Val::Px(64.0),
                                margin: UiRect::right(Val::Px(10.0)),
                                border_radius: BorderRadius::all(Val::Px(10.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                            Outline {
                                width: Val::Px(1.0),
                                offset: Val::Px(0.0),
                                color: Color::srgba(0.0, 0.0, 0.0, 0.8),
                            },
                        ));
                    }
                });
        });
    }
}

// ─────────────────────────── UI Flow ────────────────────────────────────

pub fn show_upgrade_choices_on_level_up(
    mut level_up_events: MessageReader<LevelUpEvent>,
    mut upgrade_choices: ResMut<UpgradeChoices>,
    mut commands: Commands,
    table: Query<Entity, With<WeaponTable>>,
    asset_server: Res<AssetServer>,
    locale: Res<Locale>,
    role: Res<NetworkRole>,
    upgrade_mode: Res<UpgradeMode>,
    outbox: Option<Res<NetOutbox>>,
) {
    for event in level_up_events.read() {
        let options = upgrade_choices.generate_random_options(&locale);

        let for_player: Option<u8> = match *upgrade_mode {
            UpgradeMode::Shared => None,
            UpgradeMode::Independent => Some(event.player_index),
        };
        upgrade_choices.for_player = for_player;

        if *role == NetworkRole::Host {
            if let Some(ref outbox) = outbox {
                let opts: Vec<u8> = options.iter().map(|o| o.weapon_type.to_u8()).collect();
                let fp = for_player.unwrap_or(255);

                let should_notify_client = match *upgrade_mode {
                    UpgradeMode::Shared => true,
                    UpgradeMode::Independent => event.player_index == 1,
                };

                if should_notify_client {
                    if let Ok(frame) = encode(&S2C::UpgradeOptions { opts, for_player: fp }) {
                        let _ = outbox.0.send(frame);
                    }
                }

                match *upgrade_mode {
                    UpgradeMode::Shared => {
                        if let Ok(frame) = encode(&S2C::StateChange(NetworkedGameState::HostUpgrade)) {
                            let _ = outbox.0.send(frame);
                        }
                    }
                    UpgradeMode::Independent => {
                        let target_state = if event.player_index == 0 {
                            NetworkedGameState::HostUpgrade
                        } else {
                            NetworkedGameState::RemoteUpgrade
                        };
                        if let Ok(frame) = encode(&S2C::StateChange(target_state)) {
                            let _ = outbox.0.send(frame);
                        }
                    }
                }
            }
        }

        if *role == NetworkRole::Client && *upgrade_mode == UpgradeMode::Shared {
            return;
        }

        if *role == NetworkRole::Host
            && *upgrade_mode == UpgradeMode::Independent
            && event.player_index == 1
        {
            return;
        }

        let table_entity = if table.iter().next().is_none() {
            Some(spawn_upgrade_table_ui(&mut commands))
        } else {
            None
        };

        if let Some(table_entity) = table_entity {
            populate_upgrade_table_entity(&mut commands, table_entity, &asset_server, &locale, &options);
        } else {
            populate_upgrade_table(&mut commands, &table, &asset_server, &locale, &options);
        }
    }
}

pub fn enter_host_upgrade_ui(
    role: Res<NetworkRole>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    table: Query<Entity, With<WeaponTable>>,
    notice: Query<Entity, With<RemoteUpgradeNotice>>,
) {
    if *role == NetworkRole::Host || *role == NetworkRole::Solo {
        if table.iter().next().is_none() {
            let _ = spawn_upgrade_table_ui(&mut commands);
        }
    } else {
        spawn_remote_upgrade_notice(commands, asset_server, table, notice);
    }
}

pub fn enter_remote_upgrade_ui(
    role: Res<NetworkRole>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    table: Query<Entity, With<WeaponTable>>,
    notice: Query<Entity, With<RemoteUpgradeNotice>>,
) {
    if *role == NetworkRole::Client {
        if table.iter().next().is_none() {
            let _ = spawn_upgrade_table_ui(&mut commands);
        }
    } else {
        spawn_remote_upgrade_notice(commands, asset_server, table, notice);
    }
}

pub fn handle_upgrade_input(
    interaction_q: Query<(&Interaction, &UpgradeButton), (Changed<Interaction>, With<Button>)>,
    mut upgrade_events: MessageWriter<UpgradeSelectedEvent>,
    role: Res<NetworkRole>,
    upgrade_choices: Res<UpgradeChoices>,
    outbox: Option<Res<NetOutbox>>,
    game_mode: Res<UpgradeMode>,
    state: Res<State<GameState>>,
) {
    if *game_mode == UpgradeMode::Shared && *role == NetworkRole::Client {
        return;
    }
    if *state.get() == GameState::RemoteUpgrade && *role == NetworkRole::Host {
        return;
    }
    for (interaction, upgrade_button) in interaction_q.iter() {
        if *interaction == Interaction::Pressed {
            if *role == NetworkRole::Client {
                if let (Some(outbox), Some(1)) = (&outbox, upgrade_choices.for_player) {
                    use crate::plugins::network::C2S;
                    if let Ok(frame) = encode(&C2S::UpgradeChosen(upgrade_button.0.to_u8())) {
                        let _ = outbox.0.send(frame);
                    }
                    upgrade_events.write(UpgradeSelectedEvent {
                        weapon_type: upgrade_button.0,
                    });
                    return;
                }
            }

            upgrade_events.write(UpgradeSelectedEvent {
                weapon_type: upgrade_button.0,
            });
        }
    }
}
