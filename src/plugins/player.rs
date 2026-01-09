use bevy::audio::{AudioPlayer, PlaybackSettings};
use bevy::prelude::{ButtonInput, Commands, Component, Entity, KeyCode, NextState, Query, Sprite, Time, Transform, With, Without};
use bevy_ecs::prelude::{MessageWriter, Res};
use bevy_ecs::system::{ResMut, Single};
use crate::plugins::aabb::AABB;
use crate::plugins::audio::{GameAudio, GameAudioEntity};
use crate::plugins::enemy::{Collectible, Enemy, XP};
use crate::plugins::game_state::GameState;
use crate::plugins::timers::{MoveTimer};
use crate::plugins::weapon_upgrade::LevelUpEvent;

#[derive(Component)]
pub struct Player {
    pub health: u32,
    pub score: u32,
    pub movement: f32,
    pub xp: f32,
    pub level: i32,
    pub xp_to_next_level: f32,
}

impl Default for Player {
    fn default() -> Self {
        Self { health: 100, score: 0, movement: 200., xp: 0., level: 1, xp_to_next_level: 100. }
    }
}

impl Player {
    pub fn move_around(
        &self,
        transform: &mut Transform,
        aabb: &mut AABB,
        sprite: &mut Sprite,
        camera_transform: &mut Transform,
        keyboard_input: &ButtonInput<KeyCode>,
        time: &Time,
        move_timer: &MoveTimer,
    ) {
        let mut pos = transform.translation;


        let mut dir= 5;

        if keyboard_input.pressed(KeyCode::KeyA) {
            pos.x -= self.movement * time.delta_secs();
            dir = -1;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            pos.x += self.movement * time.delta_secs();
            dir = 1;
        }
        if keyboard_input.pressed(KeyCode::KeyW) {
            pos.y += self.movement * time.delta_secs();
            dir = 2;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            pos.y -= self.movement * time.delta_secs();
            dir = 0
        }
        if let Some(ref mut atlas) = sprite.texture_atlas {
            if move_timer.timer.just_finished() {
                if dir == -1 {
                    atlas.index = 9 + (atlas.index + 1) % 9;
                } else if dir == 1 {
                    atlas.index = 27 + (atlas.index + 1) % 9;
                } else if dir == 2 {
                    atlas.index = 0 + (atlas.index + 1) % 9;
                } else if dir == 0 {
                    atlas.index = 18 + (atlas.index + 1) % 9;
                }

            }
        }
        transform.translation = pos;
        aabb.change_point(pos);
        camera_transform.translation = pos;
    }

    pub fn take_damage(
        &mut self,
        entity: Entity,
        commands: &mut Commands,
        enemy_query: Query<(&AABB, &Enemy), (With<Enemy>, Without<Player>)>,
        player_aabb: &AABB,
    ) {
        for (enemy_aabb, enemy) in enemy_query.iter() {
            if self.health > 0 && enemy_aabb.self_aabb_intersects(player_aabb) {
                if self.health > 0 {
                    self.health = self.health.saturating_sub(enemy.damage as u32);
                }
                println!("{:?}", self.health);
            }
        }
        if self.health == 0 {
            commands.entity(entity).despawn();
        }
    }

    pub fn gain_xp(&mut self, amount: f32, message_writer: &mut MessageWriter<LevelUpEvent>, next_state: &mut NextState<GameState>, commands: &mut Commands, audio: &GameAudio) {
        self.xp += amount;

        if self.xp >=self.xp_to_next_level{
            self.xp -= self.xp_to_next_level;
            self.xp_to_next_level *= 1.5;
            self.level += 1;
            commands.spawn((
                GameAudioEntity,
                AudioPlayer(audio.collect_xp.clone()),
                PlaybackSettings::DESPAWN,
            ));

            message_writer.write(LevelUpEvent{level: self.level});
            println!("🎉 LEVEL UP! Level: {}", self.level);
            next_state.set(GameState::UpgradeSelection);
        }
    }
}

pub fn collect_xp(
    mut player_query: Query<(&mut Player, &AABB), With<Player>>,
    mut xp_query: Query<(&AABB, &Collectible, &XP, Entity)>,
    mut commands: Commands,
    mut level_up_events: MessageWriter<LevelUpEvent>,
    mut next_state: ResMut<NextState<GameState>>,
    audio: Res<GameAudio>,
){
    for (mut player, player_aabb) in player_query.iter_mut(){
        for (xp_aabb, _collectible, xp, entity) in xp_query.iter_mut(){
            if xp_aabb.self_aabb_intersects(player_aabb) {
                player.gain_xp(xp.amount as f32, &mut level_up_events, &mut next_state, &mut commands, &audio);
                commands.entity(entity).despawn();
            }
        }
    }
}

#[derive(Component)]
pub struct XPMagnetite;

pub fn collect_xp_with_magnet(
    mut commands: Commands,
    mut xp_query: Query<Entity, With<XP>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
){
    if keyboard_input.just_pressed(KeyCode::KeyC){
        for entity in xp_query {
            commands.entity(entity).insert(XPMagnetite);
        }
    }
}

pub fn magnetite_xp_to_player(
    mut xp_query: Query<(&mut Transform, &mut AABB), (With<XPMagnetite>, Without<Player>)>,
    player_query: Query<&Transform, (With<Player>, Without<XPMagnetite>)>,
){
    let Ok(player_position) = player_query.single() else{
        return;
    };

    for (mut xp_transform, mut xp_aabb) in xp_query.iter_mut(){
        let direction = (player_position.translation - xp_transform.translation).normalize();
        xp_transform.translation += direction * 5.;
        xp_aabb.change_point(xp_transform.translation);
    }
}