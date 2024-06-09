use std::collections::HashMap;

use itertools::Itertools;
use shared::model::*;

use ratatui::{
    prelude::*,
    widgets::{
        block::{Position, Title},
        *,
    },
};
use taffy::{
    self,
    style_helpers::{auto, fr, length},
    AlignContent, AlignItems, FlexDirection, JustifyContent, Size,
};

use crate::{
    glyphs::{EQUALS_SIGN, NOT_EQUALS},
    hanabi_app::AppAction,
    nodes::{GridStack, HStack, LayoutRect, LayoutStyle, Node, NodeBuilder, Stack, VStack},
};

pub trait CardKey {
    fn key(&self) -> &'static str;
}

impl CardKey for CardSuit {
    fn key(&self) -> &'static str {
        match self {
            CardSuit::Red => "R",
            CardSuit::Green => "G",
            CardSuit::Yellow => "Y",
            CardSuit::White => "W",
            CardSuit::Blue => "B",
        }
    }
}

impl CardKey for CardFace {
    fn key(&self) -> &'static str {
        match self {
            CardFace::One => "1",
            CardFace::Two => "2",
            CardFace::Three => "3",
            CardFace::Four => "4",
            CardFace::Five => "5",
        }
    }
}

pub static BACKGROUND_COLOR: Color = Color::Rgb(36, 37, 47);
pub static SELECTION_COLOR: Color = Color::Rgb(117, 158, 179);
pub static TURN_COLOR: Color = Color::Rgb(239, 119, 189);
pub static NORMAL_TEXT: Color = Color::Rgb(255, 255, 255);
pub static ALMOST_WHITE: Color = Color::Rgb(200, 200, 200);
pub static BLOCK_COLOR: Color = Color::Rgb(160, 160, 160);
pub static DIM_TEXT: Color = Color::Rgb(100, 100, 100);
pub static DARK_TEXT: Color = Color::Rgb(50, 50, 60);

pub fn default_style() -> Style {
    Style::default().fg(NORMAL_TEXT).bg(BACKGROUND_COLOR)
}

pub fn colorize_suit(suit: CardSuit) -> Color {
    match suit {
        CardSuit::Red => RED_SUIT,
        CardSuit::Green => GREEEN_SUIT,
        CardSuit::Yellow => YELLOW_SUIT,
        CardSuit::White => WHITE_SUIT,
        CardSuit::Blue => BLUE_SUIT,
    }
}

pub static RED_SUIT: Color = Color::Rgb(235, 90, 78);
pub static GREEEN_SUIT: Color = Color::Rgb(113, 244, 120);
pub static YELLOW_SUIT: Color = Color::Rgb(238, 249, 137);
pub static WHITE_SUIT: Color = Color::Rgb(255, 255, 255);
pub static BLUE_SUIT: Color = Color::Rgb(90, 90, 245);

// Merged with the background color, but for performance reasons just made it static.
pub static RED_SUIT_DIM: Color = Color::Rgb(235 / 2 + 36 / 2, 90 / 2 + 37 / 2, 78 / 2 + 47 / 2);
pub static GREEEN_SUIT_DIM: Color =
    Color::Rgb(113 / 2 + 36 / 2, 244 / 2 + 37 / 2, 120 / 2 + 47 / 2);
pub static YELLOW_SUIT_DIM: Color =
    Color::Rgb(238 / 2 + 36 / 2, 249 / 2 + 37 / 2, 137 / 2 + 47 / 2);
pub static WHITE_SUIT_DIM: Color = Color::Rgb(255 / 2 + 36 / 2, 255 / 2 + 37 / 2, 255 / 2 + 47 / 2);
pub static BLUE_SUIT_DIM: Color = Color::Rgb(90 / 2 + 36 / 2, 90 / 2 + 37 / 2, 245 / 2 + 47 / 2);

