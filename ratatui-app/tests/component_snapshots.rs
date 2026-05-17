//! Component-level snapshot tests.
//!
//! Where `render_snapshots.rs` covers full screens via `HanabiApp::draw`,
//! these tests render a single `Node<'static>` subtree against a small buffer.
//! Same `insta` flow (`cargo insta review` to accept), but you can isolate a
//! `card_node`, `slot_node`, `player_node`, `board_node_tree`, etc.
//!
//! Pattern is always:
//!   1. Build the props
//!   2. Construct the `Node` via the public component function
//!   3. `compute_layout` against a known-size buffer
//!   4. `render_ref` into the buffer
//!   5. `insta::assert_snapshot!` the rendered text
//!
//! Copy one of the tests below to add coverage for a new component.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::WidgetRef;
use ratatui_app::components::{
    card_node, player_node, CardNodeProps, CardProps, CardRenderState, HintMode,
    PlayerNodeProps, PlayerRenderState, SlotNodeProps,
};
use ratatui_app::nodes::Node;
use shared::model::{CardFace, CardSuit, Hint, PlayerIndex, SlotIndex};
use taffy::geometry::Size as LayoutSize;
use taffy::style_helpers::length;

fn buffer_to_string(buf: &Buffer) -> String {
    let mut out = String::with_capacity((buf.area.width as usize + 1) * buf.area.height as usize);
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf.get(x, y).symbol());
        }
        out.push('\n');
    }
    out
}

/// Render a single Node into a buffer of the given size and return the result
/// as plain text. The component-snapshot equivalent of `render(...)` in
/// `render_snapshots.rs`.
fn render_node(mut node: Node<'static>, width: u16, height: u16) -> String {
    let mut buf = Buffer::empty(Rect {
        x: 0,
        y: 0,
        width,
        height,
    });
    node.compute_layout(LayoutSize {
        width: length(width as f32),
        height: length(height as f32),
    });
    node.render_ref(buf.area, &mut buf);
    buffer_to_string(&buf)
}

#[test]
fn renders_single_known_card() {
    let card = card_node(&CardProps {
        card: CardNodeProps::SomeCard(Some(CardFace::Three), Some(CardSuit::Red)),
        state: CardRenderState::Default,
    });
    insta::assert_snapshot!(render_node(card, 5, 3));
}

#[test]
fn renders_empty_slot() {
    let card = card_node(&CardProps {
        card: CardNodeProps::Empty,
        state: CardRenderState::Default,
    });
    insta::assert_snapshot!(render_node(card, 5, 3));
}

#[test]
fn renders_player_hand_with_hints() {
    // Five-slot hand: two visible cards, three placeholder slots; the first
    // slot has both a suit hint and a face hint.
    let hand = vec![
        SlotNodeProps {
            slot_index: SlotIndex(0),
            player_index: PlayerIndex(0),
            card: CardProps {
                card: CardNodeProps::SomeCard(Some(CardFace::One), Some(CardSuit::Red)),
                state: CardRenderState::Default,
            },
            card_id: 0,
            suit_hint: Some(Hint::IsSuit(CardSuit::Red)),
            face_hint: Some(Hint::IsFace(CardFace::One)),
            unique_hints: vec![Hint::IsSuit(CardSuit::Red), Hint::IsFace(CardFace::One)],
            unique_not_hints: vec![],
            all_hints: vec![Hint::IsSuit(CardSuit::Red), Hint::IsFace(CardFace::One)],
        },
        SlotNodeProps {
            slot_index: SlotIndex(1),
            player_index: PlayerIndex(0),
            card: CardProps {
                card: CardNodeProps::SomeCard(Some(CardFace::Two), Some(CardSuit::Blue)),
                state: CardRenderState::Default,
            },
            card_id: 1,
            suit_hint: None,
            face_hint: None,
            unique_hints: vec![],
            unique_not_hints: vec![Hint::IsNotFace(CardFace::Five)],
            all_hints: vec![Hint::IsNotFace(CardFace::Five)],
        },
        SlotNodeProps {
            slot_index: SlotIndex(2),
            player_index: PlayerIndex(0),
            card: CardProps {
                card: CardNodeProps::Empty,
                state: CardRenderState::Default,
            },
            card_id: 2,
            suit_hint: None,
            face_hint: None,
            unique_hints: vec![],
            unique_not_hints: vec![],
            all_hints: vec![],
        },
        SlotNodeProps {
            slot_index: SlotIndex(3),
            player_index: PlayerIndex(0),
            card: CardProps {
                card: CardNodeProps::Empty,
                state: CardRenderState::Default,
            },
            card_id: 3,
            suit_hint: None,
            face_hint: None,
            unique_hints: vec![],
            unique_not_hints: vec![],
            all_hints: vec![],
        },
        SlotNodeProps {
            slot_index: SlotIndex(4),
            player_index: PlayerIndex(0),
            card: CardProps {
                card: CardNodeProps::Empty,
                state: CardRenderState::Default,
            },
            card_id: 4,
            suit_hint: None,
            face_hint: None,
            unique_hints: vec![],
            unique_not_hints: vec![],
            all_hints: vec![],
        },
    ];

    let node = player_node(PlayerNodeProps {
        name: "alice".into(),
        hand,
        state: PlayerRenderState::Default,
        hint_mode: HintMode::NotHints,
        connected: true,
    });

    insta::assert_snapshot!(render_node(node, 40, 22));
}
