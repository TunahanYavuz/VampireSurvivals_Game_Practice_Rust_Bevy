//! Lobby screen: role selection (Host / Client), IP entry, and upgrade-mode choice.
//!
//! Flow
//! ────
//! 1. Player clicks "Host Game" **or** "Join Game".
//! 2. If Host → upgrade-mode buttons appear; "Start Hosting" begins the TCP listener.
//! 3. If Client → IP text-field appears; "Connect" opens the TCP connection.
//! 4. Once the background TCP handshake completes (`PendingConnection::ready`), both
//!    machines proceed to `GameState::Loading` (handled in `network::poll_pending_connection`).
//!
//! For **Solo** play the lobby can be bypassed: the "Solo" button transitions
//! directly to `GameState::Loading` without setting up any networking.

use bevy::{
    input::{
        keyboard::{Key, KeyboardInput},
        ButtonState,
    },
    prelude::*,
};
use bevy::ecs::relationship::RelatedSpawnerCommands;
use crate::plugins::game_state::GameState;
use crate::plugins::locale::Locale;
use crate::plugins::network::{
    start_client, start_host, NetworkRole, UpgradeMode,
};

pub struct LobbyPlugin;

impl Plugin for LobbyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LobbyData>()
            .add_systems(OnEnter(GameState::Lobby), setup_lobby)
            .add_systems(OnExit(GameState::Lobby), cleanup_lobby)
            .add_systems(
                Update,
                (
                    handle_lobby_buttons,
                    update_ip_text_input,
                    button_hover,
                    update_status_text,
                )
                    .run_if(in_state(GameState::Lobby)),
            );
    }
}

// ─────────────────────────── Resources / components ──────────────────────

/// Transient lobby state that survives only within `GameState::Lobby`.
#[derive(Resource, Default)]
pub struct LobbyData {
    /// IP address typed by the player when connecting as a client.
    pub ip_input: String,
    /// Which sub-screen the lobby is currently showing.
    pub phase: LobbyPhase,
    /// Status message shown near the bottom of the lobby.
    pub status: String,
}

#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum LobbyPhase {
    #[default]
    RoleSelect,
    HostReady,
    HostWaiting,
    ClientConnecting,
}

// ─────────────────────────── Marker components ───────────────────────────

#[derive(Component)]
struct LobbyUiRoot;

#[derive(Component)]
struct HostPanel;

#[derive(Component)]
struct ClientPanel;

#[derive(Component)]
struct StatusText;

#[derive(Component)]
struct IpInputText;

#[derive(Component, Clone, Copy)]
enum LobbyButton {
    Solo,
    BecomeHost,
    BecomeClient,
    ModeShared,
    ModeIndependent,
    StartHosting,
    Connect,
}

// ─────────────────────────── Setup ───────────────────────────────────────

fn setup_lobby(mut commands: Commands, asset_server: Res<AssetServer>, locale: Res<Locale>) {
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");

    commands
        .spawn((
            LobbyUiRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.08, 0.08, 0.12)),
        ))
        .with_children(|root| {
            // Title
            root.spawn((
                Text::new(locale.t("lobby_title")),
                TextFont {
                    font: font.clone(),
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(24.0)),
                    ..default()
                },
            ));

            // ── Role selection row ──
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(20.0),
                ..default()
            })
            .with_children(|row| {
                spawn_lobby_btn(row, locale.t("lobby_solo"), LobbyButton::Solo, &font);
                spawn_lobby_btn(row, locale.t("lobby_host"), LobbyButton::BecomeHost, &font);
                spawn_lobby_btn(row, locale.t("lobby_client"), LobbyButton::BecomeClient, &font);
            });

            // ── Host panel (hidden until role = Host) ──
            root.spawn((
                HostPanel,
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(10.0),
                    display: Display::None, // hidden by default
                    ..default()
                },
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new(locale.t("lobby_mode_label")),
                    TextFont {
                        font: font.clone(),
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.8, 0.8)),
                ));
                let mode_row = panel
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(16.0),
                        ..default()
                    })
                    .id();
                // We have to add children to the mode_row separately
                panel
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(16.0),
                        ..default()
                    })
                    .with_children(|row| {
                        spawn_lobby_btn(
                            row,
                            locale.t("lobby_mode_shared"),
                            LobbyButton::ModeShared,
                            &font,
                        );
                        spawn_lobby_btn(
                            row,
                            locale.t("lobby_mode_independent"),
                            LobbyButton::ModeIndependent,
                            &font,
                        );
                    });
                let _ = mode_row; // suppress unused warning
                spawn_lobby_btn(panel, locale.t("lobby_start_hosting"), LobbyButton::StartHosting, &font);
            });

            // ── Client panel (hidden until role = Client) ──
            root.spawn((
                ClientPanel,
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(10.0),
                    display: Display::None,
                    ..default()
                },
            ))
            .with_children(|panel| {
                panel.spawn((
                    IpInputText,
                    Text::new("192.168.1.1"),
                    TextFont {
                        font: font.clone(),
                        font_size: 28.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.2, 0.9, 0.4)),
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
                    Node {
                        padding: UiRect::all(Val::Px(8.0)),
                        min_width: Val::Px(260.0),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    Outline {
                        width: Val::Px(1.5),
                        offset: Val::Px(0.0),
                        color: Color::srgba(0.2, 0.9, 0.4, 0.6),
                    },
                ));
                spawn_lobby_btn(panel, locale.t("lobby_connect"), LobbyButton::Connect, &font);
            });

            // ── Status line ──
            root.spawn((
                StatusText,
                Text::new(""),
                TextFont {
                    font: font.clone(),
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.7, 0.3)),
                Node {
                    margin: UiRect::top(Val::Px(20.0)),
                    ..default()
                },
            ));
        });

    // Seed the IP input field default
    // (LobbyData is already init'd with empty string; we start with a hint)
}