pub fn colorize_suit_dim(suit: CardSuit) -> Color {
    match suit {
        CardSuit::Red => RED_SUIT_DIM,
        CardSuit::Green => GREEEN_SUIT_DIM,
        CardSuit::Yellow => YELLOW_SUIT_DIM,
        CardSuit::White => WHITE_SUIT_DIM,
        CardSuit::Blue => BLUE_SUIT_DIM,
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum HintMode {
    NotHints,
    AllPossible,
}

#[derive(Clone, Copy)]
pub enum CardNodeProps {
    Empty,
    SomeCard(Option<CardFace>, Option<CardSuit>),
    Discarded(Card),
}

pub struct SlotNodeProps {
    pub suit_hint: Option<Hint>,
    pub face_hint: Option<Hint>,
    pub unique_hints: Vec<Hint>,
    pub unique_not_hints: Vec<Hint>,
    pub all_hints: Vec<Hint>,
    pub card: CardNodeProps,
}
pub enum PlayerRenderState {
    CurrentTurn,
    CurrentSelection,
    Default,
}

pub struct PlayerNodeProps {
    pub name: String,
    pub hand: Vec<SlotNodeProps>,
    pub state: PlayerRenderState,
    pub hint_mode: HintMode,
}

pub struct BoardProps {
    pub highest_played_card_for_suit: HashMap<CardSuit, CardFace>,
    pub discards: Vec<Card>,
    pub draw_remaining: usize,
    pub hints_remaining: usize,
    pub fuse_remaining: usize,
}

pub fn padding(size: f32) -> taffy::Rect<taffy::LengthPercentage> {
    use taffy::Rect;

    Rect {
        left: length(size),
        right: length(size),
        top: length(size),
        bottom: length(size),
    }
}

pub fn margin(size: f32) -> taffy::Rect<taffy::LengthPercentageAuto> {
    use taffy::Rect;

    Rect {
        left: length(size),
        right: length(size),
        top: length(size),
        bottom: length(size),
    }
}

pub fn card_node(card_node: CardNodeProps) -> Node<'static> {
    let color = match card_node {
        CardNodeProps::SomeCard(_, Some(suit)) => colorize_suit(suit),
        CardNodeProps::Discarded(Card { suit, .. }) => colorize_suit_dim(suit),
        _ => Color::Gray,
    };

    let text = match card_node {
        CardNodeProps::Empty => " ".not_bold().fg(color),
        CardNodeProps::SomeCard(None, _) => "?".not_bold().fg(color),
        CardNodeProps::SomeCard(Some(f), _) => f.key().bold().fg(color),
        CardNodeProps::Discarded(Card { face: f, .. }) => f.key().not_bold().fg(color),
    };

    Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(match card_node {
            CardNodeProps::Discarded(_) => default_style().fg(color),
            _ => default_style().add_modifier(Modifier::BOLD).fg(color),
        })
        .style(default_style())
        .children(
            LayoutStyle {
                flex_direction: taffy::FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                justify_content: Some(JustifyContent::Center),
                flex_shrink: 1.,
                ..Block::default_layout()
            },
            [text.into()],
        )
}

pub fn hint_span_derive(hint: &Hint) -> Node<'static> {
    match hint {
        Hint::IsNotSuit(suit) => Span::styled(
            suit.key().to_string(),
            default_style().fg(colorize_suit_dim(*suit)).not_bold(),
        ),
        Hint::IsNotFace(face) => Span::styled(
            face.key().to_string(),
            default_style().fg(DIM_TEXT).not_bold(),
        ),
        Hint::IsSuit(suit) => Span::styled(
            suit.key().to_string(),
            default_style().fg(colorize_suit_dim(*suit)).not_bold(),
        ),
        Hint::IsFace(face) => Span::styled(
            face.key().to_string(),
            default_style().fg(DIM_TEXT).not_bold(),
        ),
    }
    .into()
}

