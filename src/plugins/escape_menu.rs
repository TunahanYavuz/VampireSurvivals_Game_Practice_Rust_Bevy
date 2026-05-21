use bevy::ecs::relationship::RelatedSpawnerCommands;
use bevy::prelude::*;
use crate::plugins::game_state::GameState;
use crate::plugins::locale::Locale;
use crate::plugins::network::{encode, NetOutbox, NetworkRole, NetworkedGameState, C2S, S2C};
use crate::plugins::settings::SettingsOrigin;

pub struct EscapeMenuPlugin;

impl Plugin for EscapeMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            toggle_escape_menu
                .run_if(in_state(GameState::Playing).or(in_state(GameState::HostEscapeMenu)).or(in_state(GameState::RemoteEscapeMenu))),
        )
            .add_systems(OnEnter(GameState::HostEscapeMenu), create_escape_menu)
            .add_systems(OnEnter(GameState::RemoteEscapeMenu), create_escape_menu)
            .add_systems(
                Update,
                handle_escape_menu_buttons.run_if(in_state(GameState::HostEscapeMenu).or(in_state(GameState::RemoteEscapeMenu))),
            )
            .add_systems(OnExit(GameState::HostEscapeMenu), cleanup_escape_menu)
            .add_systems(OnExit(GameState::RemoteEscapeMenu), cleanup_escape_menu);
    }
}

#[derive(Component)]
pub struct EscapeMenu;

fn create_escape_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    locale: Res<Locale>,
    role: Res<NetworkRole>,
    state: Res<State<GameState>>,
) {
    let node = Node{
        width: Val::Percent(90.0),
        height: Val::Percent(90.0),
        margin: UiRect::all(Val::Auto),
        align_content: AlignContent::Center,
        align_items: AlignItems::Center,
        flex_direction: FlexDirection::Column,
        align_self: AlignSelf::Center,
        border: UiRect::all(Val::Px(2.0)),
        border_radius: BorderRadius::all(Val::Px(10.0)),
        ..default()
    };
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");
    if (*state == GameState::HostEscapeMenu && *role == NetworkRole::Client) || (*state == GameState::RemoteEscapeMenu && *role == NetworkRole::Host) {
        commands.spawn((
            EscapeMenu,
            node,
            Text::new(locale.t("escape_menu_waiting")),
            TextFont{
                font: font.clone(),
                font_size: 40.0,
                ..default()
            },
            BackgroundColor::from(Color::srgba(0.1, 0.0,0.1, 0.8)),
        ));
        return;
    }
    commands.spawn((
        EscapeMenu,
        node,
        BackgroundColor::from(Color::srgba(0.1, 0.0, 0.1, 0.8)),
        )).with_children(|parent| {
        parent.spawn((
            Text::new(locale.t("escape_menu_title")),
            TextFont{
                font: font.clone(),
                font_size: 60.0,
                ..default()
            },
            Node{
                margin: UiRect::bottom(Val::Px(50.0)),
                ..default()
            }
        ));
            spawn_button(parent, locale.t("escape_menu_resume"), EscapeMenuButton::Play, font.clone());
            spawn_button(parent, locale.t("escape_menu_settings"), EscapeMenuButton::Settings, font.clone());
            spawn_button(parent, locale.t("escape_menu_quit"), EscapeMenuButton::Quit, font.clone());
    });
}
fn spawn_button(
    parent: &mut RelatedSpawnerCommands<ChildOf>,
    text: &str,
    button_type: EscapeMenuButton,
    font: Handle<Font>,
) {
    parent
        .spawn((
            Button,
            button_type,
            Node {
                width: Val::Px(200.0),
                height: Val::Px(60.0),
                margin: UiRect::all(Val::Px(10.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(text),
                TextFont {
                    font,
                    font_size: 30.0,
                    ..default()
                },
            ));
        });
}

#[derive(Component)]
enum EscapeMenuButton{
    Play,
    Settings,
    Quit,
}

fn toggle_escape_menu(
    mut next_state: ResMut<NextState<GameState>>,
    c_state: Res<State<GameState>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    role: Res<NetworkRole>,
    outbox: Option<ResMut<NetOutbox>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        match *role {
            NetworkRole::Solo => {
                    if c_state.eq(&GameState::HostEscapeMenu) {
                        next_state.set(GameState::Playing);
                    } else {
                        next_state.set(GameState::HostEscapeMenu);
                    }
            }
            NetworkRole::Host => {
                    if c_state.eq(&GameState::HostEscapeMenu) {
                        next_state.set(GameState::Playing);
                        if let Ok(frame) = encode(&S2C::StateChange(NetworkedGameState::Playing)){
                            if let Some(outbox) = outbox {
                                let _ = outbox.0.send(frame).expect("TODO: panic message");
                            };
                        }
                    } else {
                        next_state.set(GameState::HostEscapeMenu);
                        if let Ok(frame) = encode(&S2C::StateChange(NetworkedGameState::HostEscapeMenu)){
                            if let Some(outbox) = outbox {
                                let _ = outbox.0.send(frame).expect("TODO: panic message");
                            };
                        }
                    }
            }
            NetworkRole::Client => {
                    if c_state.eq(&GameState::RemoteEscapeMenu) {
                        next_state.set(GameState::Playing);
                        if let Ok(frame) = encode(&C2S::StateChange(NetworkedGameState::Playing)){
                            if let Some(outbox) = outbox {
                                let _ = outbox.0.send(frame).expect("TODO: panic message");
                            };
                        }
                    } else {
                        next_state.set(GameState::RemoteEscapeMenu);
                        if let Ok(frame) = encode(&C2S::StateChange(NetworkedGameState::RemoteEscapeMenu)){
                            if let Some(outbox) = outbox {
                                let _ = outbox.0.send(frame).expect("TODO: panic message");
                            };
                        }
                    }
            }
        }
    }
}

