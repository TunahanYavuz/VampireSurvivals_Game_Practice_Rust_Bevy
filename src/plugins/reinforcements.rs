use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::{NoAutoAabb, NoFrustumCulling};
use bevy::prelude::*;
use rand::Rng;
use strum::{EnumCount, FromRepr};
use crate::plugins::audio::GameAudio;
use crate::plugins::common::aabb_intersects;
use crate::plugins::enemy::{Collectible, Enemy, XP};
use crate::plugins::game_state::GameState;
use crate::plugins::network::{NetIdCounter, NetworkIdentity, VisualType};
use crate::plugins::player::{Player, XPMagnetite};
use crate::plugins::weapon_upgrade::LevelUpEvent;

pub struct ReinforcementsPlugin;
impl Plugin for ReinforcementsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (collect_reinforcements, apply_reinforcements)
                .chain()
                .run_if(in_state(GameState::Playing)),
        );
    }
}
#[derive(EnumCount, FromRepr)]
pub enum ReinforcementType {
    Magnet,
    HealthPack,
    KillEnemies,

}

#[derive(Component)]
pub struct Reinforcements {
    pub reinforcement_type: ReinforcementType,
    pub is_collected: bool,
}

pub fn spawn_reinforcement(
    commands: &mut Commands,
    position: Vec3,
    amount: i32,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    net_id_counter: &mut NetIdCounter,
) {
    commands.spawn((
        Collectible,
        XP{ is_collected: false, amount },
        NetworkIdentity {
            net_id: net_id_counter.next(),
            visual_type: VisualType::XpGem,
        },
        Aabb {
            center: position.to_vec3a(),
            half_extents: Vec3A::new(40.0, 40.0, 1.0),
        },
        NoAutoAabb,
        NoFrustumCulling,
        Transform::from_translation(position),
        Mesh2d(meshes.add(Circle::new(5.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgb(0.8, 0.0, 0.0)))),
    ));

    if rand::rng().random_range(0..100) < 2 {
        let index = rand::rng().random_range(0..ReinforcementType::COUNT);

        if let Some(reinforcement_type) = ReinforcementType::from_repr(index) {
            let color = match reinforcement_type {
                ReinforcementType::Magnet => Color::srgb(0.0, 0.0, 0.8),
                ReinforcementType::HealthPack => Color::srgb(0.0, 0.8, 0.0),
                ReinforcementType::KillEnemies => Color::srgb(0.8, 0.8, 0.8),
            };

            let offset_position = position + Vec3::new(30.0, 0.0, 0.0);

            commands.spawn((
                Collectible,
                Reinforcements {
                    reinforcement_type,
                    is_collected: false,
                },
                NetworkIdentity {
                    net_id: net_id_counter.next(),
                    visual_type: VisualType::Reinforcement,
                },
                Aabb {
                    center: offset_position.to_vec3a(),
                    half_extents: Vec3A::new(40.0, 40.0, 1.0),
                },
                Mesh2d(meshes.add(Circle::new(8.0))),
                MeshMaterial2d(materials.add(ColorMaterial::from(color))),
                NoAutoAabb,
                NoFrustumCulling,
                Transform::from_translation(offset_position),
            ));
        }
    }
}

pub fn collect_reinforcements(
    mut player_query: Query<&Aabb, With<Player>>,
    mut reinforcements_q: Query<(&Aabb, Option<&mut Reinforcements>, Option<&mut XP>), With<Collectible>>,
) {
    for player_aabb in player_query.iter_mut() {
        for (reinforcement_aabb, reinforcement, xp) in reinforcements_q.iter_mut() {
            if aabb_intersects(reinforcement_aabb, player_aabb) {
                if let Some(mut xp) = xp {
                    xp.is_collected = true;
                    continue;
                }
                if let Some(mut reinforcement) = reinforcement {
                    reinforcement.is_collected = true;
                }

            }
        }
    }
}

pub fn apply_reinforcements(
    mut player_query: Query<&mut Player, With<Player>>,
    mut reinforcements_q: Query<(Option<&Reinforcements>, Option<&XP>, Entity), With<Collectible>>,
    xp_query: Query<Entity, (With<XP>, Without<XPMagnetite>)>,
    mut enemies: Query<&mut Enemy>,
    mut commands: Commands,
    mut level_up_events: MessageWriter<LevelUpEvent>,
    mut next_state: ResMut<NextState<GameState>>,
    audio: Res<GameAudio>,
) {
    // Find primary player (index 0) first; fall back to any alive player.
    let primary_entity = player_query
        .iter()
        .find(|p| p.player_index == 0)
        .is_some();

    for (reinforcement, xp, entity) in reinforcements_q.iter_mut() {
        let mut should_despawn = false;

        if let Some(reinforcement) = reinforcement {
            if reinforcement.is_collected {
                // Apply effect to all players.
                match reinforcement.reinforcement_type {
                    ReinforcementType::Magnet => {
                        for xp_entity in xp_query.iter() {
                            commands.entity(xp_entity).try_insert(XPMagnetite);
                        }
                    }
                    ReinforcementType::HealthPack => {
                        for mut player in player_query.iter_mut() {
                            player.health = (player.health + 20).min(player.max_health);
                        }
                    }
                    ReinforcementType::KillEnemies => {
                        for mut enemy in enemies.iter_mut() {
                            enemy.should_despawn = true;
                            enemy.drops_loot = false;
                        }
                    }
                }
                should_despawn = true;
            }
        } else if let Some(xp) = xp {
            if xp.is_collected {
                // Credit XP only to the primary player (P1 owns the level-up system).
                if primary_entity {
                    for mut player in player_query.iter_mut() {
                        if player.player_index == 0 {
                            player.gain_xp(
                                xp.amount as f32,
                                &mut level_up_events,
                                &mut next_state,
                                &mut commands,
                                &audio,
                            );
                            break;
                        }
                    }
                } else {
                    // No P1 alive — credit any remaining player.
                    if let Some(mut player) = player_query.iter_mut().next() {
                        player.gain_xp(
                            xp.amount as f32,
                            &mut level_up_events,
                            &mut next_state,
                            &mut commands,
                            &audio,
                        );
                    }
                }
                should_despawn = true;
            }
        }

        if should_despawn {
            commands.entity(entity).try_despawn();
        }
    }
}