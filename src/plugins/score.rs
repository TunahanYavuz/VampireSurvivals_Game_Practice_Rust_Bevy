use bevy::prelude::*;
use crate::plugins::player::Player;
use crate::plugins::weapons::GameEntity;

#[derive(Component)]
pub struct ScoreText;

#[derive(Resource, Default)]
pub struct GameScore {
    pub score: u32,
}
pub fn setup_score_ui(
    mut commands: Commands,
){
    commands.spawn((
        Text::new("Score 0"),
        Node{
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            padding: UiRect::all(Val::Px(15.0)),
            ..default()
        },
        Outline{
            width: Val::Px(2.0),
            offset: Val::Px(0.0),
            color: Color::srgba(1.0, 0.0, 0.0, 0.8),
        },
        BorderRadius::all(Val::Px(5.0)),
        BackgroundColor(Color::srgba(0.15, 0.15, 0.15, 0.9)),
        ScoreText,
        ));
}
pub fn update_score_ui(
    player: Single<&Player>,
    mut query: Query<&mut Text, With<ScoreText>>
){
    for mut text in query.iter_mut() {
        text.0 = format!("Score: {}\nXP:{}\nXP to next level:{}\nPlayer HP: {}", player.score, player.xp, player.xp_to_next_level, player.health);
    }
}