use bevy::prelude::*;

#[derive(Resource)]
pub struct EnemySpawnTimer {
    pub timer: Timer,
}
impl Default for EnemySpawnTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.2, TimerMode::Repeating),
        }
    }
}
#[derive(Resource)]
pub struct ShootTimer {
    pub timer: Timer,
}
impl Default for ShootTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.2, TimerMode::Repeating),
        }
    }
}
#[derive(Resource)]
pub struct EnemyMoveTimer {
    pub timer: Timer,
}
impl Default for EnemyMoveTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.2, TimerMode::Repeating),
        }
    }

}
#[derive(Resource)]
pub struct PlayerHealthReduceTimer {
    pub timer: Timer,
}
impl Default for PlayerHealthReduceTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.2, TimerMode::Repeating),
        }
    }

}