fn spawn_lobby_btn(
    parent: &mut RelatedSpawnerCommands<ChildOf>,
    label: &str,
    btn_type: LobbyButton,
    font: &Handle<Font>,
) {
    parent
        .spawn((
            Button,
            btn_type,
            Node {
                width: Val::Px(220.0),
                height: Val::Px(54.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.22, 0.22, 0.3)),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

// ─────────────────────────── Button handler ──────────────────────────────

#[allow(clippy::too_many_arguments)]
fn handle_lobby_buttons(
    interactions: Query<(&Interaction, &LobbyButton), (Changed<Interaction>, With<Button>)>,
    mut lobby: ResMut<LobbyData>,
    mut role: ResMut<NetworkRole>,
    mut upgrade_mode: ResMut<UpgradeMode>,
    mut next_state: ResMut<NextState<GameState>>,
    mut commands: Commands,
    mut host_panel: Query<&mut Node, (With<HostPanel>, Without<ClientPanel>)>,
    mut client_panel: Query<&mut Node, (With<ClientPanel>, Without<HostPanel>)>,
) {
    for (interaction, btn) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match btn {
            LobbyButton::Solo => {
                *role = NetworkRole::Solo;
                next_state.set(GameState::Loading);
            }
            LobbyButton::BecomeHost => {
                *role = NetworkRole::Host;
                lobby.phase = LobbyPhase::HostReady;
                lobby.status = "Select upgrade mode, then start hosting.".to_string();
                if let Ok(mut n) = host_panel.single_mut() {
                    n.display = Display::Flex;
                }
                if let Ok(mut n) = client_panel.single_mut() {
                    n.display = Display::None;
                }
            }
            LobbyButton::BecomeClient => {
                *role = NetworkRole::Client;
                lobby.phase = LobbyPhase::RoleSelect;
                lobby.status = "Enter the host IP and click Connect.".to_string();
                if let Ok(mut n) = client_panel.single_mut() {
                    n.display = Display::Flex;
                }
                if let Ok(mut n) = host_panel.single_mut() {
                    n.display = Display::None;
                }
            }
            LobbyButton::ModeShared => {
                *upgrade_mode = UpgradeMode::Shared;
                lobby.status = "Mode B (Shared) selected.".to_string();
            }
            LobbyButton::ModeIndependent => {
                *upgrade_mode = UpgradeMode::Independent;
                lobby.status = "Mode A (Independent) selected.".to_string();
            }
            LobbyButton::StartHosting => {
                if lobby.phase != LobbyPhase::HostReady {
                    continue;
                }
                lobby.phase = LobbyPhase::HostWaiting;
                lobby.status = "Waiting for client to connect…".to_string();
                let pending = start_host();
                commands.insert_resource(pending);
            }
            LobbyButton::Connect => {
                if lobby.phase == LobbyPhase::ClientConnecting {
                    continue;
                }
                let ip = if lobby.ip_input.trim().is_empty() {
                    "127.0.0.1".to_string()
                } else {
                    lobby.ip_input.trim().to_string()
                };
                lobby.phase = LobbyPhase::ClientConnecting;
                lobby.status = format!("Connecting to {ip}…");
                let pending = start_client(ip);
                commands.insert_resource(pending);
            }
        }
    }
}

// ─────────────────────────── IP text input ───────────────────────────────

fn update_ip_text_input(
    mut lobby: ResMut<LobbyData>,
    mut key_events: MessageReader<KeyboardInput>,
    role: Res<NetworkRole>,
    mut ip_text: Query<&mut Text, With<IpInputText>>,
) {
    // Only active when the user has chosen the client role.
    if *role != NetworkRole::Client {
        key_events.clear();
        return;
    }

    for event in key_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        match &event.logical_key {
            Key::Character(s) => {
                // Allow digits and dots only for IP addresses.
                for ch in s.chars() {
                    if ch.is_ascii_digit() || ch == '.' {
                        lobby.ip_input.push(ch);
                    }
                }
            }
            Key::Backspace => {
                lobby.ip_input.pop();
            }
            Key::Space => {
                // Ignore spaces
            }
            _ => {}
        }
    }

    for mut text in ip_text.iter_mut() {
        let display = if lobby.ip_input.is_empty() {
            "Type host IP (e.g. 192.168.1.5)".to_string()
        } else {
            lobby.ip_input.clone()
        };
        text.0 = display;
    }
}

// ─────────────────────────── Status text ─────────────────────────────────

fn update_status_text(
    lobby: Res<LobbyData>,
    mut query: Query<&mut Text, With<StatusText>>,
) {
    for mut text in query.iter_mut() {
        text.0 = lobby.status.clone();
    }
}

// ─────────────────────────── Hover effect ────────────────────────────────

fn button_hover(
    mut query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<LobbyButton>)>,
) {
    for (interaction, mut color) in query.iter_mut() {
        match interaction {
            Interaction::Hovered => *color = BackgroundColor(Color::srgb(0.38, 0.38, 0.5)),
            Interaction::None => *color = BackgroundColor(Color::srgb(0.22, 0.22, 0.3)),
            Interaction::Pressed => {}
        }
    }
}

// ─────────────────────────── Cleanup ─────────────────────────────────────

fn cleanup_lobby(
    mut commands: Commands,
    roots: Query<Entity, With<LobbyUiRoot>>,
    mut lobby: ResMut<LobbyData>,
) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    // Reset lobby phase for next time.
    *lobby = LobbyData::default();
}
