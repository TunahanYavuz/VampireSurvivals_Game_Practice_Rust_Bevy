use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;

use crate::plugins::game_state::GameState;
use crate::plugins::locale::{Language, Locale};

/// Path to the settings file saved next to the executable / working directory.
const SETTINGS_PATH: &str = "settings.ron";

// ---------------------------------------------------------------------------
// Settings resource
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize, Clone, Resource)]
pub struct Settings {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub language: Language,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            music_volume: 0.7,
            sfx_volume: 1.0,
            language: Language::English,
        }
    }
}

impl Settings {
    /// Load from disk, falling back to defaults when the file is absent or corrupt.
    pub fn load() -> Self {
        match fs::read_to_string(SETTINGS_PATH) {
            Ok(content) => ron::from_str(&content).unwrap_or_default(),
            Err(_) => Settings::default(),
        }
    }

    /// Persist to disk.
    pub fn save(&self) {
        match ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()) {
            Ok(content) => {
                if let Err(e) = fs::write(SETTINGS_PATH, content) {
                    eprintln!("Failed to save settings: {e}");
                }
            }
            Err(e) => eprintln!("Failed to serialize settings: {e}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        let settings = Settings::load();
        let locale = Locale::load(settings.language.clone());

        app.insert_resource(settings)
            .insert_resource(locale)
            .add_systems(OnEnter(GameState::Settings), setup_settings_ui)
            .add_systems(
                Update,
                (handle_settings_buttons, refresh_settings_labels)
                    .run_if(in_state(GameState::Settings)),
            )
            .add_systems(OnExit(GameState::Settings), cleanup_settings_ui);
    }
}

// ---------------------------------------------------------------------------
// UI markers
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct SettingsUI;

#[derive(Component, Clone, Copy)]
pub enum SettingsButton {
    MasterVolumeDown,
    MasterVolumeUp,
    MusicVolumeDown,
    MusicVolumeUp,
    SfxVolumeDown,
    SfxVolumeUp,
    ToggleLanguage,
    Back,
}

#[derive(Component)]
pub struct MasterVolumeLabel;

#[derive(Component)]
pub struct MusicVolumeLabel;

#[derive(Component)]
pub struct SfxVolumeLabel;

#[derive(Component)]
pub struct LanguageLabel;

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

fn setup_settings_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<Settings>,
    locale: Res<Locale>,
) {
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");

    commands
        .spawn((
            SettingsUI,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(18.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.08, 0.08, 0.14)),
        ))
        .with_children(|root| {
            // Title
            root.spawn((
                Text::new(locale.t("settings_title")),
                TextFont {
                    font: font.clone(),
                    font_size: 52.0,
                    ..default()
                },
                Node {
                    margin: UiRect::bottom(Val::Px(24.0)),
                    ..default()
                },
            ));

            // Master volume
            spawn_volume_row(
                root,
                &font,
                locale.t("master_volume"),
                settings.master_volume,
                SettingsButton::MasterVolumeDown,
                SettingsButton::MasterVolumeUp,
                MasterVolumeLabel,
            );

            // Music volume
            spawn_volume_row(
                root,
                &font,
                locale.t("music_volume"),
                settings.music_volume,
                SettingsButton::MusicVolumeDown,
                SettingsButton::MusicVolumeUp,
                MusicVolumeLabel,
            );

            // SFX volume
            spawn_volume_row(
                root,
                &font,
                locale.t("sfx_volume"),
                settings.sfx_volume,
                SettingsButton::SfxVolumeDown,
                SettingsButton::SfxVolumeUp,
                SfxVolumeLabel,
            );

            // Language toggle
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    Text::new(format!(
                        "{}: {}",
                        locale.t("language"),
                        settings.language.display_name()
                    )),
                    TextFont {
                        font: font.clone(),
                        font_size: 24.0,
                        ..default()
                    },
                    LanguageLabel,
                ));
                spawn_small_button(
                    row,
                    &font,
                    locale.t("toggle"),
                    SettingsButton::ToggleLanguage,
                );
            });

            // Back button
            root.spawn((
                Button,
                SettingsButton::Back,
                Node {
                    width: Val::Px(220.0),
                    height: Val::Px(60.0),
                    margin: UiRect::top(Val::Px(28.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.35, 0.1, 0.1)),
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new(locale.t("save_and_back")),
                    TextFont {
                        font: font.clone(),
                        font_size: 26.0,
                        ..default()
                    },
                ));
            });
        });
}

