use rand::{distributions::Alphanumeric, thread_rng, Rng};

use serde::{Deserialize, Serialize};
use shared::model::{GameConfig, PlayerAction, PlayerIndex};
use sqlx::{FromRow, PgPool};

#[derive(Deserialize, Serialize)]
struct NewGameConfig {
    game_id: String,
    num_players: i16,
    hand_size: i16,
    num_fuses: i16,
    num_hints: i16,
    starting_player: i16,
    seed: i64,
}

#[derive(Serialize, Deserialize, FromRow)]
struct GameConfigEntry {
    pub game_id: String,
}

const COLORS: [&str; 11] = [
    "red", "blue", "green", "yellow", "orange", "purple", "pink", "grey", "white", "black", "teal",
];

const ANIMALS: [&str; 23] = [
    "dog",
    "cat",
    "parrot",
    "elephant",
    "leopard",
    "tiger",
    "bear",
    "monkey",
    "horse",
    "cow",
    "rabbit",
    "dolphin",
    "penguin",
    "snake",
    "fox",
    "giraffe",
    "kangaroo",
    "owl",
    "wolf",
    "crocodile",
    "plytapus",
    "raccoon",
    "chicken",
];

fn generate_random_string() -> String {
    let rng = thread_rng();
    let random_string: String = rng
        .sample_iter(&Alphanumeric)
        .take(4)
        .map(char::from)
        .collect();
    random_string
}

fn generate_game_id() -> String {
    let mut rng = thread_rng();
    let color = COLORS[rng.gen_range(0..COLORS.len())];
    let animal = ANIMALS[rng.gen_range(0..ANIMALS.len())];
    let random_string = generate_random_string();

    format!("{color}-{animal}-{random_string}")
}

pub async fn generate_unique_game_id(pool: &PgPool) -> Result<String, sqlx::Error> {
    loop {
        let game_id = generate_game_id();

        let game_config =
            sqlx::query_as::<_, GameConfigRow>("SELECT * FROM game_config WHERE game_id = $1")
                .bind(&game_id)
                .fetch_optional(pool)
                .await?;

        if let None = game_config {
            return Ok(game_id);
        }
    }
}

#[derive(Serialize, FromRow)]
struct GameConfigRow {
    game_id: String,
    num_players: i16,
    hand_size: i16,
    num_fuses: i16,
    num_hints: i16,
    starting_player: i16,
    seed: i64,
}

#[derive(Serialize, FromRow)]
struct GameLogRow {
    pub game_id: String,
    pub turn_id: i16,
    pub player_index: i16,
    pub player_action: sqlx::types::Json<PlayerAction>,
}

#[derive(Serialize, FromRow)]
struct PlayerRow {
    pub game_id: String,
    pub player_index: i16,
    pub display_name: String,
}

pub async fn get_game_config(pool: &PgPool, game_id: String) -> Result<GameConfig, sqlx::Error> {
    let game_config =
        sqlx::query_as::<_, GameConfigRow>("SELECT * FROM game_config WHERE game_id = $1")
            .bind(&game_id)
            .fetch_one(pool)
            .await?;

    Ok(GameConfig {
        num_players: game_config.num_players as usize,
        hand_size: game_config.hand_size as usize,
        num_fuses: game_config.num_fuses as u8,
        num_hints: game_config.num_hints as u8,
        starting_player: PlayerIndex(game_config.starting_player as usize),
        seed: game_config.seed as u64,
    })
}

pub async fn get_game_actions(
    pool: &PgPool,
    game_id: String,
) -> Result<Vec<PlayerAction>, sqlx::Error> {
    let log = sqlx::query_as::<_, GameLogRow>(
        "SELECT * FROM game_log WHERE game_id = $1 ORDER BY turn_id ASC",
    )
    .bind(&game_id)
    .fetch_all(pool)
    .await?;

    let log: Vec<_> = log.into_iter().map(|log| log.player_action.0).collect();

    Ok(log)
}

pub async fn get_players(pool: &PgPool, game_id: String) -> Result<Vec<String>, sqlx::Error> {
    let players = sqlx::query_as::<_, PlayerRow>(
        "SELECT * FROM player WHERE game_id = $1 ORDER BY player_index ASC",
    )
    .bind(&game_id)
    .fetch_all(pool)
    .await?;

    let players: Vec<_> = players
        .into_iter()
        .map(|player| player.display_name)
        .collect();

    Ok(players)
}

pub async fn create_game(
    pool: &PgPool,
    game_id: String,
    game_config: &GameConfig,
    players: &Vec<String>,
) -> Result<String, sqlx::Error> {
    //let game_id = random();
    // xxxx => az45
    // blue, green, red, yello ...
    // cobra
    // blue-cobra-az45

    let new_game_config = NewGameConfig {
        // game_id: generate_id(),
        game_id,
        num_players: game_config.num_players as i16,
        hand_size: game_config.hand_size as i16,
        num_fuses: game_config.num_fuses as i16,
        num_hints: game_config.num_hints as i16,
        starting_player: game_config.starting_player.0 as i16,
        seed: game_config.seed as i64,
    };

    let game_id = match sqlx::query_as::<_, GameConfigEntry>("INSERT INTO game_config (game_id, num_players, hand_size, num_fuses, num_hints, starting_player, seed) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING game_id")
        .bind(&new_game_config.game_id)
        .bind(&new_game_config.num_players)
        .bind(&new_game_config.hand_size)
        .bind(&new_game_config.num_fuses)
        .bind(&new_game_config.num_hints)
        .bind(&new_game_config.starting_player)
        .bind(&new_game_config.seed)
        .fetch_one(pool)
        .await
    {
        Ok(todo) => todo.game_id,
        Err(e) => return Err(e),
    };

    for (index, player) in players.iter().enumerate() {
        sqlx::query_as::<_, _>(
            "INSERT INTO player (game_id, player_index, display_name) VALUES ($1, $2, $3) RETURNING player_index",
        )
        .bind(&game_id)
        .bind(index as i16)
        .bind(player)
        .fetch_one(pool).await?;
    }

    Ok(game_id)
}

pub async fn save_action(
    pool: &PgPool,
    game_id: &String,
    turn_id: u8,
    player_action: PlayerAction,
    player_index: usize,
) -> Result<(), sqlx::Error> {
    let player_action_json = serde_json::to_value(player_action).unwrap();

    sqlx::query_as::<_, GameConfigEntry>("INSERT INTO game_log (game_id, turn_id, player_index, player_action) VALUES ($1, $2, $3, $4) RETURNING game_id")
        .bind(&game_id)
        .bind(turn_id as i16)
        .bind(player_index as i16)
        .bind(&player_action_json)
        .fetch_one(pool)
        .await?;

    Ok(())
}
