use enum_map::Enum;
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

#[derive(
    Serialize, Deserialize, Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Enum, EnumIter,
)]
pub enum CardFace {
    One,
    Two,
    Three,
    Four,
    Five,
}

#[derive(
    Serialize, Deserialize, Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Enum, EnumIter,
)]
pub enum CardSuit {
    Red,
    Green,
    Yellow,
    White,
    Blue,
}
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub struct PlayerIndex(pub usize);

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct FromPlayerIndex(pub usize);

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Copy, Clone)]
pub struct SlotIndex(pub usize);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameConfig {
    pub num_players: usize,
    pub hand_size: usize,
    pub num_fuses: u8,
    pub num_hints: u8,
    pub starting_player: PlayerIndex,
    pub seed: u64,
}

impl GameConfig {
    pub fn new(num_players: usize, seed: u64) -> Self {
        Self {
            num_players,
            hand_size: match num_players {
                2 | 3 => 5,
                4 | 5 => 4,
                _ => 4, // error?
            },
            num_fuses: 3,
            num_hints: 8,
            starting_player: PlayerIndex(0),
            seed,
        }
    }
}

// TODO Maybe use something like this for clarity
// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub enum GameStatus {
//     WaitingToStart,
//     Playing { turn_count: usize },
//     LastRound { turns_remaining: usize },
//     Finished(GameOutcome),
// }

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct GameState {
    pub draw_pile: Vec<Card>, // TODO: maybe convert to a board with a draw pile and discard pile and organized sets
    pub played_cards: Vec<Card>, // TODO: organize by suit sets
    pub discard_pile: Vec<Card>,
    pub players: Vec<Player>,
    pub remaining_bomb_count: u8,
    pub remaining_hint_count: u8,
    pub turn: u8,              // todo maybe convert to player index
    pub last_turn: Option<u8>, // we end there
    pub outcome: Option<GameOutcome>,
    // pub status: GameStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GameEvent {
    PlayerAction {
        player_index: PlayerIndex,
        action: PlayerAction,
        effects: Vec<GameEffect>,
    },
    GameOver(GameOutcome),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameSnapshotEvent {
    pub event: GameEvent,
    pub snapshot: GameStateSnapshot,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameStateSnapshot {
    pub this_client_player_index: PlayerIndex,
    pub draw_pile_count: u8, // TODO: maybe convert to a board with a draw pile and discard pile and organized sets
    pub played_cards: Vec<Card>, // TODO: organize by suit sets
    pub discard_pile: Vec<Card>,
    pub players: Vec<ClientPlayerView>,
    pub remaining_bomb_count: u8,
    pub remaining_hint_count: u8,
    pub current_turn_player_index: PlayerIndex,
    pub num_rounds: u8,        // todo maybe convert to player index
    pub last_turn: Option<u8>, // we end there
    pub outcome: Option<GameOutcome>,

    pub game_config: GameConfig,
    // TODO
    // Player names
    //  - Connection status (eventually)
    // History / log
    // Status (waiting to start, playing)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClientVisibleCard {
    pub hints: Vec<Hint>,
    pub card: Card,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientPlayerView {
    Me {
        name: String,
        hand: Vec<Option<HiddenSlot>>,
    },
    Teammate {
        name: String,
        hand: Vec<Option<Slot>>,
    },
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Card {
    pub face: CardFace,
    pub suit: CardSuit,
    // hints: Vec<Hint>,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
pub enum HintAction {
    SameSuit(CardSuit),
    SameFace(CardFace),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Copy)]
pub enum PlayerAction {
    PlayCard(SlotIndex),
    DiscardCard(SlotIndex),
    GiveHint(PlayerIndex, HintAction),
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
pub enum PlayedCardResult {
    Accepted,
    CompletedSet,
    Rejected,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEffect {
    DrawCard(PlayerIndex, SlotIndex),
    RemoveCard(PlayerIndex, SlotIndex),
    AddToDiscard(Card),
    PlaceOnBoard(Card),
    HintCard(PlayerIndex, SlotIndex, Hint),
    DecHint,
    IncHint,
    BurnFuse,
    NextTurn(u8),
    MarkLastTurn(u8),
    LastTurn,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Hint {
    IsSuit(CardSuit),
    IsFace(CardFace),
    IsNotSuit(CardSuit),
    IsNotFace(CardFace),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Slot {
    pub card: Card,
    pub hints: Vec<Hint>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HiddenSlot {
    pub hints: Vec<Hint>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Player {
    pub hand: Vec<Option<Slot>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameOutcome {
    Win,
    Fail { score: usize },
}
