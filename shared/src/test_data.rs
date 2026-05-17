//! Shared test fixtures and builders.
//!
//! Gated behind the `test-helpers` Cargo feature so it only compiles when
//! consumed by tests in any crate of the workspace. Production code does not
//! pull this in.

use crate::client_logic::{ConnectionStatus, HanabiGame, OnlinePlayer};
use crate::model::{
    Card, CardFace, CardSuit, ClientPlayerView, GameConfig, GameStateSnapshot, HiddenSlot, Hint,
    PlayerIndex, Slot, SlotIndex,
};

/// Smallest possible playable snapshot — two players with empty hands and a fresh
/// board. Useful when you don't care what the game state actually contains.
pub fn generate_minimal_test_game_state() -> GameStateSnapshot {
    GameStateSnapshot {
        this_client_player_index: PlayerIndex(0),
        draw_pile_count: 0,
        played_cards: vec![],
        discard_pile: vec![],
        players: vec![
            ClientPlayerView::Me {
                name: "p1".to_string(),
                hand: vec![None, None, None, None, None],
            },
            ClientPlayerView::Teammate {
                name: "p2".to_string(),
                hand: vec![None, None, None, None, None],
            },
        ],
        remaining_bomb_count: 1,
        remaining_hint_count: 1,
        current_turn_player_index: PlayerIndex(0),
        num_rounds: 0,
        last_turn: None,
        outcome: None,
        game_config: GameConfig {
            num_players: 2,
            hand_size: 5,
            num_fuses: 3,
            num_hints: 8,
            starting_player: PlayerIndex(0),
            seed: 0,
        },
    }
}

