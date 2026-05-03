use crate::plugins::audio::{GameAudio, GameAudioEntity};
use crate::plugins::common::aabb_intersects;
use crate::plugins::enemy::{Enemy, XP};
use crate::plugins::game::Atlases;
use crate::plugins::game_state::GameState;
use crate::plugins::timers::{MoveTimer, PlayerHealthReduceTimer};
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
    /// 0 = Player 1 (WASD), 1 = Player 2 (Arrow keys)
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

            message_writer.write(LevelUpEvent { level: self.level });
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

        // Choose input keys per player index.
        let (key_left, key_right, key_up, key_down) = if player.player_index == 0 {
            (KeyCode::KeyA, KeyCode::KeyD, KeyCode::KeyW, KeyCode::KeyS)
        } else {
            (
                KeyCode::ArrowLeft,
                KeyCode::ArrowRight,
                KeyCode::ArrowUp,
                KeyCode::ArrowDown,
            )
        };

        let mut pos = transform.translation;
        let mut dir: i32 = 5; // 5 = idle

        if keyboard_input.pressed(key_left) {
            pos.x -= player.movement * time.delta_secs();
            dir = -1;
        }
        if keyboard_input.pressed(key_right) {
            pos.x += player.movement * time.delta_secs();
            dir = 1;
        }
        if keyboard_input.pressed(key_up) {
            pos.y += player.movement * time.delta_secs();
            dir = 2;
        }
        if keyboard_input.pressed(key_down) {
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
) {
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