pub fn hint_span(hint: &Hint) -> Node<'static> {
    match hint {
        Hint::IsNotSuit(suit) => Span::styled(
            suit.key().to_string(),
            default_style().fg(colorize_suit_dim(*suit)).not_bold(),
        ),
        Hint::IsNotFace(face) => Span::styled(
            face.key().to_string(),
            default_style().fg(DIM_TEXT).not_bold(),
        ),
        Hint::IsSuit(suit) => Span::styled(
            suit.key().to_string(),
            default_style().fg(colorize_suit(*suit)).bold(),
        ),
        Hint::IsFace(face) => Span::styled(
            face.key().to_string(),
            default_style().fg(ALMOST_WHITE).bold(),
        ),
    }
    .into()
}

pub fn hand_node(card_props: Vec<CardNodeProps>) -> Node<'static> {
    HStack::new().childs(card_props.into_iter().map(card_node).collect_vec())
}

pub fn rounded_block(title: Span<'static>) -> Block<'static> {
    Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .style(default_style())
}

pub fn header_block(title: Span<'static>) -> Block<'static> {
    Block::new()
        .borders(Borders::TOP)
        .border_type(BorderType::Plain)
        .title(title)
        .style(default_style())
}

pub static BOARD_SUIT_ORDER: [CardSuit; 5] = [
    CardSuit::Blue,
    CardSuit::Green,
    CardSuit::Red,
    CardSuit::White,
    CardSuit::Yellow,
];

pub static CARD_FACE_ORDER: [CardFace; 5] = [
    CardFace::One,
    CardFace::Two,
    CardFace::Three,
    CardFace::Four,
    CardFace::Five,
];

pub fn player_node(player_props: PlayerNodeProps) -> Node<'static> {
    let player_block = Block::new()
        .borders(Borders::ALL)
        .border_type(match player_props.state {
            PlayerRenderState::CurrentTurn => BorderType::Double,
            // PlayerRenderState::CurrentSelection => BorderType::QuadrantOutside,
            _ => BorderType::Rounded,
        })
        .border_style(match player_props.state {
            PlayerRenderState::CurrentTurn => Style::default().fg(TURN_COLOR),
            PlayerRenderState::CurrentSelection => {
                Style::default().fg(BACKGROUND_COLOR).bg(SELECTION_COLOR)
            }
            _ => Style::default().fg(Color::White),
        })
        .title(
            format!("{}", player_props.name.clone()).set_style(match player_props.state {
                PlayerRenderState::CurrentTurn => Style::default().bold(),
                PlayerRenderState::CurrentSelection => Style::default().bold().fg(BACKGROUND_COLOR),
                _ => Style::default(),
            }),
        )
        .title_alignment(Alignment::Center);

    player_block.children(
        LayoutStyle {
            flex_direction: taffy::FlexDirection::Column,
            size: taffy::Size {
                width: auto(),
                height: length(20.),
            },
            // padding: padding(1.),
            // margin: margin(1.),
            ..Block::default_layout()
        },
        [
            HStack::new().childs(
                player_props
                    .hand
                    .iter()
                    .map(|s| card_node(s.card.clone()))
                    .collect_vec(),
            ),
            Block::new()
                .borders(Borders::TOP)
                .border_type(BorderType::Double)
                .border_style(default_style().not_bold().fg(BLOCK_COLOR))
                .title_top(" hints ".set_style(default_style().fg(BLOCK_COLOR)))
                // .title_bottom(" \u{eab2} ".set_style(default_style().fg(BLOCK_COLOR)))
                .title_alignment(Alignment::Center)
                .layout(LayoutStyle {
                    size: taffy::Size {
                        width: auto(),
                        height: auto(),
                    },
                    ..Block::default_layout()
                })
                .child(
                    HStack::new().children(
                        LayoutStyle {
                            // justify_content: Some(JustifyContent::SpaceBetween),
                            padding: LayoutRect {
                                left: length(0.),
                                right: length(0.),
                                top: length(0.),
                                bottom: length(0.),
                            },
                            gap: Size {
                                width: length(2.),
                                height: length(0.),
                            },
                            ..HStack::default_layout()
                        },
                        player_props
                            .hand
                            .iter()
                            .map(|s| {
                                VStack::new().children(
                                    LayoutStyle {
                                        size: Size {
                                            width: length(1.),
                                            height: length(2.),
                                        },
                                        ..VStack::default_layout()
                                    },
                                    s.unique_hints.iter().map(hint_span).collect_vec(),
                                )
                            })
                            .collect_vec(),
                    ),
                ),
            Block::new()
                .borders(Borders::TOP)
                .border_type(BorderType::Plain)
                .border_style(default_style().fg(BLOCK_COLOR).not_bold())
                .title(
                    format!(
                        " {} ",
                        match player_props.hint_mode {
                            HintMode::NotHints => NOT_EQUALS,
                            HintMode::AllPossible => EQUALS_SIGN,
                        }
                    )
                    .set_style(default_style().fg(BLOCK_COLOR).not_bold()),
                )
                .title_alignment(Alignment::Center)
                .title_position(Position::Top)
                .layout(LayoutStyle {
                    size: taffy::Size {
                        width: auto(),
                        height: auto(),
                    },
                    ..Block::default_layout()
                })
                .child(
                    HStack::new().children(
                        LayoutStyle {
                            // justify_content: Some(JustifyContent::SpaceBetween),
                            padding: LayoutRect {
                                left: length(0.),
                                right: length(0.),
                                top: length(0.),
                                bottom: length(0.),
                            },
                            gap: Size {
                                width: length(2.),
                                height: length(0.),
                            },
                            ..HStack::default_layout()
                        },
                        player_props
                            .hand
                            .iter()
                            .map(|s| {
                                match player_props.hint_mode {
                                    HintMode::NotHints => VStack::new().children(
                                        LayoutStyle {
                                            size: Size {
                                                width: length(1.),
                                                height: length(8.),
                                            },
                                            ..VStack::default_layout()
                                        },
                                        s.unique_not_hints.iter().map(hint_span).collect_vec(),
                                    ),
                                    HintMode::AllPossible => {
                                        let possible_faces: Vec<_> = CARD_FACE_ORDER
                                            .into_iter()
                                            .filter(|possible_face| {
                                                if let Some(face_hint) = s.face_hint {
                                                    face_hint == Hint::IsFace(*possible_face)
                                                } else {
                                                    s.unique_not_hints.iter().all(|h| {
                                                        h != &Hint::IsNotFace(*possible_face)
                                                    })
                                                }
                                            })
                                            .collect();

                                        let possible_suits: Vec<_> = BOARD_SUIT_ORDER
                                            .into_iter()
                                            .filter(|possible_suit| {
                                                if let Some(suit_hint) = s.suit_hint {
                                                    suit_hint == Hint::IsSuit(*possible_suit)
                                                } else {
                                                    s.unique_not_hints.iter().all(|h| {
                                                        h != &Hint::IsNotSuit(*possible_suit)
                                                    })
                                                }
                                            })
                                            .collect();

                                        VStack::new().children(
                                            LayoutStyle {
                                                size: Size {
                                                    width: length(1.),
                                                    height: length(11.),
                                                },
                                                ..VStack::default_layout()
                                            },
                                            (CARD_FACE_ORDER.iter().map(|f| {
                                                if possible_faces
                                                    .iter()
                                                    .any(|possible_face| f == possible_face)
                                                {
                                                    hint_span_derive(&Hint::IsFace(*f))
                                                } else {
                                                    // hint_span(&Hint::IsNotFace(*f))
                                                    Span::from(" ").into()
                                                }
                                            }))
                                            .chain(BOARD_SUIT_ORDER.iter().map(|s| {
                                                if possible_suits
                                                    .iter()
                                                    .any(|possible_suit| s == possible_suit)
                                                {
                                                    hint_span_derive(&Hint::IsSuit(*s))
                                                } else {
                                                    // hint_span(&Hint::IsNotSuit(*s))
                                                    Span::from(" ").into()
                                                }
                                            }))
                                            .collect_vec(),
                                        )
                                    }
                                }
                            })
                            .collect_vec(),
                    ),
                )
                .touchable(AppAction::ChangeHintMode(
                    if player_props.hint_mode == HintMode::NotHints {
                        HintMode::AllPossible
                    } else {
                        HintMode::NotHints
                    },
                )),
        ],
    )
}

pub fn discarded_cards_tree(board_props: &BoardProps) -> Node<'static> {
    let card_key = |card: &Card| -> String { format!("{}{}", card.face.key(), card.suit.key()) };

    let grouped_discards = board_props
        .discards
        .iter()
        .copied()
        .sorted_by_key(card_key)
        .dedup_by_with_count(|a, b| card_key(a).cmp(&card_key(b)).is_eq())
        .map(|(num, c)| [CardNodeProps::Discarded(c)].repeat(num.min(3)))
        .collect_vec();

    HStack::new().children(
        LayoutStyle {
            justify_content: Some(JustifyContent::FlexEnd),
            align_items: Some(AlignItems::FlexStart),
            flex_wrap: taffy::FlexWrap::Wrap,
            align_content: Some(AlignContent::FlexStart),
            ..HStack::default_layout()
        },
        grouped_discards
            .into_iter()
            .map(|c| card_pile(FlexDirection::Column, c))
            .collect_vec(),
    )
}

