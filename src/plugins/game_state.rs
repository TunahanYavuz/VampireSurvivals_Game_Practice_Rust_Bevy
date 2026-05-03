use bevy::prelude::States;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    MainMenu,
    /// Host/Client role selection, IP entry, and upgrade-mode choice.
    Lobby,
    Settings,
    Loading,
    Playing,
    GameOver,
    UpgradeSelection,
}
