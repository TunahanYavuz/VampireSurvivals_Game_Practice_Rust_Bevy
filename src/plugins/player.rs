use crate::plugins::audio::{GameAudio, GameAudioEntity};
use crate::plugins::common::aabb_intersects;
use crate::plugins::enemy::{Enemy, XP};
use crate::plugins::game::Atlases;
use crate::plugins::game_state::GameState;
use crate::plugins::network::{
    C2S, NetworkRole, NetOutbox, PendingStatSnapshot, RemoteInput, StatSnapshotMsg, PlayerStat,
    NetworkIdentity, EntitySnapshot, TransformSnapshot,
    encode,
};
use crate::plugins::timers::{MoveTimer, PlayerHealthReduceTimer, GameTimer};
use crate::plugins::weapon_upgrade::LevelUpEvent;
use bevy::audio::{AudioPlayer, PlaybackSettings};
use bevy::camera::primitives::Aabb;
use bevy::image::TextureAtlas;
use bevy::prelude::*;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                move_player,
                sync_camera.after(move_player),
                reduce_player_health,
                collect_xp_with_magnet,
                magnetite_xp_to_player,
                // Network systems
                send_client_input,
                apply_stat_snapshot,
            )
                .run_if(in_state(GameState::Playing)),
        );
    }
}

#[derive(Component)]
pub struct Player {
    pub health: u32,
    pub max_health: u32,
    pub score: u32,
    pub movement: f32,
    #[allow(unused)]
    pub starting_weapon: String,
    pub xp: f32,
    pub level: i32,
    pub xp_to_next_level: f32,
    /// 0 = Player 1 (WASD / Host), 1 = Player 2 (Arrow keys / Client)
    pub player_index: u8,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            health: 100,
            max_health: 100,
            score: 0,
            movement: 200.,
            starting_weapon: "Flame Thrower".to_string(),
            xp: 0.,
            level: 1,
            xp_to_next_level: 100.,
            player_index: 0,
        }
    }
}

impl Player {
    pub fn take_damage(
        &mut self,
        entity: Entity,
        commands: &mut Commands,
        enemy_query: Query<(&Aabb, &Enemy), (With<Enemy>, Without<Player>)>,
        player_aabb: &Aabb,
    ) {
        for (enemy_aabb, enemy) in enemy_query.iter() {
            if self.health > 0 && aabb_intersects(enemy_aabb, player_aabb) {
                if self.health > 0 {
                    self.health = self.health.saturating_sub(enemy.damage as u32);
                }
            }
        }
        if self.health == 0 {
            commands.entity(entity).despawn();
        }
    }

    pub fn gain_xp(
        &mut self,
        amount: f32,
        message_writer: &mut MessageWriter<LevelUpEvent>,
        next_state: &mut NextState<GameState>,
        commands: &mut Commands,
        audio: &GameAudio,
    ) {
        self.xp += amount;

        if self.xp >= self.xp_to_next_level {
            self.xp -= self.xp_to_next_level;
            self.xp_to_next_level *= 1.5;
            self.level += 1;
            commands.spawn((
                GameAudioEntity,
                AudioPlayer(audio.collect_xp.clone()),
                PlaybackSettings::DESPAWN,
            ));

            message_writer.write(LevelUpEvent {
                level: self.level,
                player_index: self.player_index,
            });
            next_state.set(GameState::UpgradeSelection);
        }
    }
}



#[derive(Component)]
pub struct XPMagnetite;

pub fn collect_xp_with_magnet(
    mut commands: Commands,
    xp_query: Query<Entity, With<XP>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyC) {
        for entity in xp_query.iter() {
            commands.entity(entity).insert(XPMagnetite);
        }
    }
}

pub fn magnetite_xp_to_player(
    mut xp_query: Query<(&mut Transform, &mut Aabb), (With<XPMagnetite>, Without<Player>)>,
    player_query: Query<(&Transform, &Player), (With<Player>, Without<XPMagnetite>)>,
) {
    // Prefer P1 (player_index == 0) as the magnet target; fall back to any alive player.
    let target = player_query
        .iter()
        .min_by_key(|(_, p)| p.player_index)
        .map(|(t, _)| t.translation);

    let Some(target_pos) = target else {
        return;
    };

    for (mut xp_transform, mut xp_aabb) in xp_query.iter_mut() {
        let direction = (target_pos - xp_transform.translation).normalize();
        xp_transform.translation += direction * 5.;
        xp_aabb.center = xp_transform.translation.into();
    }
}