pub fn played_cards_tree(board_props: &BoardProps) -> Node<'static> {
    HStack::new().children(
        LayoutStyle {
            size: Size {
                width: auto(),
                height: length(7.),
            },
            justify_content: Some(JustifyContent::Center),
            ..HStack::default_layout()
        },
        BOARD_SUIT_ORDER
            .iter()
            .map(|s| {
                let highest = board_props
                    .highest_played_card_for_suit
                    .get(&s)
                    .and_then(|highest| CARD_FACE_ORDER.iter().find_position(|&c| c == highest));

                VStack::new().children(
                    LayoutStyle {
                        position: taffy::Position::Relative,
                        size: Size {
                            width: length(3.),
                            height: length(3.),
                        },
                        ..VStack::default_layout()
                    },
                    if let Some((highest_index, _)) = highest {
                        vec![card_pile(
                            FlexDirection::Column,
                            (0..=highest_index)
                                .map(|i| {
                                    CardNodeProps::SomeCard(Some(CARD_FACE_ORDER[i]), Some(*s))
                                })
                                .collect_vec(),
                        )]
                    } else {
                        vec![card_node(CardNodeProps::Empty)]
                    },
                )
            })
            .collect_vec(),
    )
}

pub fn board_stats_node_tree(board_props: &BoardProps) -> Node<'static> {
    fn key_value_pairs<const N: usize>(pairs: [(String, String); N]) -> Node<'static> {
        HStack::new().children(
            LayoutStyle {
                gap: Size {
                    width: length(1.),
                    height: length(0.),
                },
                ..HStack::default_layout()
            },
            [
                VStack::new()
                    .layout(LayoutStyle {
                        align_items: Some(AlignItems::FlexEnd),
                        ..VStack::default_layout()
                    })
                    .childs_iter(
                        pairs
                            .clone()
                            .into_iter()
                            .map(|(k, _)| k.not_bold().fg(DIM_TEXT)),
                    ),
                VStack::new().children(
                    LayoutStyle {
                        ..VStack::default_layout()
                    },
                    pairs
                        .clone()
                        .into_iter()
                        .map(|(_, v)| v.not_bold().fg(NORMAL_TEXT).into())
                        .collect_vec(),
                ),
            ],
        )
    }

    let data = [
        (
            "hints:".to_string(),
            "\u{f017} ".repeat(board_props.hints_remaining as usize),
        ),
        (
            "bombs:".to_string(),
            "\u{f0691} ".repeat(board_props.fuse_remaining as usize),
        ),
    ];
    key_value_pairs(data)
}

