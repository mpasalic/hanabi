use shared::{
    client_logic::{HanabiGame, OnlinePlayer},
    model::*,
};

pub fn generate_minimal_test_game_state() -> GameStateSnapshot {
    GameStateSnapshot {
        player_snapshot: PlayerIndex(0),
        draw_pile_count: 0,
        played_cards: vec![],
        discard_pile: vec![],
        players: vec![
            ClientPlayerView::Me {
                hand: vec![None, None, None, None, None],
            },
            ClientPlayerView::Teammate {
                hand: vec![None, None, None, None, None],
            },
        ],
        remaining_bomb_count: 1,
        remaining_hint_count: 1,
        turn: PlayerIndex(0),
        num_rounds: 0,
        last_turn: None,
        outcome: None,
        log: vec![],
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

pub fn generate_test_game_state() -> GameStateSnapshot {
    GameStateSnapshot {
        player_snapshot: PlayerIndex(0),
        draw_pile_count: 20,
        played_cards: vec![
            Card {
                face: CardFace::One,
                suit: CardSuit::Red,
            },
            Card {
                face: CardFace::Two,
                suit: CardSuit::Red,
            },
            Card {
                face: CardFace::Three,
                suit: CardSuit::Red,
            },
            Card {
                face: CardFace::One,
                suit: CardSuit::Blue,
            },
            Card {
                face: CardFace::Five,
                suit: CardSuit::Green,
            },
        ],
        discard_pile: todo!(),
        players: todo!(),
        remaining_bomb_count: todo!(),
        remaining_hint_count: todo!(),
        turn: todo!(),
        num_rounds: todo!(),
        last_turn: todo!(),
        outcome: todo!(),
        log: todo!(),
        game_config: todo!(),
    }
    // let board_data = BoardProps {
    //     highest_played_card_for_suit: [
    //         (CardSuit::Red, CardFace::Three),
    //         (CardSuit::Blue, CardFace::One),
    //         (CardSuit::Green, CardFace::Five),
    //     ]
    //     .into_iter()
    //     .collect(),
    //     discards: vec![
    //         Card {
    //             face: CardFace::Four,
    //             suit: CardSuit::Blue,
    //         },
    //         Card {
    //             face: CardFace::Four,
    //             suit: CardSuit::Red,
    //         },
    //         Card {
    //             face: CardFace::Four,
    //             suit: CardSuit::Green,
    //         },
    //         Card {
    //             face: CardFace::One,
    //             suit: CardSuit::Blue,
    //         },
    //         Card {
    //             face: CardFace::Four,
    //             suit: CardSuit::Blue,
    //         },
    //         Card {
    //             face: CardFace::Four,
    //             suit: CardSuit::Red,
    //         },
    //         Card {
    //             face: CardFace::Four,
    //             suit: CardSuit::Red,
    //         },
    //         Card {
    //             face: CardFace::One,
    //             suit: CardSuit::Blue,
    //         },
    //         Card {
    //             face: CardFace::Four,
    //             suit: CardSuit::Blue,
    //         },
    //         Card {
    //             face: CardFace::Four,
    //             suit: CardSuit::Red,
    //         },
    //         Card {
    //             face: CardFace::Four,
    //             suit: CardSuit::Red,
    //         },
    //         Card {
    //             face: CardFace::One,
    //             suit: CardSuit::Blue,
    //         },
    //     ],
    //     draw_remaining: 20,
    //     hints_remaining: 3,
    //     fuse_remaining: 8,
    // };

    // let player_data = |i| PlayerNodeProps {
    //     name: format!("Player {}", i + 1),
    //     hand: Vec::from([
    //         SlotNodeProps {
    //             card: CardNodeProps::Empty,
    //             hints: vec![Hint::IsFace(CardFace::Five), Hint::IsNotSuit(CardSuit::Red)],
    //         },
    //         SlotNodeProps {
    //             card: CardNodeProps::SomeCard(None, None),
    //             hints: vec![Hint::IsNotFace(CardFace::Five), Hint::IsSuit(CardSuit::Red)],
    //         },
    //         SlotNodeProps {
    //             card: CardNodeProps::SomeCard(Some(CardFace::Two), None),
    //             hints: vec![Hint::IsNotFace(CardFace::Five), Hint::IsSuit(CardSuit::Red)],
    //         },
    //         SlotNodeProps {
    //             card: CardNodeProps::SomeCard(None, Some(CardSuit::Red)),
    //             hints: vec![],
    //         },
    //         SlotNodeProps {
    //             card: CardNodeProps::SomeCard(Some(CardFace::Four), Some(CardSuit::White)),
    //             hints: vec![Hint::IsNotFace(CardFace::Five), Hint::IsSuit(CardSuit::Red)],
    //         },
    //     ]),
    // };
}

pub fn generate_example_panic_case_1() -> HanabiGame {
    use shared::client_logic::ConnectionStatus::*;
    use shared::model::CardFace::*;
    use shared::model::CardSuit::*;
    use shared::model::ClientPlayerView::*;
    use shared::model::GameEffect::*;
    use shared::model::GameEvent::GameEffect;
    use shared::model::GameEvent::PlayerAction;
    use shared::model::Hint::*;
    use shared::model::HintAction::SameFace;
    use shared::model::HintAction::SameSuit;
    use shared::model::PlayerAction::*;
    use shared::model::*;

    HanabiGame::Started {
        session_id: "http://127.0.0.1:8080/?session_id=pink-cow-i4wC".to_string(),
        players: [
            OnlinePlayer {
                name: "mirza".to_string(),
                connection_status: Connected,
                is_host: false,
            },
            OnlinePlayer {
                name: "jeff".to_string(),
                connection_status: Disconnected,
                is_host: false,
            },
            OnlinePlayer {
                name: "".to_string(),
                connection_status: Disconnected,
                is_host: false,
            },
        ]
        .to_vec(),
        game_state: GameStateSnapshot {
            player_snapshot: PlayerIndex(0),
            draw_pile_count: 35,
            played_cards: [
                Card {
                    face: One,
                    suit: Yellow,
                },
                Card {
                    face: One,
                    suit: White,
                },
                Card {
                    face: One,
                    suit: Red,
                },
                Card {
                    face: Two,
                    suit: Red,
                },
            ]
            .to_vec(),
            discard_pile: [Card {
                face: Three,
                suit: White,
            }]
            .to_vec(),
            players: [
                Me {
                    hand: [
                        Some(HiddenSlot { hints: [].to_vec() }),
                        Some(HiddenSlot {
                            hints: [
                                IsSuit(White),
                                IsNotFace(One),
                                IsNotFace(One),
                                IsNotSuit(Blue),
                            ]
                            .to_vec(),
                        }),
                        Some(HiddenSlot {
                            hints: [
                                IsNotSuit(White),
                                IsNotFace(One),
                                IsNotFace(One),
                                IsNotSuit(Blue),
                            ]
                            .to_vec(),
                        }),
                        Some(HiddenSlot {
                            hints: [IsSuit(Blue)].to_vec(),
                        }),
                        Some(HiddenSlot {
                            hints: [
                                IsNotSuit(White),
                                IsNotFace(One),
                                IsNotFace(One),
                                IsSuit(Blue),
                            ]
                            .to_vec(),
                        }),
                    ]
                    .to_vec(),
                },
                Teammate {
                    hand: [
                        Some(Slot {
                            card: Card {
                                face: Three,
                                suit: Red,
                            },
                            hints: [IsSuit(Red), IsNotFace(Two)].to_vec(),
                        }),
                        Some(Slot {
                            card: Card {
                                face: Four,
                                suit: Green,
                            },
                            hints: [
                                IsNotFace(One),
                                IsNotFace(Five),
                                IsNotSuit(Red),
                                IsNotFace(Two),
                            ]
                            .to_vec(),
                        }),
                        Some(Slot {
                            card: Card {
                                face: One,
                                suit: Green,
                            },
                            hints: [].to_vec(),
                        }),
                        Some(Slot {
                            card: Card {
                                face: Five,
                                suit: Green,
                            },
                            hints: [IsNotFace(One), IsFace(Five), IsNotSuit(Red), IsNotFace(Two)]
                                .to_vec(),
                        }),
                        Some(Slot {
                            card: Card {
                                face: Four,
                                suit: Blue,
                            },
                            hints: [
                                IsNotFace(One),
                                IsNotFace(Five),
                                IsNotSuit(Red),
                                IsNotFace(Two),
                            ]
                            .to_vec(),
                        }),
                    ]
                    .to_vec(),
                },
            ]
            .to_vec(),
            remaining_bomb_count: 3,
            remaining_hint_count: 1,
            turn: PlayerIndex(1),
            num_rounds: 13,
            last_turn: None,
            outcome: None,
            log: [
                PlayerAction(PlayerIndex(0), GiveHint(PlayerIndex(1), SameFace(One))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(0), IsFace(One))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(1), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(2), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(3), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(4), IsNotFace(One))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(1), GiveHint(PlayerIndex(0), SameSuit(White))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(0), IsSuit(White))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(1), IsSuit(White))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(2), IsNotSuit(White))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(3), IsSuit(White))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(4), IsNotSuit(White))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(0))),
                PlayerAction(PlayerIndex(0), GiveHint(PlayerIndex(1), SameFace(Five))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(0), IsNotFace(Five))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(1), IsNotFace(Five))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(2), IsNotFace(Five))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(3), IsFace(Five))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(4), IsNotFace(Five))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(1), PlayCard(SlotIndex(0))),
                GameEffect(RemoveCard(PlayerIndex(1), SlotIndex(0))),
                GameEffect(PlaceOnBoard(Card {
                    face: One,
                    suit: Yellow,
                })),
                GameEffect(DrawCard(PlayerIndex(1), SlotIndex(0))),
                GameEffect(NextTurn(PlayerIndex(0))),
                PlayerAction(PlayerIndex(0), GiveHint(PlayerIndex(1), SameSuit(Red))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(0), IsSuit(Red))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(1), IsNotSuit(Red))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(2), IsSuit(Red))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(3), IsNotSuit(Red))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(4), IsNotSuit(Red))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(1), GiveHint(PlayerIndex(0), SameFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(0), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(1), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(2), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(3), IsFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(4), IsNotFace(One))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(0))),
                PlayerAction(PlayerIndex(0), PlayCard(SlotIndex(3))),
                GameEffect(RemoveCard(PlayerIndex(0), SlotIndex(3))),
                GameEffect(PlaceOnBoard(Card {
                    face: One,
                    suit: White,
                })),
                GameEffect(DrawCard(PlayerIndex(0), SlotIndex(3))),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(1), GiveHint(PlayerIndex(0), SameFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(0), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(1), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(2), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(3), IsFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(4), IsNotFace(One))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(0))),
                PlayerAction(PlayerIndex(0), PlayCard(SlotIndex(3))),
                GameEffect(RemoveCard(PlayerIndex(0), SlotIndex(3))),
                GameEffect(PlaceOnBoard(Card {
                    face: One,
                    suit: Red,
                })),
                GameEffect(DrawCard(PlayerIndex(0), SlotIndex(3))),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(1), GiveHint(PlayerIndex(0), SameSuit(Blue))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(0), IsNotSuit(Blue))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(1), IsNotSuit(Blue))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(2), IsNotSuit(Blue))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(3), IsSuit(Blue))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(4), IsSuit(Blue))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(0))),
                PlayerAction(PlayerIndex(0), GiveHint(PlayerIndex(1), SameFace(Two))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(0), IsNotFace(Two))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(1), IsNotFace(Two))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(2), IsFace(Two))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(3), IsNotFace(Two))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(4), IsNotFace(Two))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(1), PlayCard(SlotIndex(2))),
                GameEffect(RemoveCard(PlayerIndex(1), SlotIndex(2))),
                GameEffect(PlaceOnBoard(Card {
                    face: Two,
                    suit: Red,
                })),
                GameEffect(DrawCard(PlayerIndex(1), SlotIndex(2))),
                GameEffect(NextTurn(PlayerIndex(0))),
                PlayerAction(PlayerIndex(0), DiscardCard(SlotIndex(0))),
                GameEffect(RemoveCard(PlayerIndex(0), SlotIndex(0))),
                GameEffect(AddToDiscrard(Card {
                    face: Three,
                    suit: White,
                })),
                GameEffect(DrawCard(PlayerIndex(0), SlotIndex(0))),
                GameEffect(IncHint),
                GameEffect(NextTurn(PlayerIndex(1))),
            ]
            .to_vec(),
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

