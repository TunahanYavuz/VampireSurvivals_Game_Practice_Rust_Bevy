use bevy::prelude::*;

/// Oyun içi entity'leri işaretleyen marker component.
/// Oyun sona erdiğinde veya restart edildiğinde bu component'a sahip
/// tüm entity'ler temizlenir.
#[derive(Component)]
pub struct GameEntity;