pub fn card_pile(direction: FlexDirection, card_props: Vec<CardNodeProps>) -> Node<'static> {
    Stack::new().children(
        LayoutStyle {
            flex_direction: direction,
            position: taffy::Position::Relative,
            size: Size {
                width: length(
                    3. + match direction {
                        FlexDirection::Row | FlexDirection::RowReverse => card_props.len() as f32,
                        _ => 0.,
                    },
                ),
                height: length(
                    3. + match direction {
                        FlexDirection::Column | FlexDirection::ColumnReverse => {
                            card_props.len() as f32
                        }
                        _ => 0.,
                    },
                ),
            },
            ..Stack::default_layout()
        },
        card_props
            .into_iter()
            .enumerate()
            .map(|(i, card)| {
                let mut inset = taffy::Rect {
                    left: length(0.),
                    right: length(0.),
                    top: length(0.),
                    bottom: length(0.),
                };

                match direction {
                    FlexDirection::Row => inset.left = length(i as f32),
                    FlexDirection::Column => inset.top = length(i as f32),
                    FlexDirection::RowReverse => inset.right = length(i as f32),
                    FlexDirection::ColumnReverse => inset.bottom = length(i as f32),
                }

                Stack::new().children(
                    LayoutStyle {
                        position: taffy::Position::Absolute,
                        size: Size {
                            width: length(3.),
                            height: length(3.),
                        },
                        inset,
                        ..Stack::default_layout()
                    },
                    [card_node(card)],
                )
            })
            .collect_vec(),
    )
}

