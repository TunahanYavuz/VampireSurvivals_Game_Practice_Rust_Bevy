//! LAN networking for the game.
//!
//! Architecture
//! ────────────
//! One machine is the **Host** (authoritative simulation, Player 1 / WASD).
//! The other is the **Client** (thin renderer + input forwarder, Player 2 / Arrow keys).
//!
//! Transport
//! ─────────
//! Plain TCP with length-prefixed frames (`u32 LE` + bincode payload).
//! Background reader/writer threads communicate with Bevy via `mpsc` channels.
//! All Bevy systems check for the optional `NetInbox`/`NetOutbox` resources so
//! the game works identically in `Solo` mode (no resources → systems are no-ops).
//!
//! Upgrade modes
//! ─────────────
//! • **Mode B – Shared** (default): any level-up pauses BOTH machines; the chosen
//!   upgrade is applied to every player's weapon set.
//! • **Mode A – Independent**: each player upgrades their own weapons separately.
//!   P1 levels up on the host machine only; P2's upgrade UI appears on the client.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use crate::plugins::game_state::GameState;

/// TCP port the host listens on.
pub const NET_PORT: u16 = 7777;

// Tüm ağ objelerinin benzersiz bir ID' si olmalı ki Host ve Client aynı obje olduğunu anlasın
pub type NetId = u32;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TransformSnapshot {
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EnemySnapshot {
    pub net_id: NetId,
    pub enemy_type: u8, // Görsel tip
    pub transform: TransformSnapshot,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PlayerSnapshot {
    pub stat: PlayerStat,
    pub transform: TransformSnapshot,
}

// Host' un her tick' te göndereceği ana paket
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct StatSnapshotMsg {
    pub p1: PlayerSnapshot,
    pub p2: PlayerSnapshot,
    pub enemies: Vec<EnemySnapshot>,
}

#[derive(Component)]
pub struct NetworkIdentity(pub NetId);

use std::collections::HashMap;
use std::future::pending;
use bevy::camera::visibility::{NoAutoAabb, NoFrustumCulling};
use crate::plugins::game::Atlases;
use crate::plugins::player::Player;
use crate::plugins::texture_handling::{TextureAssets, TextureType};
use crate::plugins::weapon_stats::{spawn_flame_weapon, spawn_lazer_weapon, spawn_rocket_weapon};

#[derive(Resource, Default)]
pub struct LocalNetworkMapping(pub HashMap<NetId, Entity>);
#[derive(Resource, Default)]
pub struct NetIdGenerator(pub u32);

// ─────────────────────────── Message types ───────────────────────────────

/// Messages the client sends to the host every frame / on events.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum C2S {
    /// Client P2's keyboard state this frame.
    PlayerInput(InputState),
    /// Client's P2 chose an upgrade; payload is the `WeaponType` index (0-4).
    UpgradeChosen(u8),
}

/// Messages the host sends to the client every frame / on events.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum S2C {
    /// Both players' current stats — sent every frame by the host.
    StatSnapshot(StatSnapshotMsg),
    /// A `GameState` transition the client must mirror.
    StateChange(NetworkedGameState),
    /// Upgrade options the client should present.
    /// `opts`: up to 3 `WeaponType` indices; `for_player`: 0=P1, 1=P2, 255=both.
    UpgradeOptions { opts: Vec<u8>, for_player: u8 },
    /// An upgrade was applied on the host; client must apply it too.
    /// `weapon_type`: `WeaponType` index; `for_player`: 0=P1, 1=P2, 255=both.
    UpgradeApplied { weapon_type: u8, for_player: u8 },
    /// The upgrade mode chosen in the lobby (0 = Shared, 1 = Independent).
    UpgradeMode(u8),
}

/// A compact representation of `GameState` that can be sent over the network.
/// Only the states that need to be mirrored on the client are included.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkedGameState {
    Playing,
    UpgradeSelection,
    GameOver,
}

impl From<NetworkedGameState> for GameState {
    fn from(s: NetworkedGameState) -> GameState {
        match s {
            NetworkedGameState::Playing => GameState::Playing,
            NetworkedGameState::UpgradeSelection => GameState::UpgradeSelection,
            NetworkedGameState::GameOver => GameState::GameOver,
        }
    }
}

/// The per-frame keyboard input the client sends to the host for P2.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct InputState {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub collect_magnet: bool,
}

