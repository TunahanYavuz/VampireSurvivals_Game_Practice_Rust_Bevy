use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::{NoAutoAabb, NoFrustumCulling};
use bevy::prelude::*;
use rand::Rng;
use strum::{EnumCount, FromRepr};
use crate::plugins::common::aabb_intersects;
use crate::plugins::enemy::{Collectible, Enemy, XP};
use crate::plugins::game_state::GameState;
use crate::plugins::network::{NetIdCounter, NetworkIdentity, NetworkRole, UpgradeMode, VisualType};
use crate::plugins::player::{GainXpEvent, Player, XPMagnetite};
use crate::plugins::texture_handling::{TextureAssets, TextureType};

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
    AtomBomb,

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
    net_id_counter: &mut NetIdCounter,
    textures: &Res<TextureAssets>,
) {
    commands.spawn((
        Collectible,
        XP{ is_collected: false, amount, collected_by: None },
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
        Sprite::from_image(textures.textures.get(&TextureType::XPGem).unwrap().clone()),
    ));

    if rand::rng().random_range(0..100) < 2 {
        let index = rand::rng().random_range(0..ReinforcementType::COUNT);

        if let Some(reinforcement_type) = ReinforcementType::from_repr(index) {
            let v_type;
            let sprite = match reinforcement_type {
                ReinforcementType::Magnet => {
                    v_type = VisualType::Magnet;
                    Sprite::from_image(textures.textures.get(&TextureType::Magnet).unwrap().clone()) },
                ReinforcementType::HealthPack => {
                    v_type = VisualType::HealthPack;
                    Sprite::from_image(textures.textures.get(&TextureType::HealthPack).unwrap().clone()) },
                ReinforcementType::AtomBomb => {
                    v_type = VisualType::AtomBomb;
                    Sprite::from_image(textures.textures.get(&TextureType::AtomBomb).unwrap().clone()) },
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
                    visual_type: v_type,
                },
                Aabb {
                    center: offset_position.to_vec3a(),
                    half_extents: Vec3A::new(40.0, 40.0, 1.0),
                },
                sprite,
                NoAutoAabb,
                NoFrustumCulling,
                Transform::from_translation(offset_position),
            ));
        }
    }
}

pub fn collect_reinforcements(
    role: Res<NetworkRole>,
    player_query: Query<(&Aabb, &Player), With<Player>>,
    mut reinforcements_q: Query<(&Aabb, Option<&mut Reinforcements>, Option<&mut XP>), With<Collectible>>,
) {
    if *role == NetworkRole::Client {
        return;
    }

    for (player_aabb, player) in player_query.iter() {
        for (reinforcement_aabb, reinforcement, xp) in reinforcements_q.iter_mut() {
            if aabb_intersects(reinforcement_aabb, player_aabb) {
                if let Some(mut xp) = xp {
                    xp.is_collected = true;
                    xp.collected_by = Some(player.player_index);
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
    role: Res<NetworkRole>,
    upgrade_mode: Res<UpgradeMode>,
    mut player_query: Query<&mut Player, With<Player>>,
    mut reinforcements_q: Query<(Option<&Reinforcements>, Option<&XP>, Entity), With<Collectible>>,
    xp_query: Query<Entity, (With<XP>, Without<XPMagnetite>)>,
    mut enemies: Query<&mut Enemy>,
    mut commands: Commands,
    mut gain_xp_events: MessageWriter<GainXpEvent>,
) {
    if *role == NetworkRole::Client {
        return;
    }

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
                    ReinforcementType::AtomBomb => {
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
                let target_player_index = match *upgrade_mode {
                    UpgradeMode::Shared => {
                        if primary_entity { Some(0) } else { None }
                    }
                    UpgradeMode::Independent => xp.collected_by,
                };

                if let Some(player_index) = target_player_index {
                    gain_xp_events.write(GainXpEvent {
                        player_index,
                        amount: xp.amount as f32,
                    });
                } else if let Some(player) = player_query.iter_mut().next() {
                    gain_xp_events.write(GainXpEvent {
                        player_index: player.player_index,
                        amount: xp.amount as f32,
                    });
                }

                should_despawn = true;
            }
        }

        if should_despawn {
            commands.entity(entity).try_despawn();
        }
    }
}