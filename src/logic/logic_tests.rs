#[cfg(test)]
use super::*;

#[test]
fn standard_deck_contains_all_cards() {
    let deck = new_standard_deck();

    assert_eq!(
        deck.iter()
            .filter(|card| card.face == CardFace::One)
            .count(),
        3 * 5
    );
    assert_eq!(
        deck.iter()
            .filter(|card| card.face == CardFace::Two)
            .count(),
        2 * 5
    );
    assert_eq!(
        deck.iter()
            .filter(|card| card.face == CardFace::Three)
            .count(),
        2 * 5
    );
    assert_eq!(
        deck.iter()
            .filter(|card| card.face == CardFace::Four)
            .count(),
        2 * 5
    );
    assert_eq!(
        deck.iter()
            .filter(|card| card.face == CardFace::Five)
            .count(),
        1 * 5
    );

    const NUM_CARDS_PER_SUIT: usize = 3 + 2 + 2 + 2 + 1;

    assert_eq!(
        deck.iter()
            .filter(|card| card.suit == CardSuit::Red)
            .count(),
        NUM_CARDS_PER_SUIT
    );
    assert_eq!(
        deck.iter()
            .filter(|card| card.suit == CardSuit::Blue)
            .count(),
        NUM_CARDS_PER_SUIT
    );
    assert_eq!(
        deck.iter()
            .filter(|card| card.suit == CardSuit::Green)
            .count(),
        NUM_CARDS_PER_SUIT
    );
    assert_eq!(
        deck.iter()
            .filter(|card| card.suit == CardSuit::White)
            .count(),
        NUM_CARDS_PER_SUIT
    );
    assert_eq!(
        deck.iter()
            .filter(|card| card.suit == CardSuit::Yellow)
            .count(),
        NUM_CARDS_PER_SUIT
    );

    assert_eq!(
        deck.contains(&Card {
            face: CardFace::One,
            suit: CardSuit::Red,
        }),
        true
    );
}

#[test]
fn initial_game_state() {
    let game = GameState::start(5);
    let standard_deck_size = new_standard_deck().len();

    match game {
        Ok(game) => {
            assert_eq!(game.players.len(), 5, "wrong num of players");
            assert_eq!(game.discard_pile, vec![], "cards already discarded");
            assert_eq!(game.played_cards, vec![], "cards already played");
            assert_eq!(game.draw_pile.len(), standard_deck_size - 5 * 5);
            assert_eq!(game.turn, 0, "turn wrongly incremented");
            assert_eq!(game.last_turn, None, "last turn already marked");
            assert_eq!(
                game.current_player_index(),
                PlayerIndex(0),
                "wrong starting player"
            );
            assert_eq!(game.remaining_bomb_count, 3, "wrong bomb count");
            assert_eq!(game.remaining_hint_count, 10, "wrong hint count");
        }
        Err(_) => assert!(false, "game failed to be created"),
    }
}