fn cleanup_escape_menu(
    mut commands: Commands,
    escape_menu: Query<Entity, With<EscapeMenu>>
){
    for entity in escape_menu.iter() {
        commands.entity(entity).despawn();
    }
}

fn handle_escape_menu_buttons(
    interactions_q: Query<(&Interaction, &EscapeMenuButton), Changed<Interaction>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut settings_origin: ResMut<SettingsOrigin>,
    role: Res<NetworkRole>,
    outbox: Option<ResMut<NetOutbox>>,
){
    for (interactions, escape_menu) in interactions_q.iter() {
        if *interactions == Interaction::Pressed {
            match escape_menu {
                EscapeMenuButton::Play => {
                    next_state.set(GameState::Playing);
                    if *role == NetworkRole::Host {
                        if let Ok(frame) = encode(&S2C::StateChange(NetworkedGameState::Playing)){
                            if let Some(outbox) = &outbox {
                                let _ = outbox.0.send(frame);
                            };
                        }
                    } else if *role == NetworkRole::Client {
                        if let Ok(frame) = encode(&C2S::StateChange(NetworkedGameState::Playing)){
                            if let Some(outbox) = &outbox {
                                let _ = outbox.0.send(frame);
                            };
                        }
                    }
                },
                EscapeMenuButton::Settings => {
                    *settings_origin = SettingsOrigin::EscapeMenu;
                    next_state.set(GameState::Settings)
                },
                EscapeMenuButton::Quit => {
                    next_state.set(GameState::MainMenu);
                    if *role == NetworkRole::Host {
                        if let Ok(frame) = encode(&S2C::StateChange(NetworkedGameState::MainMenu)){
                            if let Some(outbox) = &outbox {
                                let _ = outbox.0.send(frame);
                            };
                        }
                    } else if *role == NetworkRole::Client {
                        if let Ok(frame) = encode(&C2S::StateChange(NetworkedGameState::MainMenu)){
                            if let Some(outbox) = &outbox {
                                 let _ = outbox.0.send(frame);
                            };
                        }
                    }
                },
            };
        }
    }
}