pub fn draw_pile(count: usize) -> Node<'static> {
    card_pile(
        FlexDirection::Row,
        [CardNodeProps::SomeCard(None, None)].repeat(count),
    )
}

pub fn board_node_tree(board_props: BoardProps) -> Node<'static> {
    use taffy::prelude::line;

    fn block_title(s: &str, alignment: Alignment, position: Position) -> Title {
        Title::from(s.fg(BLOCK_COLOR))
            .alignment(alignment)
            .position(position)
    }

    Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(default_style().fg(BLOCK_COLOR))
        .bg(BACKGROUND_COLOR)
        .title(block_title("Draw Pile", Alignment::Left, Position::Top))
        .title(block_title("Board", Alignment::Center, Position::Top))
        .title(block_title("Discards", Alignment::Right, Position::Top))
        .title(block_title("Stats", Alignment::Left, Position::Bottom))
        .children(
            LayoutStyle {
                padding: padding(1.),
                ..Block::default_layout()
            },
            [GridStack::new().children(
                LayoutStyle {
                    flex_grow: 1.,
                    grid_template_columns: vec![fr(1.), fr(1.), fr(1.)],
                    grid_template_rows: vec![auto()],
                    ..GridStack::default_layout()
                },
                [
                    VStack::new().children(
                        LayoutStyle {
                            grid_row: line(1),
                            grid_column: line(1),
                            justify_content: Some(JustifyContent::SpaceBetween),
                            ..VStack::default_layout()
                        },
                        [
                            draw_pile(board_props.draw_remaining),
                            board_stats_node_tree(&board_props)
                                .append_layout(|l| LayoutStyle { ..l }),
                        ],
                    ),
                    played_cards_tree(&board_props).append_layout(|l| LayoutStyle {
                        grid_row: line(1),
                        grid_column: line(2),
                        ..l
                    }),
                    discarded_cards_tree(&board_props).append_layout(|l| LayoutStyle {
                        grid_row: line(1),
                        grid_column: line(3),
                        ..l
                    }),
                ],
            )],
        )
}

pub fn game_log_tree(log: &Vec<String>) -> Node {
    let lines: Vec<Span> = log
        .iter()
        .map(|line| Span::from(format!("{}", line)).style(default_style().fg(NORMAL_TEXT)))
        .collect_vec();

    Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("Game Log")
        .children(
            LayoutStyle {
                ..VStack::default_layout()
            },
            lines.into_iter().map(|l| l.into()).collect_vec(),
        )
        .debug("game_log_tree")
}

pub fn game_action_tree(actions: Vec<String>) -> Node<'static> {
    HStack::new().children(
        LayoutStyle {
            ..HStack::default_layout()
        },
        actions
            .into_iter()
            .map(|a| {
                Span::from(a)
                    .style(default_style().bg(SELECTION_COLOR).fg(Color::White))
                    .into()
            })
            .collect_vec(),
    )
}
