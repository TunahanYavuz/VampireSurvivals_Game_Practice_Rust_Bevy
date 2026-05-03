use bevy::prelude::*;
use crate::plugins::game_state::GameState;

pub struct TimerPlugin;

impl Plugin for TimerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MoveTimer>()
            .init_resource::<PlayerHealthReduceTimer>()
            .init_resource::<GameTimer>()
            .add_systems(Update, tick_game_timer.run_if(in_state(GameState::Playing)));
    }
}

#[derive(Resource, Default)]
pub struct GameTimer {
    pub elapsed_secs: f32,
}

fn tick_game_timer(time: Res<Time>, mut game_timer: ResMut<GameTimer>) {
    game_timer.elapsed_secs += time.delta_secs();
}

#[derive(Resource)]
pub struct EnemySpawnTimer {
    pub timer: Timer,
}
impl Default for EnemySpawnTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.1, TimerMode::Repeating),
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