pub fn generate_example_panic_case_2() -> HanabiGame {
    use shared::client_logic::ConnectionStatus::*;
    use shared::model::CardFace::*;
    use shared::model::CardSuit::*;
    use shared::model::ClientPlayerView::*;
    use shared::model::GameEffect::*;
    use shared::model::GameEvent::GameEffect;
    use shared::model::GameEvent::PlayerAction;
    use shared::model::Hint::*;
    use shared::model::HintAction::SameFace;
    use shared::model::HintAction::SameSuit;
    use shared::model::PlayerAction::*;
    use shared::model::*;

    HanabiGame::Started {
        session_id: "http://127.0.0.1:8080/?session_id=pink-cow-i4wC".to_string(),
        players: [
            OnlinePlayer {
                name: "mirza".to_string(),
                connection_status: Connected,
                is_host: false,
            },
            OnlinePlayer {
                name: "jeff".to_string(),
                connection_status: Disconnected,
                is_host: false,
            },
            OnlinePlayer {
                name: "".to_string(),
                connection_status: Disconnected,
                is_host: false,
            },
        ]
        .to_vec(),
        game_state: GameStateSnapshot {
            player_snapshot: PlayerIndex(0),
            draw_pile_count: 35,
            played_cards: [
                Card {
                    face: One,
                    suit: Yellow,
                },
                Card {
                    face: One,
                    suit: White,
                },
                Card {
                    face: One,
                    suit: Red,
                },
                Card {
                    face: Two,
                    suit: Red,
                },
            ]
            .to_vec(),
            discard_pile: [Card {
                face: Three,
                suit: White,
            }]
            .to_vec(),
            players: [
                Me {
                    hand: [
                        Some(HiddenSlot { hints: [].to_vec() }),
                        Some(HiddenSlot {
                            hints: [
                                IsSuit(White),
                                IsNotFace(One),
                                IsNotFace(One),
                                IsNotSuit(Blue),
                            ]
                            .to_vec(),
                        }),
                        Some(HiddenSlot {
                            hints: [
                                IsNotSuit(White),
                                IsNotFace(One),
                                IsNotFace(One),
                                IsNotSuit(Blue),
                            ]
                            .to_vec(),
                        }),
                        Some(HiddenSlot {
                            hints: [IsSuit(Blue)].to_vec(),
                        }),
                        Some(HiddenSlot {
                            hints: [
                                IsNotSuit(White),
                                IsNotFace(One),
                                IsNotFace(One),
                                IsSuit(Blue),
                            ]
                            .to_vec(),
                        }),
                    ]
                    .to_vec(),
                },
                Teammate {
                    hand: [
                        Some(Slot {
                            card: Card {
                                face: Three,
                                suit: Red,
                            },
                            hints: [IsSuit(Red), IsNotFace(Two)].to_vec(),
                        }),
                        Some(Slot {
                            card: Card {
                                face: Four,
                                suit: Green,
                            },
                            hints: [
                                IsNotFace(One),
                                IsNotFace(Five),
                                IsNotSuit(Red),
                                IsNotFace(Two),
                            ]
                            .to_vec(),
                        }),
                        Some(Slot {
                            card: Card {
                                face: One,
                                suit: Green,
                            },
                            hints: [].to_vec(),
                        }),
                        Some(Slot {
                            card: Card {
                                face: Five,
                                suit: Green,
                            },
                            hints: [IsNotFace(One), IsFace(Five), IsNotSuit(Red), IsNotFace(Two)]
                                .to_vec(),
                        }),
                        Some(Slot {
                            card: Card {
                                face: Four,
                                suit: Blue,
                            },
                            hints: [
                                IsNotFace(One),
                                IsNotFace(Five),
                                IsNotSuit(Red),
                                IsNotFace(Two),
                            ]
                            .to_vec(),
                        }),
                    ]
                    .to_vec(),
                },
            ]
            .to_vec(),
            remaining_bomb_count: 3,
            remaining_hint_count: 1,
            turn: PlayerIndex(1),
            num_rounds: 13,
            last_turn: None,
            outcome: None,
            log: [
                PlayerAction(PlayerIndex(0), GiveHint(PlayerIndex(1), SameFace(One))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(0), IsFace(One))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(1), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(2), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(3), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(4), IsNotFace(One))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(1), GiveHint(PlayerIndex(0), SameSuit(White))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(0), IsSuit(White))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(1), IsSuit(White))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(2), IsNotSuit(White))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(3), IsSuit(White))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(4), IsNotSuit(White))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(0))),
                PlayerAction(PlayerIndex(0), GiveHint(PlayerIndex(1), SameFace(Five))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(0), IsNotFace(Five))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(1), IsNotFace(Five))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(2), IsNotFace(Five))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(3), IsFace(Five))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(4), IsNotFace(Five))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(1), PlayCard(SlotIndex(0))),
                GameEffect(RemoveCard(PlayerIndex(1), SlotIndex(0))),
                GameEffect(PlaceOnBoard(Card {
                    face: One,
                    suit: Yellow,
                })),
                GameEffect(DrawCard(PlayerIndex(1), SlotIndex(0))),
                GameEffect(NextTurn(PlayerIndex(0))),
                PlayerAction(PlayerIndex(0), GiveHint(PlayerIndex(1), SameSuit(Red))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(0), IsSuit(Red))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(1), IsNotSuit(Red))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(2), IsSuit(Red))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(3), IsNotSuit(Red))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(4), IsNotSuit(Red))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(1), GiveHint(PlayerIndex(0), SameFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(0), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(1), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(2), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(3), IsFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(4), IsNotFace(One))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(0))),
                PlayerAction(PlayerIndex(0), PlayCard(SlotIndex(3))),
                GameEffect(RemoveCard(PlayerIndex(0), SlotIndex(3))),
                GameEffect(PlaceOnBoard(Card {
                    face: One,
                    suit: White,
                })),
                GameEffect(DrawCard(PlayerIndex(0), SlotIndex(3))),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(1), GiveHint(PlayerIndex(0), SameFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(0), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(1), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(2), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(3), IsFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(4), IsNotFace(One))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(0))),
                PlayerAction(PlayerIndex(0), PlayCard(SlotIndex(3))),
                GameEffect(RemoveCard(PlayerIndex(0), SlotIndex(3))),
                GameEffect(PlaceOnBoard(Card {
                    face: One,
                    suit: Red,
                })),
                GameEffect(DrawCard(PlayerIndex(0), SlotIndex(3))),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(1), GiveHint(PlayerIndex(0), SameSuit(Blue))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(0), IsNotSuit(Blue))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(1), IsNotSuit(Blue))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(2), IsNotSuit(Blue))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(3), IsSuit(Blue))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(4), IsSuit(Blue))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(0))),
                PlayerAction(PlayerIndex(0), GiveHint(PlayerIndex(1), SameFace(Two))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(0), IsNotFace(Two))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(1), IsNotFace(Two))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(2), IsFace(Two))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(3), IsNotFace(Two))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(4), IsNotFace(Two))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(1), PlayCard(SlotIndex(2))),
                GameEffect(RemoveCard(PlayerIndex(1), SlotIndex(2))),
                GameEffect(PlaceOnBoard(Card {
                    face: Two,
                    suit: Red,
                })),
                GameEffect(DrawCard(PlayerIndex(1), SlotIndex(2))),
                GameEffect(NextTurn(PlayerIndex(0))),
                PlayerAction(PlayerIndex(0), DiscardCard(SlotIndex(0))),
                GameEffect(RemoveCard(PlayerIndex(0), SlotIndex(0))),
                GameEffect(AddToDiscrard(Card {
                    face: Three,
                    suit: White,
                })),
                GameEffect(DrawCard(PlayerIndex(0), SlotIndex(0))),
                GameEffect(IncHint),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(0), GiveHint(PlayerIndex(1), SameFace(One))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(0), IsFace(One))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(1), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(2), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(3), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(4), IsNotFace(One))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(1), GiveHint(PlayerIndex(0), SameSuit(White))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(0), IsSuit(White))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(1), IsSuit(White))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(2), IsNotSuit(White))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(3), IsSuit(White))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(4), IsNotSuit(White))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(0))),
                PlayerAction(PlayerIndex(0), GiveHint(PlayerIndex(1), SameFace(Five))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(0), IsNotFace(Five))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(1), IsNotFace(Five))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(2), IsNotFace(Five))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(3), IsFace(Five))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(4), IsNotFace(Five))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(1), PlayCard(SlotIndex(0))),
                GameEffect(RemoveCard(PlayerIndex(1), SlotIndex(0))),
                GameEffect(PlaceOnBoard(Card {
                    face: One,
                    suit: Yellow,
                })),
                GameEffect(DrawCard(PlayerIndex(1), SlotIndex(0))),
                GameEffect(NextTurn(PlayerIndex(0))),
                PlayerAction(PlayerIndex(0), GiveHint(PlayerIndex(1), SameSuit(Red))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(0), IsSuit(Red))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(1), IsNotSuit(Red))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(2), IsSuit(Red))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(3), IsNotSuit(Red))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(4), IsNotSuit(Red))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(1), GiveHint(PlayerIndex(0), SameFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(0), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(1), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(2), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(3), IsFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(4), IsNotFace(One))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(0))),
                PlayerAction(PlayerIndex(0), PlayCard(SlotIndex(3))),
                GameEffect(RemoveCard(PlayerIndex(0), SlotIndex(3))),
                GameEffect(PlaceOnBoard(Card {
                    face: One,
                    suit: White,
                })),
                GameEffect(DrawCard(PlayerIndex(0), SlotIndex(3))),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(1), GiveHint(PlayerIndex(0), SameFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(0), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(1), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(2), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(3), IsFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(4), IsNotFace(One))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(0))),
                PlayerAction(PlayerIndex(0), PlayCard(SlotIndex(3))),
                GameEffect(RemoveCard(PlayerIndex(0), SlotIndex(3))),
                GameEffect(PlaceOnBoard(Card {
                    face: One,
                    suit: Red,
                })),
                GameEffect(DrawCard(PlayerIndex(0), SlotIndex(3))),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(1), GiveHint(PlayerIndex(0), SameSuit(Blue))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(0), IsNotSuit(Blue))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(1), IsNotSuit(Blue))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(2), IsNotSuit(Blue))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(3), IsSuit(Blue))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(4), IsSuit(Blue))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(0))),
                PlayerAction(PlayerIndex(0), GiveHint(PlayerIndex(1), SameFace(Two))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(0), IsNotFace(Two))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(1), IsNotFace(Two))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(2), IsFace(Two))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(3), IsNotFace(Two))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(4), IsNotFace(Two))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(1), PlayCard(SlotIndex(2))),
                GameEffect(RemoveCard(PlayerIndex(1), SlotIndex(2))),
                GameEffect(PlaceOnBoard(Card {
                    face: Two,
                    suit: Red,
                })),
                GameEffect(DrawCard(PlayerIndex(1), SlotIndex(2))),
                GameEffect(NextTurn(PlayerIndex(0))),
                PlayerAction(PlayerIndex(0), DiscardCard(SlotIndex(0))),
                GameEffect(RemoveCard(PlayerIndex(0), SlotIndex(0))),
                GameEffect(AddToDiscrard(Card {
                    face: Three,
                    suit: White,
                })),
                GameEffect(DrawCard(PlayerIndex(0), SlotIndex(0))),
                GameEffect(IncHint),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(0), GiveHint(PlayerIndex(1), SameFace(One))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(0), IsFace(One))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(1), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(2), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(3), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(4), IsNotFace(One))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(1), GiveHint(PlayerIndex(0), SameSuit(White))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(0), IsSuit(White))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(1), IsSuit(White))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(2), IsNotSuit(White))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(3), IsSuit(White))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(4), IsNotSuit(White))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(0))),
                PlayerAction(PlayerIndex(0), GiveHint(PlayerIndex(1), SameFace(Five))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(0), IsNotFace(Five))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(1), IsNotFace(Five))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(2), IsNotFace(Five))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(3), IsFace(Five))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(4), IsNotFace(Five))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(1), PlayCard(SlotIndex(0))),
                GameEffect(RemoveCard(PlayerIndex(1), SlotIndex(0))),
                GameEffect(PlaceOnBoard(Card {
                    face: One,
                    suit: Yellow,
                })),
                GameEffect(DrawCard(PlayerIndex(1), SlotIndex(0))),
                GameEffect(NextTurn(PlayerIndex(0))),
                PlayerAction(PlayerIndex(0), GiveHint(PlayerIndex(1), SameSuit(Red))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(0), IsSuit(Red))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(1), IsNotSuit(Red))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(2), IsSuit(Red))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(3), IsNotSuit(Red))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(4), IsNotSuit(Red))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(1), GiveHint(PlayerIndex(0), SameFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(0), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(1), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(2), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(3), IsFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(4), IsNotFace(One))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(0))),
                PlayerAction(PlayerIndex(0), PlayCard(SlotIndex(3))),
                GameEffect(RemoveCard(PlayerIndex(0), SlotIndex(3))),
                GameEffect(PlaceOnBoard(Card {
                    face: One,
                    suit: White,
                })),
                GameEffect(DrawCard(PlayerIndex(0), SlotIndex(3))),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(1), GiveHint(PlayerIndex(0), SameFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(0), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(1), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(2), IsNotFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(3), IsFace(One))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(4), IsNotFace(One))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(0))),
                PlayerAction(PlayerIndex(0), PlayCard(SlotIndex(3))),
                GameEffect(RemoveCard(PlayerIndex(0), SlotIndex(3))),
                GameEffect(PlaceOnBoard(Card {
                    face: One,
                    suit: Red,
                })),
                GameEffect(DrawCard(PlayerIndex(0), SlotIndex(3))),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(1), GiveHint(PlayerIndex(0), SameSuit(Blue))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(0), IsNotSuit(Blue))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(1), IsNotSuit(Blue))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(2), IsNotSuit(Blue))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(3), IsSuit(Blue))),
                GameEffect(HintCard(PlayerIndex(0), SlotIndex(4), IsSuit(Blue))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(0))),
                PlayerAction(PlayerIndex(0), GiveHint(PlayerIndex(1), SameFace(Two))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(0), IsNotFace(Two))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(1), IsNotFace(Two))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(2), IsFace(Two))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(3), IsNotFace(Two))),
                GameEffect(HintCard(PlayerIndex(1), SlotIndex(4), IsNotFace(Two))),
                GameEffect(DecHint),
                GameEffect(NextTurn(PlayerIndex(1))),
                PlayerAction(PlayerIndex(1), PlayCard(SlotIndex(2))),
                GameEffect(RemoveCard(PlayerIndex(1), SlotIndex(2))),
                GameEffect(PlaceOnBoard(Card {
                    face: Two,
                    suit: Red,
                })),
                GameEffect(DrawCard(PlayerIndex(1), SlotIndex(2))),
                GameEffect(NextTurn(PlayerIndex(0))),
                PlayerAction(PlayerIndex(0), DiscardCard(SlotIndex(0))),
                GameEffect(RemoveCard(PlayerIndex(0), SlotIndex(0))),
                GameEffect(AddToDiscrard(Card {
                    face: Three,
                    suit: White,
                })),
                GameEffect(DrawCard(PlayerIndex(0), SlotIndex(0))),
                GameEffect(IncHint),
                GameEffect(NextTurn(PlayerIndex(1))),
            ]
            .to_vec(),
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