#[test]
fn player_functions() {
    // let mut player = Player::new(0);

    // assert_eq!(player.hand.len(), 0);

    // player.dealt(
    //     0,
    //     &Card {
    //         face: CardFace::One,
    //         suit: CardSuit::Red,
    //     },
    // );

    // player.dealt(
    //     1,
    //     &Card {
    //         face: CardFace::Two,
    //         suit: CardSuit::Blue,
    //     },
    // );

    // player.dealt(
    //     2,
    //     &Card {
    //         face: CardFace::Three,
    //         suit: CardSuit::Green,
    //     },
    // );

    // player.dealt(
    //     3,
    //     &Card {
    //         face: CardFace::Four,
    //         suit: CardSuit::Red,
    //     },
    // );

    // player.dealt(
    //     4,
    //     &Card {
    //         face: CardFace::Four,
    //         suit: CardSuit::Blue,
    //     },
    // );

    // assert_eq!(
    //     player.hand[0],
    //     (Card {
    //         face: CardFace::One,
    //         suit: CardSuit::Red,
    //     }, Vec::new())
    // );

    // assert_eq!(
    //     player.hand[1],
    //     (Card {
    //         face: CardFace::Two,
    //         suit: CardSuit::Blue,
    //     }, Vec::new())
    // );

    // assert_eq!(
    //     player.hand[2],
    //     (Card {
    //         face: CardFace::Three,
    //         suit: CardSuit::Green,
    //     }, Vec::new())
    // );

    // assert_eq!(
    //     player.hand[3],
    //     (Card {
    //         face: CardFace::Four,
    //         suit: CardSuit::Red,
    //     }, Vec::new())
    // );

    // assert_eq!(
    //     player.hand[4],
    //     (Card {
    //         face: CardFace::Four,
    //         suit: CardSuit::Blue,
    //     }, Vec::new())
    // );

    // player.hinted(HintAction::SameSuit(CardSuit::Blue));

    // assert_eq!(
    //     player.hand[0],
    //     (Card {
    //         face: CardFace::One,
    //         suit: CardSuit::Red,
    //     }, Vec::from([Hint::IsNot(Box::new(Hint::IsSuit(CardSuit::Blue)))]))
    // );

    // assert_eq!(
    //     player.hand[1],
    //     (Card {
    //         face: CardFace::Two,
    //         suit: CardSuit::Blue,
    //     }, Vec::from([Hint::IsSuit(CardSuit::Blue)]))
    // );

    // assert_eq!(
    //     player.hand[2],
    //     (Card {
    //         face: CardFace::Three,
    //         suit: CardSuit::Green,
    //     }, Vec::new())
    // );

    // assert_eq!(
    //     player.hand[3],
    //     (Card {
    //         face: CardFace::Four,
    //         suit: CardSuit::Red,
    //     }, Vec::new())
    // );

    // assert_eq!(
    //     player.hand[4],
    //     (Card {
    //         face: CardFace::Four,
    //         suit: CardSuit::Blue,
    //     }, Vec::new())
    // );

    // assert_eq!(
    //     player.hints[0].contains(&Hint::IsNot(Box::new(Hint::IsSuit(CardSuit::Blue)))),
    //     true
    // );
    // assert_eq!(
    //     player.hints[1].contains(&Hint::IsSuit(CardSuit::Blue)),
    //     true
    // );
    // assert_eq!(
    //     player.hints[2].contains(&Hint::IsNot(Box::new(Hint::IsSuit(CardSuit::Blue)))),
    //     true
    // );
    // assert_eq!(
    //     player.hints[3].contains(&Hint::IsNot(Box::new(Hint::IsSuit(CardSuit::Blue)))),
    //     true
    // );
    // assert_eq!(
    //     player.hints[4].contains(&Hint::IsSuit(CardSuit::Blue)),
    //     true
    // );

    // player.hinted(HintAction::SameFace(CardFace::Four));

    // assert_eq!(
    //     player.hints[0].contains(&Hint::IsNot(Box::new(Hint::IsFace(CardFace::Four)))),
    //     true
    // );
    // assert_eq!(
    //     player.hints[1].contains(&Hint::IsNot(Box::new(Hint::IsFace(CardFace::Four)))),
    //     true
    // );
    // assert_eq!(
    //     player.hints[2].contains(&Hint::IsNot(Box::new(Hint::IsFace(CardFace::Four)))),
    //     true
    // );
    // assert_eq!(
    //     player.hints[3].contains(&Hint::IsFace(CardFace::Four)),
    //     true
    // );

    // assert_eq!(
    //     player.hints[4].contains(&Hint::IsFace(CardFace::Four)),
    //     true
    // );
    // assert_eq!(
    //     player.hints[4].contains(&Hint::IsSuit(CardSuit::Blue)),
    //     true
    // );

    // player.played(4);

    // assert_eq!(player.hand[4], None);
    // assert_eq!(player.hints[4], Vec::new());

    // player.dealt(
    //     4,
    //     &Card {
    //         face: CardFace::Four,
    //         suit: CardSuit::White,
    //     },
    // );

    // assert_eq!(
    //     player.hand[4],
    //     Some(Card {
    //         face: CardFace::Four,
    //         suit: CardSuit::White,
    //     })
    // );
    // assert_eq!(player.hints[4], Vec::new());
}
