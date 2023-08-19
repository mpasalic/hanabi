use colored::Colorize;
use enum_map::{enum_map, Enum, EnumMap};
use rand::Rng;
use std::collections::HashSet;
use std::fmt;
use std::io;

// Data Model

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Enum)]
enum CardFace {
    One,
    Two,
    Three,
    Four,
    Five,
}

const ALL_CARD_FACES: [CardFace; 5] = [
    CardFace::One,
    CardFace::Two,
    CardFace::Three,
    CardFace::Four,
    CardFace::Five,
];

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Enum)]
enum CardSuit {
    Red,
    Green,
    Yellow,
    White,
    Blue,
}

const ALL_CARD_SUITS: [CardSuit; 5] = [
    CardSuit::Red,
    CardSuit::Green,
    CardSuit::Yellow,
    CardSuit::White,
    CardSuit::Blue,
];

struct GameState {
    deck: Deck,
    board: Vec<Card>,
    discard: Vec<Card>,
    remaining_bomb_count: i32,
    remaining_hint_count: i32,
    turn: usize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct Card {
    face: CardFace,
    suit: CardSuit,
}

struct DeckConfig {
    faces: EnumMap<CardFace, i32>,
    suits: Vec<CardSuit>,
}

struct Deck {
    config: DeckConfig,
    cards: Vec<Card>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum HintType {
    SameSuit(CardSuit),
    SameFace(CardFace),
}

enum ActionType {
    PlayCard(usize),
    DiscardCard(usize),
    GiveHint(usize, HintType),
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum PlayResult {
    MatchedSet,
    CompletedSet,
    MismatchedSet,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Hint {
    IsSuit(CardSuit),
    IsFace(CardFace),
    IsNot(Box<Hint>),
}

struct Player {
    hand: [Option<Card>; 5],
    hints: [Vec<Hint>; 5],
    index: usize,
}

enum Game {
    Win,
    Fail(usize),
}

// Traits

trait CardKey {
    fn key(&self) -> &str;
}

trait ColoredCard {
    fn color(&self) -> colored::ColoredString;
    fn color_string(&self, string: String) -> colored::ColoredString;
    fn inactive_color(&self) -> colored::ColoredString;
}

impl CardKey for CardSuit {
    fn key(&self) -> &str {
        match self {
            CardSuit::Red => "R",
            CardSuit::Green => "G",
            CardSuit::Yellow => "Y",
            CardSuit::White => "W",
            CardSuit::Blue => "B",
        }
    }
}

impl ColoredCard for CardSuit {
    fn color_string(&self, string: String) -> colored::ColoredString {
        match self {
            CardSuit::Red => string.red(),
            CardSuit::Green => string.green(),
            CardSuit::Yellow => string.yellow(),
            CardSuit::White => string.white(),
            CardSuit::Blue => string.blue(),
        }
    }

    fn color(&self) -> colored::ColoredString {
        self.color_string(self.key().to_string()).bold()
    }

    fn inactive_color(&self) -> colored::ColoredString {
        self.key().to_string().dimmed()
    }
}

impl CardKey for CardFace {
    fn key(&self) -> &str {
        match self {
            CardFace::One => "1",
            CardFace::Two => "2",
            CardFace::Three => "3",
            CardFace::Four => "4",
            CardFace::Five => "5",
        }
    }
}

impl ColoredCard for CardFace {
    fn color_string(&self, string: String) -> colored::ColoredString {
        string.bold()
    }

    fn color(&self) -> colored::ColoredString {
        self.color_string(self.key().to_string())
    }

    fn inactive_color(&self) -> colored::ColoredString {
        self.key().to_string().dimmed()
    }
}

impl fmt::Display for CardSuit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.color())
    }
}

impl fmt::Display for CardFace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.key())
    }
}

impl Deck {
    fn new_standard_deck() -> Self {
        let face_map = enum_map! {
            CardFace::One => 3,
            CardFace::Two => 2,
            CardFace::Three => 2,
            CardFace::Four => 2,
            CardFace::Five => 1,
        };
        let config = DeckConfig {
            faces: face_map,
            suits: ALL_CARD_SUITS.to_vec(),
        };
        return Deck::new(config);
    }

