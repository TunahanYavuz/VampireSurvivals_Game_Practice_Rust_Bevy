use crate::plugins::enemy::GameStageManager;
use crate::plugins::game_state::GameState;
use crate::plugins::locale::Locale;
use crate::plugins::player::Player;
use crate::plugins::timers::GameTimer;
use bevy::prelude::*;

pub struct ScorePlugin;

impl Plugin for ScorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameScore>()
            .add_systems(Startup, setup_score_ui)
            .add_systems(OnEnter(GameState::Playing), visible_score_ui)
            .add_systems(OnEnter(GameState::MainMenu), hide_score_ui)
            .add_systems(Update, update_score_ui.run_if(in_state(GameState::Playing)));
    }
}

#[derive(Component)]
pub struct ScoreText;

#[derive(Resource, Default)]
pub struct GameScore {
    pub score: u32,
}
pub fn setup_score_ui(mut commands: Commands) {
    commands.spawn((
        Text::new("Score 0"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            padding: UiRect::all(Val::Px(15.0)),
            border_radius: BorderRadius::all(Val::Px(10.0)),
            ..default()
        },
        Visibility::Hidden,
        Outline {
            width: Val::Px(2.0),
            offset: Val::Px(0.0),
            color: Color::srgba(1.0, 0.0, 0.0, 0.8),
        },
        BackgroundColor(Color::srgba(0.15, 0.15, 0.15, 0.9)),
        ScoreText,
    ));
}

pub fn update_score_ui(
    players: Query<&Player>,
    mut query: Query<&mut Text, With<ScoreText>>,
    game_timer: Res<GameTimer>,
    stage_manager: Res<GameStageManager>,
    locale: Res<Locale>,
) {
    let elapsed = game_timer.elapsed_secs as u32;
    let minutes = elapsed / 60;
    let seconds = elapsed % 60;
    let stage = stage_manager.current_stage_index + 1;

    // Aggregate across all players — use primary player (index 0) for XP display.
    let (total_score, total_hp) = players
        .iter()
        .fold((0u32, 0u32), |(s, h), p| (s + p.score, h + p.health));

    let primary = players.iter().find(|p| p.player_index == 0);
    let (xp, xp_next, level) = primary
        .map(|p| (p.xp, p.xp_to_next_level, p.level))
        .unwrap_or((0.0, 100.0, 1));

    for mut text in query.iter_mut() {
        text.0 = format!(
            "{}: {total_score}\n{}: {xp:.0} | {}: {xp_next:.0}\n{} (all): {total_hp}\nLv {level} | {}: {} ({}:{:02} {})",
            locale.t("score_label"),
            locale.t("xp_label"),
            locale.t("xp_to_next"),
            locale.t("player_hp"),
            locale.t("stage_label"),
            stage,
            minutes,
            seconds,
            locale.t("elapsed"),
        );
    }
}

fn visible_score_ui(mut query: Query<&mut Visibility, With<ScoreText>>) {
    for mut visibility in query.iter_mut() {
        *visibility = Visibility::Visible;
    }
}

fn hide_score_ui(mut query: Query<&mut Visibility, With<ScoreText>>) {
    for mut visibility in query.iter_mut() {
        *visibility = Visibility::Hidden;
    }
}