/// Per-player stat data packed into every snapshot frame.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PlayerStat {
    pub health: u32,
    pub xp: f32,
    pub level: i32,
    pub xp_to_next_level: f32,
    pub score: u32,
}

// ──────────────────────────── Upgrade mode ───────────────────────────────

/// Upgrade mode selected by the host in the lobby.
#[derive(Serialize, Deserialize, Resource, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum UpgradeMode {
    /// Mode B – one shared upgrade choice is applied to both players simultaneously.
    #[default]
    Shared,
    /// Mode A – each player upgrades their own character independently.
    Independent,
}

impl UpgradeMode {
    pub fn to_u8(self) -> u8 {
        match self {
            UpgradeMode::Shared => 0,
            UpgradeMode::Independent => 1,
        }
    }
    pub fn from_u8(v: u8) -> Self {
        if v == 1 {
            UpgradeMode::Independent
        } else {
            UpgradeMode::Shared
        }
    }
}

// ──────────────────────────── Network role ───────────────────────────────

/// Whether this app instance is running as host, client, or standalone.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NetworkRole {
    /// No networking — two local players on one machine (original behaviour).
    #[default]
    Solo,
    Host,
    Client,
}

// ─────────────────────────── Bevy resources ──────────────────────────────

/// Sender end: game systems queue outgoing byte frames here.
#[derive(Resource)]
pub struct NetOutbox(pub Sender<Vec<u8>>);

/// Receiver end: the reader thread pushes incoming byte frames here each frame.
#[derive(Resource)]
pub struct NetInbox(pub Mutex<Receiver<Vec<u8>>>);

/// The most recent P2 input received from the client.
/// Only meaningful on the host.
#[derive(Resource, Default)]
pub struct RemoteInput(pub InputState);

/// Set while a background TCP connection attempt is still in progress.
#[derive(Resource)]
pub struct PendingConnection {
    /// Becomes `true` when the connection is established.
    pub ready: Arc<AtomicBool>,
    /// Once `ready`, contains the channel pair to hand to Bevy.
    pub channels: Arc<Mutex<Option<(Sender<Vec<u8>>, Receiver<Vec<u8>>)>>>,
}

// ────────────── Pending-message resources (populated by drain_inbox) ─────

/// Latest stat snapshot from the host; consumed by `player.rs`.
#[derive(Resource, Default)]
pub struct PendingStatSnapshot(pub Option<StatSnapshotMsg>);

/// Upgrade options pushed by the host; consumed by `weapon_upgrade.rs`.
/// `(weapon_type_indices, for_player)`.
#[derive(Resource, Default)]
pub struct PendingUpgradeOptions(pub Option<(Vec<u8>, u8)>);

/// Upgrade-applied notification from the host; consumed by `weapon_upgrade.rs`.
/// `(weapon_type_index, for_player)`.
#[derive(Resource, Default)]
pub struct PendingUpgradeApplied(pub Option<(u8, u8)>);

/// Upgrade choice received from the client (P2); consumed by `weapon_upgrade.rs`.
/// Only populated on the host. Value is a `WeaponType` index.
#[derive(Resource, Default)]
pub struct PendingClientUpgradeChoice(pub Option<u8>);

/// Game-state change received from the host; applied by `apply_pending_state`.
#[derive(Resource, Default)]
pub struct PendingStateChange(pub Option<NetworkedGameState>);

// ─────────────────────────── Plugin ──────────────────────────────────────

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NetworkRole>()
            .init_resource::<UpgradeMode>()
            .init_resource::<RemoteInput>()
            .init_resource::<PendingStatSnapshot>()
            .init_resource::<PendingUpgradeOptions>()
            .init_resource::<PendingUpgradeApplied>()
            .init_resource::<PendingClientUpgradeChoice>()
            .init_resource::<PendingStateChange>()
            .init_resource::<NetIdGenerator>()
            .init_resource::<LocalNetworkMapping>()
            .add_systems(
                Update,
                (
                    host_send_snapshot_system,
                    client_sync_enemies_system,
                    appy_weapon_visual_system,
                    poll_pending_connection,
                    drain_inbox,
                    apply_pending_state,
                ),
            );
    }
}