    fn new(config: DeckConfig) -> Self {
        let mut cards: Vec<Card> = Vec::new();
        for (face, num) in config.faces {
            for _ in 0..num {
                for suit in config.suits.clone() {
                    cards.push(Card { face, suit })
                }
            }
        }

        Self { config, cards }
    }

    fn shuffle(&mut self) {
        for index in 0..self.cards.len() {
            let swap = rand::thread_rng().gen_range(index..self.cards.len());

            self.cards.swap(index, swap);
        }
    }

    fn draw(&mut self) -> Option<Card> {
        return self.cards.pop();
    }
}

impl Card {
    fn prev_face(&self) -> Option<CardFace> {
        match self.face {
            CardFace::One => None,
            CardFace::Two => Some(CardFace::One),
            CardFace::Three => Some(CardFace::Two),
            CardFace::Four => Some(CardFace::Three),
            CardFace::Five => Some(CardFace::Four),
        }
    }

    fn prev_card(&self) -> Option<Card> {
        if let Some(face) = self.prev_face() {
            Some(Card {
                face: face,
                suit: self.suit,
            })
        } else {
            return None;
        }
    }

    fn is_final_set_card(&self) -> bool {
        self.face == CardFace::Five
    }
    // fn suit(&self) -> char {
    //     let suit_index : usize = self.suit.try_into().unwrap();
    //     return SUITS[suit_index - 1]
    // }
    // fn same_number(&self, card: &Card) -> bool {
    //     return self.face == card.face
    // }
    // fn same_suit(&self, card: &Card) -> bool {
    //     return self.suit == card.suit
    // }

    // fn random() -> Self {

    //     let suit: i32 = rand::thread_rng()
    //         .gen_range(1..=NUM_SUITS);
    //     let face: i32 = rand::thread_rng()
    //         .gen_range(1..=NUM_FACES);

    //     Self { suit, face }
    // }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.suit.color_string(self.face.key().to_string()))
    }
}

fn fmt_card(card: Option<Card>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if let Some(card) = card {
        write!(f, "{}", card)
    } else {
        write!(f, "_")
    }
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_card(self.hand[0], f)?;
        fmt_card(self.hand[1], f)?;
        fmt_card(self.hand[2], f)?;
        fmt_card(self.hand[3], f)?;
        fmt_card(self.hand[4], f)?;
        return fmt::Result::Ok(());
        //write!(f, "[{:?},{:?},{:?},{:?},{:?}]", self.hand[0], self.hand[1], self.hand[2], self.hand[3], self.hand[4])
    }
}

impl Player {

    fn played(&mut self, card_index: usize) {
        self.hand[card_index] = None;
        self.hints[card_index] = Vec::new()
    }

    fn dealt(&mut self, card_index: usize, card: &Card) {
        self.hand[card_index] = Some(*card);
        self.hints[card_index] = Vec::new()
    }

    fn hinted(&mut self, hint: HintType) {
        for index in 0..5 {
            if let Some(Some(card)) = self.hand.get(index) {
                match (hint, card.suit, card.face) {
                    (HintType::SameSuit(hint_suit), card_suit, _) if hint_suit == card_suit => {
                        self.hints[index].push(Hint::IsSuit(hint_suit));
                    },
                    (HintType::SameFace(hint_face), _, card_face) if hint_face == card_face  => {
                        self.hints[index].push(Hint::IsFace(hint_face));
                    },
                    (HintType::SameSuit(suit), _, _) => {
                        self.hints[index].push(Hint::IsNot(Box::new(Hint::IsSuit(suit))));
                    },
                    (HintType::SameFace(face), _, _) => {
                        self.hints[index].push(Hint::IsNot(Box::new(Hint::IsFace(face))));
                    },
                }
            } else {
                panic!("invalid card index")
            }
        }
    }

    fn new(index: usize) -> Self {
        Self {
            hand: [None, None, None, None, None],
            hints: [Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()],
            index: index,
        }
    }

    /**
     * Your hand [hints]: [? ![Three, Five, Two][Green]] [5] [?2 ![Green]] [3] [?3 ![Green]]
     *
     * --new version--
     *
     * Card 1: RGYWB 1 2 3 4 5
     * Card 1: RYWB 1 4 3 4
     * Card 2: G5
     * Card 3:
     * Card 4: G3
     * Card 5:
     *
     */

