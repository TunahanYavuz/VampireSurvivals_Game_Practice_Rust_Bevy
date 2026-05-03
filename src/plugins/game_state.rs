use bevy::prelude::States;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    MainMenu,
    Settings,
    Loading,
    Playing,
    GameOver,
    UpgradeSelection,
}