fn host_send_snapshot_system(
    players: Query<(&Transform, &Player)>,
    role: Res<NetworkRole>,
    outbox: Option<Res<NetOutbox>>,
    enemy_query: Query<(&NetworkIdentity, &Transform)>,
){
    if *role != NetworkRole::Host {return;}
    let Some(outbox) = outbox else { return; };
    let mut enemies_snap = Vec::new();
    for (net_id, transform) in enemy_query.iter() {
        enemies_snap.push(EnemySnapshot {
            net_id: net_id.0,
            enemy_type: 0,
            transform: TransformSnapshot {
                x: transform.translation.x,
                y: transform.translation.y,
                rotation: transform.rotation.z,
            },
        });
    }
    let mut p1_snap = PlayerSnapshot::default();
    let mut p2_snap = PlayerSnapshot::default();

    for (transform, player) in players.iter() {
        let snap = PlayerSnapshot {
            stat: PlayerStat {
                health: player.health,
                xp: player.xp,
                level: player.level,
                xp_to_next_level: player.xp_to_next_level,
                score: player.score,
            },
            transform: TransformSnapshot {
                x: transform.translation.x,
                y: transform.translation.y,
                rotation: transform.rotation.z,
            },
        };
        if player.player_index == 0 {
            p1_snap = snap;
        } else if player.player_index == 1 {
            p2_snap = snap;
        }
    }


    let snapshot = StatSnapshotMsg {
        p1: p1_snap,
        p2: p2_snap,
        enemies: enemies_snap,
    };
    if let Ok(frame) = encode(&S2C::StatSnapshot(snapshot)) {
        let _ = outbox.0.send(frame);
    }
}

fn client_sync_enemies_system(
    role: Res<NetworkRole>,
    mut pending_snap: ResMut<PendingStatSnapshot>,
    mut mapping: ResMut<LocalNetworkMapping>,
    mut commands: Commands,
    mut transform_query: Query<&mut Transform>,
    atlases: Res<Atlases>,
    textures: Res<TextureAssets>,

){
    if *role != NetworkRole::Client {return;}
    if let Some(snap) = pending_snap.0.take(){

        let mut alive_host_ids = std::collections::HashSet::new();
        for enemy in snap.enemies {
            alive_host_ids.insert(enemy.net_id);

            if let Some(&local_entity) = mapping.0.get(&enemy.net_id) {
                // Düşman zaten var transform güncelle
                if let Ok(mut transform) = transform_query.get_mut(local_entity) {
                    transform.translation.x = enemy.transform.x;
                    transform.translation.y = enemy.transform.y;
                    transform.rotation.z = enemy.transform.rotation;
                }
            }else {
                let body_atlas = atlases.body.as_ref().unwrap().clone();
                //Yeni düşman oluştur
                let new_entity = commands.spawn((
                    Sprite::from_atlas_image(
                        textures.textures.get(&TextureType::Zombie).unwrap().clone(),
                        TextureAtlas {
                            layout: body_atlas,
                            index: 15,
                        },
                    ),
                    Transform::from_xyz(enemy.transform.x, enemy.transform.y, enemy.transform.y),
                    NoFrustumCulling,
                    NoAutoAabb,
                )
                ).id();
                mapping.0.insert(enemy.net_id, new_entity);
            }
        }

        // Hostta olmayan düşmanları sil
        mapping.0.retain(|&net_id, &mut local_entity| {
            if !alive_host_ids.contains(&net_id) {
                commands.entity(local_entity).despawn();
                commands.entity(local_entity).queue_handled(|entity: EntityWorldMut| -> Result {
                   entity.despawn();
                    Ok(())
                }, bevy::ecs::error::warn);
                false
            }else { true }
        })
    }
}

fn appy_weapon_visual_system(
    mut commands: Commands,
    mut pending_upgrade: ResMut<PendingUpgradeApplied>,
    players: Query<(Entity, &Player)>,
){
    let Some((weapon_type, for_player_id)) = pending_upgrade.0.take() else { return; };
    let mut target_entity = None;
    for (entity, player) in players.iter() {
        if player.player_index == for_player_id {
            target_entity = Some(entity);
            break;
        }
    }

    let Some(p1) = target_entity else { return; };

    match weapon_type {
        1 => spawn_rocket_weapon(
            &mut commands,
            p1,
        ),
        2 => spawn_lazer_weapon(
            &mut commands,
            p1,
        ),
        _ => {}
    }
}