    fn hints_to_string(&self) {
        for index in 0..5 {
            print!("Card {}: ", index);
            if let Some(Some(_card)) = self.hand.get(index) {
                let mut face_hints_set: HashSet<CardFace> = HashSet::new();
                let mut suit_hints_set: HashSet<CardSuit> = HashSet::new();

                for face in ALL_CARD_FACES {
                    if !self.hints[index].iter().any(|hint| match hint {
                        Hint::IsFace(hint_face) => *hint_face != face,
                        Hint::IsNot(not_hint) => match **not_hint {
                            Hint::IsFace(hint_not_face) => hint_not_face == face,
                            _ => false,
                        },
                        _ => false,
                    }) {
                        face_hints_set.insert(face.clone());
                    }
                }

                for suit in ALL_CARD_SUITS {
                    if !self.hints[index].iter().any(|hint| match hint {
                        Hint::IsSuit(suit_hint) => *suit_hint != suit,
                        Hint::IsNot(not_hint) => match **not_hint {
                            Hint::IsSuit(hint_not_suit) => hint_not_suit == suit,
                            _ => false,
                        },
                        _ => false,
                    }) {
                        suit_hints_set.insert(suit.clone());
                    }
                }

                let face_hints_output: String = ALL_CARD_FACES
                .into_iter()
                .map(|face| {
                    if face_hints_set.contains(&face) {
                        format!("{}", face.color())
                    } else {
                        format!("{}", face.inactive_color())
                    }
                })
                .collect();

                let suit_hints_output: String = ALL_CARD_SUITS
                .into_iter()
                .map(|suit| {
                    if suit_hints_set.contains(&suit) {
                        format!("{}", suit.color())
                    } else {
                        format!("{}", suit.inactive_color())
                    }
                })
                .collect();

                print!("{}\t{}", suit_hints_output, face_hints_output);
            } else {
                print!("<empty>");
            }
            println!("");
        }
    }
}

impl fmt::Display for GameState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "State {{").expect("format");
        write!(f, " bombs={}", self.remaining_bomb_count).expect("format");
        for _ in 0..self.remaining_bomb_count {
            write!(f, "X").expect("format");
        }
        write!(f, " hints={}", self.remaining_hint_count).expect("format");
        for _ in 0..self.remaining_hint_count {
            write!(f, "!").expect("format");
        }

        write!(f, " board=").expect("format");
        for suit in ALL_CARD_SUITS {
            for face in ALL_CARD_FACES {
                let card = Card { face, suit };
                if self.board.contains(&card) {
                    fmt_card(Some(card), f).expect("format");
                } else {
                    write!(f, "_").expect("format")
                }
            }
            write!(f, "|").expect("format");
        }

        write!(f, " discard=").expect("format");
        for card in self.discard.iter() {
            fmt_card(Some(*card), f).expect("format");
        }

        write!(f, " }}").expect("format");
        return fmt::Result::Ok(());
    }
}

impl GameState {
    fn play_result(&self, card_played: &Card) -> PlayResult {
        if let Some(required_card) = card_played.prev_card() {
            if !self.board.contains(&required_card) {
                return PlayResult::MismatchedSet;
            }
        }

        let has_same_card = self.board.contains(card_played);
        if has_same_card {
            return PlayResult::MismatchedSet;
        }

        if card_played.is_final_set_card() {
            PlayResult::CompletedSet
        } else {
            PlayResult::MatchedSet
        }
    }

    fn is_all_sets_complete(&self) -> bool {
        for face in ALL_CARD_FACES {
            for suit in ALL_CARD_SUITS {
                let card = Card { face, suit };
                if !self.board.contains(&card) {
                    return false;
                }
            }
        }
        return true;
    }
}

