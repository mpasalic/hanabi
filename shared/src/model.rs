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

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash, Enum, EnumIter)]
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone)]
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

// TODO Maybe use something like this for clarity
// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub enum GameStatus {
//     WaitingToStart,
//     Playing { turn_count: usize },
//     LastRound { turns_remaining: usize },
//     Finished(GameOutcome),
// }

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    pub history: Vec<GameEffect>,
    pub game_config: GameConfig,
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
    pub log: Vec<GameEvent>,
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

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum GameEffect {
    DrawCard(PlayerIndex, SlotIndex),
    RemoveCard(PlayerIndex, SlotIndex),
    AddToDiscrard(Card),
    PlaceOnBoard(Card),
    HintCard(PlayerIndex, SlotIndex, Hint),
    DecHint,
    IncHint,
    BurnFuse,
    NextTurn(PlayerIndex),
    MarkLastTurn(u8),
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Player {
    pub hand: Vec<Option<Slot>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameOutcome {
    Win,
    Fail { score: usize },
}