// ────────────────────────── TCP framing helpers ───────────────────────────

/// Serialise `msg` into a `u32-LE length` + bincode payload frame.
pub fn encode<T: Serialize>(msg: &T) -> Result<Vec<u8>, bincode::Error> {
    let payload = bincode::serialize(msg)?;
    let len = payload.len() as u32;
    let mut frame = len.to_le_bytes().to_vec();
    frame.extend(payload);
    Ok(frame)
}

/// Read exactly `n` bytes from the stream (blocking).
fn read_exact(stream: &mut TcpStream, n: usize) -> std::io::Result<Vec<u8>> {
    let mut buf = vec![0u8; n];
    stream.read_exact(&mut buf)?;
    Ok(buf)
}

// ─────────────────────────── Background threads ──────────────────────────

/// Spawn a reader and a writer background thread for an established TCP stream.
///
/// Returns `(outbox_tx, inbox_rx)`:
/// - Push encoded frames into `outbox_tx` to send them.
/// - Pull received frames from `inbox_rx`.
fn spawn_net_threads(stream: TcpStream) -> (Sender<Vec<u8>>, Receiver<Vec<u8>>) {
    let (inbox_tx, inbox_rx) = mpsc::channel::<Vec<u8>>();
    let (outbox_tx, outbox_rx) = mpsc::channel::<Vec<u8>>();

    let mut read_stream = stream.try_clone().expect("TcpStream clone failed");
    let mut write_stream = stream;

    // Reader: length-prefix decode → push to inbox_tx
    thread::spawn(move || loop {
        let len_bytes = match read_exact(&mut read_stream, 4) {
            Ok(b) => b,
            Err(_) => break,
        };
        let len =
            u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;
        // Sanity guard: skip absurdly large frames (> 1 MiB)
        if len == 0 || len > 1_048_576 {
            break;
        }
        match read_exact(&mut read_stream, len) {
            Ok(payload) => {
                if inbox_tx.send(payload).is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    });

    // Writer: drain outbox_rx → send frames
    thread::spawn(move || {
        while let Ok(frame) = outbox_rx.recv() {
            if write_stream.write_all(&frame).is_err() {
                break;
            }
        }
    });

    (outbox_tx, inbox_rx)
}

// ────────────────────────── Host / Client startup ────────────────────────

/// Start a TCP listener in a background thread and wait for exactly one client.
/// Returns a `PendingConnection`; poll `ready` in a Bevy system.
pub fn start_host() -> PendingConnection {
    let ready = Arc::new(AtomicBool::new(false));
    let channels = Arc::new(Mutex::new(
        None::<(Sender<Vec<u8>>, Receiver<Vec<u8>>)>,
    ));

    let ready2 = ready.clone();
    let channels2 = channels.clone();

    thread::spawn(move || {
        let listener = match TcpListener::bind(format!("0.0.0.0:{NET_PORT}")) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[host] bind failed: {e}");
                return;
            }
        };
        println!("[host] listening on :{NET_PORT}");
        match listener.accept() {
            Ok((stream, addr)) => {
                println!("[host] client connected from {addr}");
                let (tx, rx) = spawn_net_threads(stream);
                *channels2.lock().unwrap() = Some((tx, rx));
                ready2.store(true, Ordering::Release);
            }
            Err(e) => eprintln!("[host] accept error: {e}"),
        }
    });

    PendingConnection { ready, channels }
}

/// Connect to `host_ip` in a background thread (retries up to 10 times).
/// Returns a `PendingConnection`; poll `ready` in a Bevy system.
pub fn start_client(host_ip: String) -> PendingConnection {
    let ready = Arc::new(AtomicBool::new(false));
    let channels = Arc::new(Mutex::new(
        None::<(Sender<Vec<u8>>, Receiver<Vec<u8>>)>,
    ));

    let ready2 = ready.clone();
    let channels2 = channels.clone();

    thread::spawn(move || {
        let addr = format!("{host_ip}:{NET_PORT}");
        println!("[client] connecting to {addr}");
        for attempt in 0..10u32 {
            match TcpStream::connect(&addr) {
                Ok(stream) => {
                    println!("[client] connected to host");
                    let (tx, rx) = spawn_net_threads(stream);
                    *channels2.lock().unwrap() = Some((tx, rx));
                    ready2.store(true, Ordering::Release);
                    return;
                }
                Err(e) => {
                    if attempt < 9 {
                        thread::sleep(Duration::from_millis(500));
                    } else {
                        eprintln!("[client] connect failed after {attempt} attempts: {e}");
                    }
                }
            }
        }
    });

    PendingConnection { ready, channels }
}