fn run_hanabi() -> Game {
    let num_players: usize = 5;

    let mut deck = Deck::new_standard_deck();
    deck.shuffle();

    let mut players: [Player; 5] = [
        Player::new(0),
        Player::new(1),
        Player::new(2),
        Player::new(3),
        Player::new(4),
    ];
    let mut game = GameState {
        deck: deck,
        discard: Vec::new(),
        board: Vec::new(),
        remaining_bomb_count: 3,
        remaining_hint_count: 10,
        turn: 0,
    };

    print!("Deck: ");
    for card in game.deck.cards.iter().filter(|card| card.suit == CardSuit::Red) {
        print!("{}", card);
    }
    println!("");

    for player_index in 0..5 {
        for card_index in 0..5 {
            players[player_index].hand[card_index] = game.deck.draw();
        }
    }

    println!("> Starting Game!");
    println!(
        "> P0:{{{}}} P1:{{{}}} P2:{{{}}} P3:{{{}}} P4:{{{}}}",
        players[0], players[1], players[2], players[3], players[4]
    );

    let mut last_round = None;

    while last_round == None || game.turn < last_round.unwrap() + 5 {
        println!(
            "############ ROUND #{} PLAYER #{} ############",
            game.turn / 5,
            game.turn % num_players
        );

        //let other_players = player.iter().filter(|player| player.index != current_player.index).collect::<Vec<&Player>>();
        let current_player: Option<&Player> = players.get(game.turn % num_players);
        if let None = current_player {
            panic!("no more players")
        }
        let current_player: &Player = if let Some(player) = current_player {
            player
        } else {
            panic!("no more players")
        };

        let next_action = player_turn(&current_player, &game, &players);

        //let (player, next_action) = game.next();
        let current_player: Option<&mut Player> = players.get_mut(game.turn % num_players);
        if let None = current_player {
            panic!("no more players")
        }

        match next_action {
            ActionType::PlayCard(index) => {
                // let current_player: &mut Player = if let Some(player) = current_player {
                //     player
                // } else {
                //     panic!("no more players")
                // };
                let current_player = current_player.unwrap();

                let card = current_player.hand[index].unwrap();
                let play_result = game.play_result(&card);

                match play_result {
                    PlayResult::MatchedSet => {
                        game.board.push(card);
                        println!("Success! {} matches set!", card);
                    }
                    PlayResult::CompletedSet => {
                        game.board.push(card);
                        game.remaining_hint_count = game.remaining_hint_count + 1;
                        println!("Superb! {} completes set!", card);
                    }
                    PlayResult::MismatchedSet => {
                        game.discard.push(card);
                        game.remaining_bomb_count = game.remaining_bomb_count - 1;
                        println!("Oops! {} doesn't match!", card);
                    }
                }

                current_player.played(index);

                if let Some(card) = game.deck.draw() {
                    current_player.dealt(index, &card);
                }
            }
            ActionType::DiscardCard(card_index) => {
                let current_player: &mut Player = if let Some(player) = current_player {
                    player
                } else {
                    panic!("no more players")
                };
                let card = current_player.hand[card_index].unwrap();

                game.remaining_hint_count = game.remaining_hint_count + 1;
                game.discard.push(card);

                current_player.played(card_index);

                if let Some(card) = game.deck.draw() {
                    current_player.dealt(card_index, &card);
                }
                println!("Discard card {}", card_index)
            }
            ActionType::GiveHint(hinted_player_index, hint_type) => {
                game.remaining_hint_count = game.remaining_hint_count - 1;

                if let Some(hinted_player) = players.get_mut(hinted_player_index) {
                    hinted_player.hinted(hint_type);
                    println!(
                        "Gave hint {:?} to player {}",
                        hint_type, hinted_player_index
                    );
                    hinted_player.hints_to_string();
                } else {
                    panic!("hint to no player")
                }
            }
        }

        if game.deck.cards.len() == 0 && last_round == None {
            last_round = Some(game.turn);
        }

        game.turn = game.turn + 1;

        if game.is_all_sets_complete() {
            return Game::Win;
        } else if game.remaining_bomb_count < 0 {
            return Game::Fail(game.board.len());
        }
    }
    return Game::Fail(game.board.len());
}

