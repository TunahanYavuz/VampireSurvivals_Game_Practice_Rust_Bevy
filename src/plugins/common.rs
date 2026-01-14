use bevy::camera::primitives::Aabb;
use bevy::prelude::*;

/// Oyun içi entity'leri işaretleyen marker component.
/// Oyun sona erdiğinde veya restart edildiğinde bu component'a sahip
/// tüm entity'ler temizlenir.
#[derive(Component)]
pub struct GameEntity;

pub fn aabb_intersects(a: &Aabb, b: &Aabb) -> bool {
    let a_min = a.min();
    let a_max = a.max();
    let b_min = b.min();
    let b_max = b.max();

    !(a_min.x > b_max.x || a_max.x < b_min.x || a_min.y > b_max.y || a_max.y < b_min.y)
}

pub fn contains_point(aabb: &Aabb, point: Vec3) -> bool {

    point.x >= aabb.center.x - aabb.half_extents.x
        && point.x<= aabb.center.x + aabb.half_extents.x
        && point.y >= aabb.center.y - aabb.half_extents.y
        && point.y <= aabb.center.y + aabb.half_extents.y
}