/// A mid-game snapshot captured from a real (panicking) production game. Useful
/// for rendering tests because it exercises played cards, discards, hints, and
/// the teammate hand projection in the same fixture.
pub fn generate_example_panic_case_1() -> HanabiGame {
    use CardFace::*;
    use CardSuit::*;
    use ClientPlayerView::*;
    use Hint::*;

    HanabiGame::Playing {
        log: vec![],
        session_id: "http://127.0.0.1:8080/?session_id=pink-cow-i4wC".to_string(),
        players: vec![
            OnlinePlayer {
                name: "mirza".to_string(),
                connection_status: ConnectionStatus::Connected,
                is_host: true,
            },
            OnlinePlayer {
                name: "jeff".to_string(),
                connection_status: ConnectionStatus::Disconnected,
                is_host: false,
            },
        ],
        game_state: GameStateSnapshot {
            this_client_player_index: PlayerIndex(0),
            draw_pile_count: 35,
            played_cards: vec![
                Card { face: One, suit: Yellow },
                Card { face: One, suit: White },
                Card { face: One, suit: Red },
                Card { face: Two, suit: Red },
            ],
            discard_pile: vec![Card { face: Three, suit: White }],
            players: vec![
                Me {
                    name: "Mirza".to_string(),
                    hand: vec![
                        Some(HiddenSlot { hints: vec![], draw_number: 0 }),
                        Some(HiddenSlot {
                            hints: vec![IsSuit(White), IsNotFace(One), IsNotFace(One), IsNotSuit(Blue)],
                            draw_number: 0,
                        }),
                        Some(HiddenSlot {
                            hints: vec![IsNotSuit(White), IsNotFace(One), IsNotFace(One), IsNotSuit(Blue)],
                            draw_number: 0,
                        }),
                        Some(HiddenSlot { hints: vec![IsSuit(Blue)], draw_number: 0 }),
                        Some(HiddenSlot {
                            hints: vec![IsNotSuit(White), IsNotFace(One), IsNotFace(One), IsSuit(Blue)],
                            draw_number: 0,
                        }),
                    ],
                },
                Teammate {
                    name: "Jeff".to_string(),
                    hand: vec![
                        Some(Slot {
                            card: Card { face: Three, suit: Red },
                            hints: vec![IsSuit(Red), IsNotFace(Two)],
                            draw_number: 0,
                        }),
                        Some(Slot {
                            card: Card { face: Four, suit: Green },
                            hints: vec![IsNotFace(One), IsNotFace(Five), IsNotSuit(Red), IsNotFace(Two)],
                            draw_number: 0,
                        }),
                        Some(Slot {
                            card: Card { face: One, suit: Green },
                            hints: vec![],
                            draw_number: 0,
                        }),
                        Some(Slot {
                            card: Card { face: Five, suit: Green },
                            hints: vec![IsNotFace(One), IsFace(Five), IsNotSuit(Red), IsNotFace(Two)],
                            draw_number: 0,
                        }),
                        Some(Slot {
                            card: Card { face: Four, suit: Blue },
                            hints: vec![IsNotFace(One), IsNotFace(Five), IsNotSuit(Red), IsNotFace(Two)],
                            draw_number: 0,
                        }),
                    ],
                },
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 1,
            current_turn_player_index: PlayerIndex(1),
            num_rounds: 13,
            last_turn: None,
            outcome: None,
            game_config: GameConfig {
                num_players: 2,
                hand_size: 5,
                num_fuses: 3,
                num_hints: 8,
                starting_player: PlayerIndex(0),
                seed: 0,
            },
        },
    }
}

/// Fluent builder over [`GameStateSnapshot`]. Start from a minimal valid state and
/// override only what each test cares about — keeps fixtures terse and prevents
/// every test from re-stating the same default fields.
///
/// ```ignore
/// let snap = GameStateSnapshotBuilder::new()
///     .with_played(CardSuit::Red, CardFace::One)
///     .with_bomb_count(2)
///     .build();
/// ```
pub struct GameStateSnapshotBuilder {
    snapshot: GameStateSnapshot,
}

impl Default for GameStateSnapshotBuilder {
    fn default() -> Self {
        Self {
            snapshot: generate_minimal_test_game_state(),
        }
    }
}

impl GameStateSnapshotBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_snapshot(snapshot: GameStateSnapshot) -> Self {
        Self { snapshot }
    }

    pub fn viewing_as(mut self, player: PlayerIndex) -> Self {
        self.snapshot.this_client_player_index = player;
        self
    }

    pub fn with_draw_pile_count(mut self, n: u8) -> Self {
        self.snapshot.draw_pile_count = n;
        self
    }

    pub fn with_played(mut self, suit: CardSuit, face: CardFace) -> Self {
        self.snapshot.played_cards.push(Card { face, suit });
        self
    }

    pub fn with_discard(mut self, suit: CardSuit, face: CardFace) -> Self {
        self.snapshot.discard_pile.push(Card { face, suit });
        self
    }

    pub fn with_bomb_count(mut self, n: u8) -> Self {
        self.snapshot.remaining_bomb_count = n;
        self
    }

    pub fn with_hint_count(mut self, n: u8) -> Self {
        self.snapshot.remaining_hint_count = n;
        self
    }

    pub fn current_turn(mut self, player: PlayerIndex) -> Self {
        self.snapshot.current_turn_player_index = player;
        self
    }

    pub fn with_my_hand(mut self, slots: Vec<Option<HiddenSlot>>) -> Self {
        if let Some(me) = self.snapshot.players.iter_mut().find_map(|p| match p {
            ClientPlayerView::Me { hand, .. } => Some(hand),
            _ => None,
        }) {
            *me = slots;
        }
        self
    }

    pub fn with_teammate_hand(mut self, name: &str, slots: Vec<Option<Slot>>) -> Self {
        if let Some(mate) = self.snapshot.players.iter_mut().find_map(|p| match p {
            ClientPlayerView::Teammate { name: n, hand } if n == name => Some(hand),
            _ => None,
        }) {
            *mate = slots;
        }
        self
    }

    pub fn build(self) -> GameStateSnapshot {
        self.snapshot
    }
}

/// Helper: wrap a snapshot into a `HanabiGame::Playing` with throwaway lobby
/// metadata. Convenient for render tests that don't care about the lobby fields.
pub fn playing_game(session_id: &str, snapshot: GameStateSnapshot) -> HanabiGame {
    let num_players = snapshot.game_config.num_players;
    let players = (0..num_players)
        .map(|i| OnlinePlayer {
            name: format!("p{}", i + 1),
            connection_status: ConnectionStatus::Connected,
            is_host: i == 0,
        })
        .collect();

    HanabiGame::Playing {
        session_id: session_id.to_string(),
        players,
        game_state: snapshot,
        log: vec![],
    }
}

/// Stable default `SlotIndex` constants — saves typing in inline tests.
pub const SLOT_0: SlotIndex = SlotIndex(0);
pub const SLOT_1: SlotIndex = SlotIndex(1);
pub const SLOT_2: SlotIndex = SlotIndex(2);
pub const SLOT_3: SlotIndex = SlotIndex(3);
pub const SLOT_4: SlotIndex = SlotIndex(4);
