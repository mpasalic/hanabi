//! In-memory `Database` impl for tests. Stores config / players / actions in
//! plain hash maps under a mutex — no SQL, no Docker, no migrations. Lets us
//! drive `LobbyServer` deterministically in unit tests.

use std::collections::HashMap;
use std::sync::Mutex;

use anyhow::Result;
use async_trait::async_trait;
use server::model::{Database, PersistedAction};
use shared::model::{GameConfig, PlayerAction};

struct Game {
    config: GameConfig,
    players: Vec<String>,
    actions: Vec<PersistedAction>,
}

#[derive(Default)]
pub struct MemDatabase {
    games: Mutex<HashMap<String, Game>>,
    next_id: Mutex<u64>,
}

impl MemDatabase {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Database for MemDatabase {
    async fn generate_unique_game_id(&self) -> Result<String> {
        let mut n = self.next_id.lock().unwrap();
        *n += 1;
        Ok(format!("test-game-{n:04}"))
    }

    async fn get_game_config(&self, game_id: &str) -> Result<GameConfig> {
        let games = self.games.lock().unwrap();
        games
            .get(game_id)
            .map(|g| g.config.clone())
            .ok_or_else(|| anyhow::anyhow!("game {game_id} not found"))
    }

    async fn get_game_actions(&self, game_id: &str) -> Result<Vec<PersistedAction>> {
        let games = self.games.lock().unwrap();
        Ok(games
            .get(game_id)
            .map(|g| g.actions.clone())
            .unwrap_or_default())
    }

    async fn get_players(&self, game_id: &str) -> Result<Vec<String>> {
        let games = self.games.lock().unwrap();
        Ok(games
            .get(game_id)
            .map(|g| g.players.clone())
            .unwrap_or_default())
    }

    async fn create_game(
        &self,
        game_id: &str,
        config: &GameConfig,
        players: &[String],
    ) -> Result<()> {
        let mut games = self.games.lock().unwrap();
        if games.contains_key(game_id) {
            anyhow::bail!("game {game_id} already exists");
        }
        games.insert(
            game_id.to_string(),
            Game {
                config: config.clone(),
                players: players.to_vec(),
                actions: vec![],
            },
        );
        Ok(())
    }

    async fn save_action(
        &self,
        game_id: &str,
        _turn_id: u8,
        action: PlayerAction,
        player_index: usize,
    ) -> Result<()> {
        let mut games = self.games.lock().unwrap();
        let game = games
            .get_mut(game_id)
            .ok_or_else(|| anyhow::anyhow!("game {game_id} not found"))?;
        game.actions.push(PersistedAction {
            player_index: player_index as i16,
            player_action: action,
        });
        Ok(())
    }
}

