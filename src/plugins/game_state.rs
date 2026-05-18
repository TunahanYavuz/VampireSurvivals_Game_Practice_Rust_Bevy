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
    /// Host is choosing upgrades; client shows a waiting notice.
    HostUpgrade,
    /// Other player is choosing upgrades; local input is paused.
    RemoteUpgrade,
}