pub fn move_player(
    mut player_query: Query<(&mut Transform, &Player, &mut Aabb, &mut Sprite), With<Player>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    atlases: Res<Atlases>,
    enemy_move_timer: Res<MoveTimer>,
    role: Res<NetworkRole>,
    remote_input: Res<RemoteInput>,
) {
    if !atlases.ready {
        return;
    }

    for (mut transform, player, mut aabb, mut sprite) in player_query.iter_mut() {
        if sprite.texture_atlas.is_none() {
            if let Some(layout_handle) = &atlases.body {
                sprite.texture_atlas = Some(TextureAtlas {
                    layout: layout_handle.clone(),
                    index: 0,
                });
            }
        }

        // Determine which input source to use for this player:
        // • Solo: keyboard for both players (original behavior)
        // • Host: keyboard for P1; RemoteInput (from client) for P2
        // • Client: keyboard for P2 only; P1 position arrives via stat snapshot
        let (left, right, up, down) = match *role {
            NetworkRole::Solo => {
                let (kl, kr, ku, kd) = if player.player_index == 0 {
                    (KeyCode::KeyA, KeyCode::KeyD, KeyCode::KeyW, KeyCode::KeyS)
                } else {
                    (KeyCode::ArrowLeft, KeyCode::ArrowRight, KeyCode::ArrowUp, KeyCode::ArrowDown)
                };
                (
                    keyboard_input.pressed(kl),
                    keyboard_input.pressed(kr),
                    keyboard_input.pressed(ku),
                    keyboard_input.pressed(kd),
                )
            }
            NetworkRole::Host => {
                if player.player_index == 0 {
                    (
                        keyboard_input.pressed(KeyCode::KeyA),
                        keyboard_input.pressed(KeyCode::KeyD),
                        keyboard_input.pressed(KeyCode::KeyW),
                        keyboard_input.pressed(KeyCode::KeyS),
                    )
                } else {
                    // P2's input comes from the remote client.
                    let ri = &remote_input.0;
                    (ri.left, ri.right, ri.up, ri.down)
                }
            }
            NetworkRole::Client => {
                if player.player_index == 1 {
                    // Local player on the client.
                    (
                        keyboard_input.pressed(KeyCode::ArrowLeft),
                        keyboard_input.pressed(KeyCode::ArrowRight),
                        keyboard_input.pressed(KeyCode::ArrowUp),
                        keyboard_input.pressed(KeyCode::ArrowDown),
                    )
                } else {
                    // P1 is controlled by the host; skip local movement.
                    continue;
                }
            }
        };

        let mut pos = transform.translation;
        let mut dir: i32 = 5; // 5 = idle

        if left {
            pos.x -= player.movement * time.delta_secs();
            dir = -1;
        }
        if right {
            pos.x += player.movement * time.delta_secs();
            dir = 1;
        }
        if up {
            pos.y += player.movement * time.delta_secs();
            dir = 2;
        }
        if down {
            pos.y -= player.movement * time.delta_secs();
            dir = 0;
        }

        if let Some(ref mut atlas) = sprite.texture_atlas {
            if enemy_move_timer.timer.just_finished() {
                if dir == -1 {
                    atlas.index = 9 + (atlas.index + 1) % 9;
                } else if dir == 1 {
                    atlas.index = 27 + (atlas.index + 1) % 9;
                } else if dir == 2 {
                    atlas.index = (atlas.index + 1) % 9;
                } else if dir == 0 {
                    atlas.index = 18 + (atlas.index + 1) % 9;
                }
            }
        }
        transform.translation = pos;
        aabb.center = transform.translation.to_vec3a();
    }
}

fn sync_camera(
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<Camera>, Without<Player>)>,
) {
    let positions: Vec<Vec3> = player_query.iter().map(|t| t.translation).collect();
    if positions.is_empty() {
        return;
    }
    let midpoint = positions.iter().copied().sum::<Vec3>() / positions.len() as f32;

    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };
    camera_transform.translation.x = midpoint.x;
    camera_transform.translation.y = midpoint.y;
}

pub fn reduce_player_health(
    mut commands: Commands,
    mut player_query: Query<(&mut Player, &mut Aabb, Entity), With<Player>>,
    enemy_query: Query<(&Aabb, &Enemy), (With<Enemy>, Without<Player>)>,
    mut player_health_reduce_timer: ResMut<PlayerHealthReduceTimer>,
    time: Res<Time>,
    mut next_state: ResMut<NextState<GameState>>,
    role: Res<NetworkRole>,
) {
    // On the client, health is authoritative from the host snapshot; skip local damage.
    if *role == NetworkRole::Client {
        return;
    }

    player_health_reduce_timer.timer.tick(time.delta());
    if !player_health_reduce_timer.timer.just_finished() {
        return;
    }

    let mut all_dead = true;

    for (mut player, aabb, entity) in player_query.iter_mut() {
        // Apply enemy contact damage.
        for (enemy_aabb, enemy) in enemy_query.iter() {
            if player.health > 0 && aabb_intersects(enemy_aabb, &aabb) {
                player.health = player.health.saturating_sub(enemy.damage as u32);
            }
        }

        if player.health == 0 {
            commands.entity(entity).despawn();
        } else {
            all_dead = false;
        }
    }

    if all_dead {
        next_state.set(GameState::GameOver);
    }
}

