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
            ..default()
        },
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