fn player_turn(current_player: &Player, game: &GameState, players: &[Player; 5]) -> ActionType {
    let current_index = current_player.index;
    println!("> Game State: {} ", game);

    print!("> Players: ");
    for index in 0..5 {
        if index != current_index {
            print!("P{}:{{{}}} ", index, players[index]);
        }
    }
    println!("");

    println!("> Your hand [hints]: ");
    current_player.hints_to_string();

    println!("> What is your move? [play: p (card_index), discard: d (card_index), hint: h (player_index) (suit:RGYWB|face:12345)]");

    let mut action_input = String::new();

    io::stdin()
        .read_line(&mut action_input)
        .expect("Failed to read line");

    let action_input = action_input.trim();
    let action_input = action_input.split(" ").collect::<Vec<&str>>();

    match action_input[..] {
        ["p", card_index] => match card_index.trim().parse() {
            Ok(card_index) => {
                if let Some(Some(_card)) = current_player.hand.get(card_index) {
                    ActionType::PlayCard(card_index)
                } else {
                    panic!("invalid card index")
                }
            }
            Err(_) => panic!("invalid card index"),
        },
        ["d", card_index] => match card_index.trim().parse() {
            Ok(card_index) => {
                if let Some(Some(_card)) = current_player.hand.get(card_index) {
                    ActionType::DiscardCard(card_index)
                } else {
                    panic!("invalid card index")
                }
            }
            Err(_) => panic!("invalid card index"),
        },
        ["h", player_index, suit_or_face] => match player_index.trim().parse() {
            Ok(player_index) => {
                if let Some(_player) = players.get(player_index) {
                    let hint = match suit_or_face {
                        "R" => HintType::SameSuit(CardSuit::Red),
                        "G" => HintType::SameSuit(CardSuit::Green),
                        "Y" => HintType::SameSuit(CardSuit::Yellow),
                        "W" => HintType::SameSuit(CardSuit::White),
                        "B" => HintType::SameSuit(CardSuit::Blue),
                        "1" => HintType::SameFace(CardFace::One),
                        "2" => HintType::SameFace(CardFace::Two),
                        "3" => HintType::SameFace(CardFace::Three),
                        "4" => HintType::SameFace(CardFace::Four),
                        "5" => HintType::SameFace(CardFace::Five),
                        _ => panic!("invalid suit or face"),
                    };
                    ActionType::GiveHint(player_index, hint)
                } else {
                    panic!("invalid card index")
                }
            }
            Err(_) => panic!("invalid card index"),
        },
        _ => panic!("invalid action"),
    }

    // for index in 0..5 {
    //     if let Some(Some(card)) = self.hand.get(index) {
    //         return Action::PlayCard(index, *card);
    //     }
    // }
}

fn main() {
    println!("{}", "Hanabi Simulator v0.1.0".blue());

    let result = run_hanabi();
    print!("Game ended: ");
    if let Game::Win = result {
        println!("Won!")
    } else {
        println!("Lost!")
    }
}

#[test]
fn it_works() {
    let result = 2 + 2;
    assert_eq!(result, 4);
}

#[test]
fn standard_deck_contains_all_cards() {
    let deck = Deck::new_standard_deck();

    assert_eq!(deck.cards.iter().filter(|card| card.face == CardFace::One).count(), 3 * 5);
    assert_eq!(deck.cards.iter().filter(|card| card.face == CardFace::Two).count(), 2 * 5);
    assert_eq!(deck.cards.iter().filter(|card| card.face == CardFace::Three).count(), 2 * 5);
    assert_eq!(deck.cards.iter().filter(|card| card.face == CardFace::Four).count(), 2 * 5);
    assert_eq!(deck.cards.iter().filter(|card| card.face == CardFace::Five).count(), 1 * 5);

    const NUM_CARDS_PER_SUIT : usize = 3 + 2 + 2 + 2 + 1;

    assert_eq!(deck.cards.iter().filter(|card| card.suit == CardSuit::Red).count(), NUM_CARDS_PER_SUIT);
    assert_eq!(deck.cards.iter().filter(|card| card.suit == CardSuit::Blue).count(), NUM_CARDS_PER_SUIT);
    assert_eq!(deck.cards.iter().filter(|card| card.suit == CardSuit::Green).count(), NUM_CARDS_PER_SUIT);
    assert_eq!(deck.cards.iter().filter(|card| card.suit == CardSuit::White).count(), NUM_CARDS_PER_SUIT);
    assert_eq!(deck.cards.iter().filter(|card| card.suit == CardSuit::Yellow).count(), NUM_CARDS_PER_SUIT);

    assert_eq!(
        deck.cards.contains(&Card {
            face: CardFace::One,
            suit: CardSuit::Red,
        }),
        true
    );
}

#[test]
fn deck_shuffles() {
    let mut deck = Deck::new_standard_deck();

    let original_cards = deck.cards.clone();

    deck.shuffle();

    assert_ne!(deck.cards, original_cards);
    assert_eq!(deck.cards.len(), original_cards.len());
}

