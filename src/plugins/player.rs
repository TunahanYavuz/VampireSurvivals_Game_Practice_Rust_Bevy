use std::time::Duration;
use bevy::prelude::{Assets, ButtonInput, Circle, Color, ColorMaterial, Command, Commands, Component, Entity, KeyCode, Mesh, Mesh2d, MeshMaterial2d, NextState, Query, Sprite, Time, Transform, Vec3, With, Without};
use bevy_ecs::prelude::MessageWriter;
use bevy_ecs::system::{Local, Res, ResMut};
use crate::plugins::aabb::AABB;
use crate::plugins::enemy::Enemy;
use crate::plugins::game_state::GameState;
use crate::plugins::score::GameScore;
use crate::plugins::timers::{EnemyMoveTimer, ShootTimer};
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
        enemy_move_timer: &EnemyMoveTimer,
    ) {
        let mut pos = transform.translation;
        if let Some(ref mut atlas) = sprite.texture_atlas {
            if enemy_move_timer.timer.just_finished() {
                atlas.index = (atlas.index + 1) % 35;
            }
        }

        if keyboard_input.pressed(KeyCode::KeyA) {
            pos.x -= self.movement * time.delta_secs();
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            pos.x += self.movement * time.delta_secs();
        }
        if keyboard_input.pressed(KeyCode::KeyW) {
            pos.y += self.movement * time.delta_secs();
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            pos.y -= self.movement * time.delta_secs();
        }
        transform.translation = pos;
        aabb.change_point(pos);
        camera_transform.translation = pos;
    }

    pub fn take_damage(
        &mut self,
        entity: Entity,
        commands: &mut Commands,
        enemy_query: &Query<(&AABB, &Enemy), (With<Enemy>, Without<Player>)>,
        player_aabb: &AABB,
    ) {
        for (enemy_aabb, _) in enemy_query.iter() {
            if self.health > 0 && enemy_aabb.self_aabb_intersects(player_aabb) {
                if self.health > 0 {
                    self.health -= 1;
                }
                println!("{:?}", self.health);
            }
        }
        if self.health == 0 {
            commands.entity(entity).despawn();
        }
    }
    pub fn level_up(&mut self, shoot_timer: &mut ShootTimer) {
        if self.xp >= self.xp_to_next_level {
            shoot_timer.timer.set_duration(Duration::from_secs_f32(0.1));
            self.xp -= self.xp_to_next_level;
            self.xp_to_next_level *= 1.1;
            self.level += 1;
        }
    }

    pub fn shoot(
        &mut self,
        commands: &mut Commands,
        player_transform: &Transform,
        enemy_query: &Query<(&Transform, &Enemy), (With<Enemy>, Without<Player>)>,
        mesh2d: &mut Assets<Mesh>,
        mesh_materials: &mut Assets<ColorMaterial>,
        shoot_timer: &mut ShootTimer,
        time: &Time,
    ) {
        shoot_timer.timer.tick(time.delta());
        if !shoot_timer.timer.just_finished() {
            return;
        }

        if enemy_query.is_empty() {
            return;
        }
        let mut closest_enemy = f32::INFINITY;
        let mut target_enemy: Option<Vec3> = None;
        for (enemy, _) in enemy_query.iter() {
            let distance = enemy.translation.distance(player_transform.translation);
            if closest_enemy > distance {
                closest_enemy = distance;
                target_enemy = Some(enemy.translation);
            }
        }
        let target = match target_enemy {
            None => {
                return;
            }
            Some(t) => t,
        };

        let diff = target - player_transform.translation;
        if diff.length_squared() < 1e-6 {
            return;
        }
        let dir = diff.normalize();
        commands.spawn((
            Transform::from_xyz(player_transform.translation.x, player_transform.translation.y, 5.0),
            Mesh2d(mesh2d.add(Circle {
                radius: 10.0,
                ..Default::default()
            })),
            MeshMaterial2d(mesh_materials.add(Color::WHITE)),
            
        ));
        self.level_up(shoot_timer);
    }
    pub fn gain_xp(&mut self, amount: f32, message_writer: &mut MessageWriter<LevelUpEvent>, next_state: &mut NextState<GameState>){
        self.xp += amount;

        while self.xp >=self.xp_to_next_level{
            self.xp -= self.xp_to_next_level;
            self.xp_to_next_level *= 1.2;
            self.level += 1;

            message_writer.write(LevelUpEvent{level: self.level});
            println!("🎉 LEVEL UP! Level: {}", self.level);
            next_state.set(GameState::UpgradeSelection);

        }
    }


}

pub fn gain_xp_from_kills(
    mut player_query: Query<&mut Player>,
    score: Res<GameScore>,
    mut last_score: Local<u32>,
    mut level_up_events: MessageWriter<LevelUpEvent>,
    mut next_state: ResMut<NextState<GameState>>,
){
    let Ok(mut player) = player_query.single_mut() else { return; };
    let kills_gained = score.score.saturating_sub(*last_score);
    if kills_gained > 0 {
        player.gain_xp(kills_gained as f32 * 10.0, &mut level_up_events, &mut next_state);
        *last_score = score.score;
    }
}

