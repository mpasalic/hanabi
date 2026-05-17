//! Database access layer.
//!
//! All persistence goes through the `Database` trait. Production wires up
//! `PgDatabase` (sqlx + Postgres); tests can wire up an in-memory fake. This
//! keeps `LobbyServer` free of any direct sqlx coupling so it can be exercised
//! end-to-end without a Docker container.

use anyhow::{Context, Result};
use async_trait::async_trait;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use shared::model::{GameConfig, PlayerAction, PlayerIndex};
use sqlx::{FromRow, PgPool};

/// A persisted player action, abstracted away from any sqlx-specific row type
/// so that mock implementations don't need to depend on sqlx.
#[derive(Debug, Clone)]
pub struct PersistedAction {
    pub player_index: i16,
    pub player_action: PlayerAction,
}

/// All persistence operations the `LobbyServer` needs. Swap implementations to
/// run against Postgres in prod or an in-memory store in tests.
#[async_trait]
pub trait Database: Send + Sync {
    async fn generate_unique_game_id(&self) -> Result<String>;
    async fn get_game_config(&self, game_id: &str) -> Result<GameConfig>;
    async fn get_game_actions(&self, game_id: &str) -> Result<Vec<PersistedAction>>;
    async fn get_players(&self, game_id: &str) -> Result<Vec<String>>;
    async fn create_game(
        &self,
        game_id: &str,
        config: &GameConfig,
        players: &[String],
    ) -> Result<()>;
    async fn save_action(
        &self,
        game_id: &str,
        turn_id: u8,
        action: PlayerAction,
        player_index: usize,
    ) -> Result<()>;
}

// ---------------------------------------------------------------------------
// Postgres implementation
// ---------------------------------------------------------------------------

const COLORS: [&str; 11] = [
    "red", "blue", "green", "yellow", "orange", "purple", "pink", "grey", "white", "black", "teal",
];

const ANIMALS: [&str; 23] = [
    "dog", "cat", "parrot", "elephant", "leopard", "tiger", "bear", "monkey", "horse", "cow",
    "rabbit", "dolphin", "penguin", "snake", "fox", "giraffe", "kangaroo", "owl", "wolf",
    "crocodile", "plytapus", "raccoon", "chicken",
];

fn generate_random_string() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(4)
        .map(char::from)
        .collect()
}

fn generate_game_id() -> String {
    let mut rng = thread_rng();
    let color = COLORS[rng.gen_range(0..COLORS.len())];
    let animal = ANIMALS[rng.gen_range(0..ANIMALS.len())];
    let random_string = generate_random_string();
    format!("{color}-{animal}-{random_string}")
}

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

#[derive(Serialize, FromRow)]
struct GameConfigEntry {
    pub game_id: String,
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

pub struct PgDatabase {
    pool: PgPool,
}

impl PgDatabase {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl Database for PgDatabase {
    async fn generate_unique_game_id(&self) -> Result<String> {
        loop {
            let game_id = generate_game_id();
            let existing = sqlx::query_as::<_, GameConfigRow>(
                "SELECT * FROM game_config WHERE game_id = $1",
            )
            .bind(&game_id)
            .fetch_optional(&self.pool)
            .await
            .context("checking game_id uniqueness")?;
            if existing.is_none() {
                return Ok(game_id);
            }
        }
    }

    async fn get_game_config(&self, game_id: &str) -> Result<GameConfig> {
        let row = sqlx::query_as::<_, GameConfigRow>(
            "SELECT * FROM game_config WHERE game_id = $1",
        )
        .bind(game_id)
        .fetch_one(&self.pool)
        .await
        .with_context(|| format!("loading game_config for {game_id}"))?;

        Ok(GameConfig {
            num_players: row.num_players as usize,
            hand_size: row.hand_size as usize,
            num_fuses: row.num_fuses as u8,
            num_hints: row.num_hints as u8,
            starting_player: PlayerIndex(row.starting_player as usize),
            seed: row.seed as u64,
        })
    }

    async fn get_game_actions(&self, game_id: &str) -> Result<Vec<PersistedAction>> {
        let rows = sqlx::query_as::<_, GameLogRow>(
            "SELECT * FROM game_log WHERE game_id = $1 ORDER BY turn_id ASC, created_at ASC, id ASC",
        )
        .bind(game_id)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("loading game_log for {game_id}"))?;

        Ok(rows
            .into_iter()
            .map(|r| PersistedAction {
                player_index: r.player_index,
                player_action: r.player_action.0,
            })
            .collect())
    }

    async fn get_players(&self, game_id: &str) -> Result<Vec<String>> {
        let players = sqlx::query_as::<_, PlayerRow>(
            "SELECT * FROM player WHERE game_id = $1 ORDER BY player_index ASC",
        )
        .bind(game_id)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("loading players for {game_id}"))?;
        Ok(players.into_iter().map(|p| p.display_name).collect())
    }

    async fn create_game(
        &self,
        game_id: &str,
        game_config: &GameConfig,
        players: &[String],
    ) -> Result<()> {
        let new_game_config = NewGameConfig {
            game_id: game_id.to_string(),
            num_players: game_config.num_players as i16,
            hand_size: game_config.hand_size as i16,
            num_fuses: game_config.num_fuses as i16,
            num_hints: game_config.num_hints as i16,
            starting_player: game_config.starting_player.0 as i16,
            seed: game_config.seed as i64,
        };

        sqlx::query_as::<_, GameConfigEntry>(
            "INSERT INTO game_config (game_id, num_players, hand_size, num_fuses, num_hints, starting_player, seed) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING game_id",
        )
        .bind(&new_game_config.game_id)
        .bind(new_game_config.num_players)
        .bind(new_game_config.hand_size)
        .bind(new_game_config.num_fuses)
        .bind(new_game_config.num_hints)
        .bind(new_game_config.starting_player)
        .bind(new_game_config.seed)
        .fetch_one(&self.pool)
        .await
        .with_context(|| format!("inserting game_config for {game_id}"))?;

        for (index, player) in players.iter().enumerate() {
            sqlx::query(
                "INSERT INTO player (game_id, player_index, display_name) VALUES ($1, $2, $3)",
            )
            .bind(game_id)
            .bind(index as i16)
            .bind(player)
            .execute(&self.pool)
            .await
            .with_context(|| format!("inserting player {index} for {game_id}"))?;
        }

        Ok(())
    }

    async fn save_action(
        &self,
        game_id: &str,
        turn_id: u8,
        player_action: PlayerAction,
        player_index: usize,
    ) -> Result<()> {
        let player_action_json = serde_json::to_value(player_action)?;
        sqlx::query_as::<_, GameConfigEntry>(
            "INSERT INTO game_log (game_id, turn_id, player_index, player_action) VALUES ($1, $2, $3, $4) RETURNING game_id",
        )
        .bind(game_id)
        .bind(turn_id as i16)
        .bind(player_index as i16)
        .bind(&player_action_json)
        .fetch_one(&self.pool)
        .await
        .with_context(|| format!("saving action for {game_id} turn {turn_id}"))?;
        Ok(())
    }
}
