use bevy::prelude::*;
use bevy_ecs::relationship::RelatedSpawnerCommands;
use crate::plugins::game_state::GameState;

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::MainMenu), setup_main_menu)
            .add_systems(Update, handle_menu_buttons.run_if(in_state(GameState::MainMenu)))
            .add_systems(OnExit(GameState::MainMenu), cleanup_menu);
    }
}

#[derive(Component)]
struct MainMenuUI;

#[derive(Component)]
enum MenuButton {
    Play,
    Settings,
    Quit,
}

fn setup_main_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");

    commands.spawn((
        MainMenuUI,
        Node{
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
        )).with_children(|parent| {
        parent.spawn((
            Text::new("Vampire Survivals Deneme"),
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
            spawn_button(parent, "Play", MenuButton::Play, font.clone());
            spawn_button(parent, "Settings", MenuButton::Settings, font.clone());
            spawn_button(parent, "Quit", MenuButton::Quit, font.clone());
    });
}

fn spawn_button(parent: &mut RelatedSpawnerCommands<ChildOf>, text: &str, button_type: MenuButton, font: Handle<Font>){
    parent.spawn((
        Button,
        button_type,
        Node{
            width: Val::Px(200.0),
            height: Val::Px(60.0),
            margin: UiRect::all(Val::Px(10.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
        )).with_children(|btn| {
            btn.spawn((
                Text::new(text),
                TextFont{
                    font,
                    font_size: 30.0,
                    ..default()
                }
                ));
    });
}

fn handle_menu_buttons(
    interactions_q: Query<(&Interaction, &MenuButton), Changed<Interaction>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<AppExit>,
){
    for (interaction, button) in &interactions_q {
        if *interaction == Interaction::Pressed {
            match button {
                MenuButton::Play => next_state.set(GameState::Loading),
                MenuButton::Settings => println!("Settings clicked"),
                MenuButton::Quit => {exit.write(AppExit::Success);},
            };
        }
    }
}

fn cleanup_menu(
    mut commands: Commands,
    menu: Query<Entity, With<MainMenuUI>>,
){
    for entity in &menu {
        commands.entity(entity).despawn();
    }
}