// ─────────────────────────── Bevy systems ────────────────────────────────

/// Poll `PendingConnection` and, when ready, promote to `NetInbox`/`NetOutbox`.
///
/// The host immediately sends the chosen `UpgradeMode` to the client and
/// transitions both machines to `GameState::Loading`.
fn poll_pending_connection(
    pending: Option<ResMut<PendingConnection>>,
    mut commands: Commands,
    role: Res<NetworkRole>,
    upgrade_mode: Res<UpgradeMode>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Some(mut pending) = pending else {
        return;
    };
    if !pending.ready.load(Ordering::Acquire) {
        return;
    }
    let Some((tx, rx)) = pending.channels.lock().unwrap().take() else {
        return;
    };

    let tx2 = tx.clone();
    commands.insert_resource(NetOutbox(tx));
    commands.insert_resource(NetInbox(Mutex::new(rx)));
    commands.remove_resource::<PendingConnection>();

    match *role {
        NetworkRole::Host => {
            // Send the upgrade mode to the client before the game starts.
            if let Ok(frame) = encode(&S2C::UpgradeMode(upgrade_mode.to_u8())) {
                let _ = tx2.send(frame);
            }
            next_state.set(GameState::Loading);
        }
        NetworkRole::Client => {
            // Upgrade mode arrives asynchronously; transition after it lands.
            // The client transitions to Loading immediately; the upgrade mode
            // resource is updated in `drain_inbox` as soon as the message arrives.
            next_state.set(GameState::Loading);
        }
        NetworkRole::Solo => {}
    }
}

/// Drain the `NetInbox` and populate the appropriate `Pending*` resources.
///
/// The actual application of each pending value happens in dedicated systems
/// inside the relevant plugins, keeping concerns separated.
fn drain_inbox(
    inbox: Option<Res<NetInbox>>,
    role: Res<NetworkRole>,
    mut remote_input: ResMut<RemoteInput>,
    mut upgrade_mode: ResMut<UpgradeMode>,
    mut pending_snap: ResMut<PendingStatSnapshot>,
    mut pending_opts: ResMut<PendingUpgradeOptions>,
    mut pending_applied: ResMut<PendingUpgradeApplied>,
    mut pending_client_choice: ResMut<PendingClientUpgradeChoice>,
    mut pending_state: ResMut<PendingStateChange>,
) {
    let Some(inbox) = inbox else {
        return;
    };
    let rx = inbox.0.lock().unwrap();

    while let Ok(bytes) = rx.try_recv() {
        match *role {
            NetworkRole::Host => {
                if let Ok(msg) = bincode::deserialize::<C2S>(&bytes) {
                    match msg {
                        C2S::PlayerInput(input) => {
                            remote_input.0 = input;
                        }
                        C2S::UpgradeChosen(idx) => {
                            pending_client_choice.0 = Some(idx);
                        }
                    }
                }
            }
            NetworkRole::Client => {
                if let Ok(msg) = bincode::deserialize::<S2C>(&bytes) {
                    match msg {
                        S2C::UpgradeMode(mode_byte) => {
                            *upgrade_mode = UpgradeMode::from_u8(mode_byte);
                        }
                        S2C::StatSnapshot(snap) => {
                            pending_snap.0 = Some(snap);
                        }
                        S2C::StateChange(new_state) => {
                            pending_state.0 = Some(new_state);
                        }
                        S2C::UpgradeOptions { opts, for_player } => {
                            pending_opts.0 = Some((opts, for_player));
                        }
                        S2C::UpgradeApplied {
                            weapon_type,
                            for_player,
                        } => {
                            pending_applied.0 = Some((weapon_type, for_player));
                        }
                    }
                }
            }
            NetworkRole::Solo => {}
        }
    }
}

/// Apply a pending game-state change (received from the host) on the client.
fn apply_pending_state(
    role: Res<NetworkRole>,
    mut pending: ResMut<PendingStateChange>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if *role != NetworkRole::Client {
        return;
    }
    if let Some(new_state) = pending.0.take() {
        next_state.set(new_state.into());
    }
}
