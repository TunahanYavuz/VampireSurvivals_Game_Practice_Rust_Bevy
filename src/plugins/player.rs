use bevy::audio::{AudioPlayer, PlaybackSettings};
use bevy::image::TextureAtlas;
use bevy::prelude::*;
use crate::plugins::aabb::AABB;
use crate::plugins::audio::{GameAudio, GameAudioEntity};
use crate::plugins::enemy::{Collectible, Enemy, XP};
use crate::plugins::game::Atlases;
use crate::plugins::game_state::GameState;
use crate::plugins::timers::{MoveTimer, PlayerHealthReduceTimer};
use crate::plugins::weapon_upgrade::LevelUpEvent;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                move_player,
                reduce_player_health,
                collect_xp,
                collect_xp_with_magnet,
                magnetite_xp_to_player,
            ).run_if(in_state(GameState::Playing)),
        );
    }
}

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
    xp_query: Query<Entity, With<XP>>,
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

pub fn move_player(
    mut player_query: Query<(&mut Transform, &Player, &mut AABB, &mut Sprite), With<Player>>,
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    atlases: Res<Atlases>,
    enemy_move_timer: Res<MoveTimer>,
) {
    if !atlases.ready {
        return;
    }

    let Ok((mut transform, player, mut aabb, mut sprite)) = player_query.single_mut() else {
        return;
    };

    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };

    if sprite.texture_atlas.is_none() {
        if let Some(layout_handle) = &atlases.body {
            sprite.texture_atlas = Some(TextureAtlas {
                layout: layout_handle.clone(),
                index: 0,
            });
        }
    }

    player.move_around(
        &mut transform,
        &mut aabb,
        &mut sprite,
        &mut camera_transform,
        &keyboard_input,
        &time,
        &enemy_move_timer,
    );
}

pub fn reduce_player_health(
    mut commands: Commands,
    mut player_query: Query<(&mut Player, &mut AABB, Entity), With<Player>>,
    enemy_query: Query<(&AABB, &Enemy), (With<Enemy>, Without<Player>)>,
    mut player_health_reduce_timer: ResMut<PlayerHealthReduceTimer>,
    time: Res<Time>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    player_health_reduce_timer.timer.tick(time.delta());
    if !player_health_reduce_timer.timer.just_finished() {
        return;
    }

    let Ok((mut player, aabb, entity)) = player_query.single_mut() else {
        return;
    };

    player.take_damage(entity, &mut commands, enemy_query, &aabb);

    if player.health == 0 {
        next_state.set(GameState::GameOver);
    }
}