#[test]
fn deck_draws() {
    let mut deck = Deck::new_standard_deck();
    let num_cards = deck.cards.len();

    let card = deck.draw();

    assert_eq!(deck.cards.len(), num_cards - 1);
    assert_eq!(card.is_some(), true);
}

#[test]
fn deck_empties() {
    let mut deck = Deck::new_standard_deck();
    let num_cards = deck.cards.len();

    for _ in 0..num_cards {
        deck.draw();
    }

    let empty_card = deck.draw();

    assert_eq!(!empty_card.is_some(), true);
    assert_eq!(deck.cards.len(), 0);
}

#[test]
fn player_functions() {
    let mut player = Player::new(0);

    assert_eq!(player.hand[0], None);
    assert_eq!(player.hand[1], None);
    assert_eq!(player.hand[2], None);
    assert_eq!(player.hand[3], None);
    assert_eq!(player.hand[4], None);

    assert_eq!(player.hints[0], Vec::new());
    assert_eq!(player.hints[1], Vec::new());
    assert_eq!(player.hints[2], Vec::new());
    assert_eq!(player.hints[3], Vec::new());
    assert_eq!(player.hints[4], Vec::new());

    player.dealt(0, &Card {
        face: CardFace::One,
        suit: CardSuit::Red,
    });

    player.dealt(1, &Card {
        face: CardFace::Two,
        suit: CardSuit::Blue,
    });

    player.dealt(2, &Card {
        face: CardFace::Three,
        suit: CardSuit::Green,
    });

    player.dealt(3, &Card {
        face: CardFace::Four,
        suit: CardSuit::Red,
    });

    player.dealt(4, &Card {
        face: CardFace::Four,
        suit: CardSuit::Blue,
    });

    assert_eq!(player.hand[0], Some(Card {
        face: CardFace::One,
        suit: CardSuit::Red,
    }));

    assert_eq!(player.hand[1], Some(Card {
        face: CardFace::Two,
        suit: CardSuit::Blue,
    }));

    assert_eq!(player.hand[2], Some(Card {
        face: CardFace::Three,
        suit: CardSuit::Green,
    }));

    assert_eq!(player.hand[3], Some(Card {
        face: CardFace::Four,
        suit: CardSuit::Red,
    }));

    assert_eq!(player.hand[4], Some(Card {
        face: CardFace::Four,
        suit: CardSuit::Blue,
    }));

    player.hinted(HintType::SameSuit(CardSuit::Blue));

    assert_eq!(player.hints[0].contains(&Hint::IsNot(Box::new(Hint::IsSuit(CardSuit::Blue)))), true);
    assert_eq!(player.hints[1].contains(&Hint::IsSuit(CardSuit::Blue)), true);
    assert_eq!(player.hints[2].contains(&Hint::IsNot(Box::new(Hint::IsSuit(CardSuit::Blue)))), true);
    assert_eq!(player.hints[3].contains(&Hint::IsNot(Box::new(Hint::IsSuit(CardSuit::Blue)))), true);
    assert_eq!(player.hints[4].contains(&Hint::IsSuit(CardSuit::Blue)), true);

    player.hinted(HintType::SameFace(CardFace::Four));

    assert_eq!(player.hints[0].contains(&Hint::IsNot(Box::new(Hint::IsFace(CardFace::Four)))), true);
    assert_eq!(player.hints[1].contains(&Hint::IsNot(Box::new(Hint::IsFace(CardFace::Four)))), true);
    assert_eq!(player.hints[2].contains(&Hint::IsNot(Box::new(Hint::IsFace(CardFace::Four)))), true);
    assert_eq!(player.hints[3].contains(&Hint::IsFace(CardFace::Four)), true);

    assert_eq!(player.hints[4].contains(&Hint::IsFace(CardFace::Four)), true);
    assert_eq!(player.hints[4].contains(&Hint::IsSuit(CardSuit::Blue)), true);

    player.played(4);
    
    assert_eq!(player.hand[4], None);
    assert_eq!(player.hints[4], Vec::new());

    player.dealt(4, &Card {
        face: CardFace::Four,
        suit: CardSuit::White,
    });

    assert_eq!(player.hand[4], Some(Card {
        face: CardFace::Four,
        suit: CardSuit::White,
    }));
    assert_eq!(player.hints[4], Vec::new());
}
