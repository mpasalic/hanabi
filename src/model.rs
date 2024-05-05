use enum_map::Enum;
use strum_macros::EnumIter;

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Enum, EnumIter)]
pub enum CardFace {
    One,
    Two,
    Three,
    Four,
    Five,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Enum, EnumIter)]
pub enum CardSuit {
    Red,
    Green,
    Yellow,
    White,
    Blue,
}
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct PlayerIndex(pub usize);

#[derive(Debug, Copy, Clone)]
pub struct FromPlayerIndex(pub usize);

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct SlotIndex(pub usize);

#[derive(Debug, Clone)]
pub struct GameConfig {
    pub num_players: usize,
    pub hand_size: usize,
    pub num_fuses: u8,
    pub num_hints: u8,
    pub starting_player: PlayerIndex,
    pub seed: u64,
}

#[derive(Debug, Clone)]
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
}

#[derive(Debug, Clone)]
pub struct ClientGameState {
    pub draw_pile_count: u8, // TODO: maybe convert to a board with a draw pile and discard pile and organized sets
    pub played_cards: Vec<Card>, // TODO: organize by suit sets
    pub discard_pile: Vec<Card>,
    pub players: Vec<ClientPlayerView>,
    pub remaining_bomb_count: u8,
    pub remaining_hint_count: u8,
    pub current_player_index: PlayerIndex,
    pub turn: u8,              // todo maybe convert to player index
    pub last_turn: Option<u8>, // we end there
    pub outcome: Option<GameOutcome>,
}

#[derive(Debug, Clone)]
pub struct ClientVisibleCard {
    pub hints: Vec<Hint>,
    pub card: Card,
}

#[derive(Debug, Clone)]
pub enum ClientPlayerView {
    Me { hand: Vec<Option<HiddenSlot>> },
    Teammate { hand: Vec<Option<Slot>> },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Card {
    pub face: CardFace,
    pub suit: CardSuit,
    // hints: Vec<Hint>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum HintAction {
    SameSuit(CardSuit),
    SameFace(CardFace),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlayerAction {
    PlayCard(SlotIndex),
    DiscardCard(SlotIndex),
    GiveHint(PlayerIndex, HintAction),
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PlayedCardResult {
    Accepted,
    CompletedSet,
    Rejected,
}

#[derive(Debug, Clone)]
pub enum GameEffect {
    DrawCard(PlayerIndex, SlotIndex),
    RemoveCard(PlayerIndex, SlotIndex),
    AddToDiscrard(Card),
    PlaceOnBoard(Card),
    HintCard(PlayerIndex, SlotIndex, Hint),
    DecHint,
    IncHint,
    BurnFuse,
    NextTurn,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Hint {
    IsSuit(CardSuit),
    IsFace(CardFace),
    IsNotSuit(CardSuit),
    IsNotFace(CardFace),
}

#[derive(Debug, Clone)]
pub struct Slot {
    pub card: Card,
    pub hints: Vec<Hint>,
}

#[derive(Debug, Clone)]
pub struct HiddenSlot {
    pub hints: Vec<Hint>,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub hand: Vec<Option<Slot>>,
}

#[derive(Debug, Clone)]
pub enum GameOutcome {
    Win,
    Fail { score: usize },
}
