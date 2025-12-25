use bevy::prelude::*;

#[derive(Resource)]
pub struct EnemySpawnTimer {
    pub timer: Timer,
}
impl Default for EnemySpawnTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.01, TimerMode::Repeating),
        }
    }
}

#[derive(Resource)]
pub struct MoveTimer {
    pub timer: Timer,
}
impl Default for MoveTimer {
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