fn spawn_volume_row(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    label: &str,
    value: f32,
    btn_down: SettingsButton,
    btn_up: SettingsButton,
    value_marker: impl Component,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::new(format!("{}: ", label)),
                TextFont {
                    font: font.clone(),
                    font_size: 24.0,
                    ..default()
                },
            ));
            spawn_small_button(row, font, "−", btn_down);
            row.spawn((
                Text::new(format!("{:3}%", (value * 100.0).round() as u32)),
                TextFont {
                    font: font.clone(),
                    font_size: 24.0,
                    ..default()
                },
                value_marker,
            ));
            spawn_small_button(row, font, "+", btn_up);
        });
}

fn spawn_small_button(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    label: &str,
    marker: SettingsButton,
) {
    parent
        .spawn((
            Button,
            marker,
            Node {
                width: Val::Px(42.0),
                height: Val::Px(42.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.25, 0.25, 0.35)),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size: 22.0,
                    ..default()
                },
            ));
        });
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

const VOL_STEP: f32 = 0.1;

fn handle_settings_buttons(
    interaction_q: Query<(&Interaction, &SettingsButton), (Changed<Interaction>, With<Button>)>,
    mut settings: ResMut<Settings>,
    mut locale: ResMut<Locale>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, button) in &interaction_q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button {
            SettingsButton::MasterVolumeDown => {
                settings.master_volume = (settings.master_volume - VOL_STEP).max(0.0);
            }
            SettingsButton::MasterVolumeUp => {
                settings.master_volume = (settings.master_volume + VOL_STEP).min(1.0);
            }
            SettingsButton::MusicVolumeDown => {
                settings.music_volume = (settings.music_volume - VOL_STEP).max(0.0);
            }
            SettingsButton::MusicVolumeUp => {
                settings.music_volume = (settings.music_volume + VOL_STEP).min(1.0);
            }
            SettingsButton::SfxVolumeDown => {
                settings.sfx_volume = (settings.sfx_volume - VOL_STEP).max(0.0);
            }
            SettingsButton::SfxVolumeUp => {
                settings.sfx_volume = (settings.sfx_volume + VOL_STEP).min(1.0);
            }
            SettingsButton::ToggleLanguage => {
                settings.language = settings.language.toggle();
                *locale = Locale::load(settings.language.clone());
            }
            SettingsButton::Back => {
                settings.save();
                next_state.set(GameState::MainMenu);
            }
        }
    }
}

/// Refresh numeric labels whenever settings change.
fn refresh_settings_labels(
    settings: Res<Settings>,
    locale: Res<Locale>,
    mut master_q: Query<
        &mut Text,
        (
            With<MasterVolumeLabel>,
            Without<MusicVolumeLabel>,
            Without<SfxVolumeLabel>,
            Without<LanguageLabel>,
        ),
    >,
    mut music_q: Query<
        &mut Text,
        (
            With<MusicVolumeLabel>,
            Without<MasterVolumeLabel>,
            Without<SfxVolumeLabel>,
            Without<LanguageLabel>,
        ),
    >,
    mut sfx_q: Query<
        &mut Text,
        (
            With<SfxVolumeLabel>,
            Without<MasterVolumeLabel>,
            Without<MusicVolumeLabel>,
            Without<LanguageLabel>,
        ),
    >,
    mut lang_q: Query<
        &mut Text,
        (
            With<LanguageLabel>,
            Without<MasterVolumeLabel>,
            Without<MusicVolumeLabel>,
            Without<SfxVolumeLabel>,
        ),
    >,
) {
    if settings.is_changed() {
        if let Ok(mut t) = master_q.single_mut() {
            t.0 = format!("{:3}%", (settings.master_volume * 100.0).round() as u32);
        }
        if let Ok(mut t) = music_q.single_mut() {
            t.0 = format!("{:3}%", (settings.music_volume * 100.0).round() as u32);
        }
        if let Ok(mut t) = sfx_q.single_mut() {
            t.0 = format!("{:3}%", (settings.sfx_volume * 100.0).round() as u32);
        }
    }
    if locale.is_changed() || settings.is_changed() {
        if let Ok(mut t) = lang_q.single_mut() {
            t.0 = format!(
                "{}: {}",
                locale.t("language"),
                settings.language.display_name()
            );
        }
    }
}

fn cleanup_settings_ui(mut commands: Commands, ui: Query<Entity, With<SettingsUI>>) {
    for e in &ui {
        commands.entity(e).despawn();
    }
}