// ──────────────────────── Network systems ────────────────────────────────

/// Client: read P2's local keyboard state and send it to the host each frame.
fn send_client_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    role: Res<NetworkRole>,
    outbox: Option<Res<NetOutbox>>,
) {
    if *role != NetworkRole::Client {
        return;
    }
    let Some(outbox) = outbox else { return };

    let input = crate::plugins::network::InputState {
        left: keyboard.pressed(KeyCode::ArrowLeft),
        right: keyboard.pressed(KeyCode::ArrowRight),
        up: keyboard.pressed(KeyCode::ArrowUp),
        down: keyboard.pressed(KeyCode::ArrowDown),
        collect_magnet: keyboard.just_pressed(KeyCode::KeyC),
    };
    if let Ok(frame) = encode(&C2S::PlayerInput(input)) {
        let _ = outbox.0.send(frame);
    }
}

/// Client: apply the latest stat snapshot received from the host.
///
/// The host is authoritative for health, XP, level, score, and **position**.
/// We override local values so the client HUD always shows accurate figures,
/// and move player sprites to match the host's world state.
fn apply_stat_snapshot(
    role: Res<NetworkRole>,
    mut pending: ResMut<PendingStatSnapshot>,
    mut players: Query<(&mut Player, &mut Transform, &mut Aabb)>,
    mut game_timer: ResMut<GameTimer>,
) {
    if *role != NetworkRole::Client {
        return;
    }
    let Some(snap) = pending.0.take() else {
        return;
    };

    // Sync the game clock.
    game_timer.elapsed_secs = snap.game_elapsed_secs;

    for (mut player, mut transform, mut aabb) in players.iter_mut() {
        let (stat, pos): (&PlayerStat, [f32; 2]) = if player.player_index == 0 {
            (&snap.p1, snap.p1_pos)
        } else {
            (&snap.p2, snap.p2_pos)
        };

        // Sync stats.
        player.health = stat.health;
        player.xp = stat.xp;
        player.level = stat.level;
        player.xp_to_next_level = stat.xp_to_next_level;
        player.score = stat.score;

        // Sync position for P1 (authoritative from host).
        // P2's position is handled locally on the client, but we sync it too
        // for consistency — the server's position is always the truth.
        if pos != [0.0_f32, 0.0_f32] {
            transform.translation.x = pos[0];
            transform.translation.y = pos[1];
            aabb.center = transform.translation.to_vec3a();
        }
    }
}

/// Host: build a `StatSnapshotMsg` from the current player components and queue it.
///
/// In addition to player stats, this system gathers the `Transform` of every
/// entity that carries a `NetworkIdentity` and packs them into
/// `StatSnapshotMsg::entities`.  The client's `client_entity_sync` system
/// consumes that list to keep the ghost world in sync.
///
/// Player positions are sent separately as `p1_pos`/`p2_pos` so the client
/// can smoothly move the P1 and P2 sprites.
///
/// Called every frame when running as host so the client stays in sync.
pub fn flush_stat_snapshot(
    role: Res<NetworkRole>,
    outbox: Option<Res<NetOutbox>>,
    players: Query<(&Player, &Transform)>,
    net_entities: Query<(&NetworkIdentity, &Transform)>,
    game_timer: Res<GameTimer>,
) {
    if *role != NetworkRole::Host {
        return;
    }
    let Some(outbox) = outbox else { return };

    let mut msg = StatSnapshotMsg::default();
    for (player, transform) in players.iter() {
        let stat = PlayerStat {
            health: player.health,
            xp: player.xp,
            level: player.level,
            xp_to_next_level: player.xp_to_next_level,
            score: player.score,
        };
        if player.player_index == 0 {
            msg.p1 = stat;
            msg.p1_pos = [transform.translation.x, transform.translation.y];
        } else {
            msg.p2 = stat;
            msg.p2_pos = [transform.translation.x, transform.translation.y];
        }
    }

    // Gather world state: every replicated entity's current transform.
    msg.entities = net_entities
        .iter()
        .map(|(nid, transform)| EntitySnapshot {
            net_id: nid.net_id,
            visual_type: nid.visual_type,
            transform: TransformSnapshot::from_transform(transform),
        })
        .collect();

    // Sync the host's game clock so the client HUD shows the same time.
    msg.game_elapsed_secs = game_timer.elapsed_secs;

    use crate::plugins::network::S2C;
    if let Ok(frame) = encode(&S2C::StatSnapshot(msg)) {
        let _ = outbox.0.send(frame);
    }
}

