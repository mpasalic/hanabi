use std::any::Any;
use std::fmt;

use crate::hanabi_app::Binding;
use crate::key_code::KeyCode;
use crate::text::{text_measure_function, FontMetrics, TextContext, WritingMode};
use itertools::Itertools;
use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Rect};
use ratatui::text::{Span, Text};
use ratatui::widgets::block::Title;
use ratatui::widgets::{
    Block, BorderType, Borders, Paragraph, ScrollDirection, Scrollbar, ScrollbarOrientation,
    ScrollbarState, StatefulWidget, Widget, WidgetRef, Wrap,
};
use taffy::{
    compute_cached_layout, compute_flexbox_layout, compute_grid_layout, compute_leaf_layout,
    compute_root_layout, AvailableSpace, Cache, Display, FlexDirection, Layout, NodeId, Point,
    TraversePartialTree,
};
use taffy::{style_helpers::*, RoundTree};

// Ratatui and Taffy both have similarly named types. These are helper type aliases to avoid conflicts.
pub type LayoutStyle = taffy::Style;
pub type LayoutRect<T> = taffy::Rect<T>;
pub type LayoutSize<T> = taffy::Size<T>;

#[derive(Clone)]
pub struct TouchContext {
    pub touch_id: String,
}

#[derive(Debug, Clone, Copy)]
pub enum InteractionKind {
    Click,
    Keyboard(KeyCode),
    Scroll(ScrollDirection),
}

pub struct Interaction {
    pub kind: InteractionKind,
    pub payload: Box<dyn Any>,
}

#[derive(Clone)]
#[allow(dead_code)]
pub enum NodeKind<'a> {
    Flexbox,
    Grid,
    // Touchable(TouchContext),
    ScrollView(Text<'a>, i64),
    Text(Text<'a>),
    Span(Span<'a>),
    Block(Block<'a>),
}

impl fmt::Debug for NodeKind<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Flexbox => write!(f, "Flexbox"),
            Self::Grid => write!(f, "Grid"),
            // Self::Touchable(_) => write!(f, "Touchable"),
            Self::Text(_) => f.debug_tuple("Text").finish(),
            Self::Span(_) => f.debug_tuple("Span").finish(),
            Self::Block(_) => f.debug_tuple("Block").finish(),
            Self::ScrollView(_, _) => f.debug_tuple("ScrollView").finish(),
        }
    }
}

impl fmt::Debug for Node<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node")
            .field("kind", &self.kind)
            .field("debug", &self.debug_label)
            .field("child_nodes", &self.children)
            .finish()
    }
}

pub struct Node<'a> {
    pub kind: NodeKind<'a>,
    pub style: LayoutStyle,
    cache: Cache,
    unrounded_layout: Layout,
    pub final_layout: Layout,
    pub absolute_pos: Point<f32>,
    children: Vec<Node<'a>>,
    debug_label: Option<String>,
    pub interactions: Vec<Interaction>,
}

impl Default for Node<'_> {
    fn default() -> Self {
        Node {
            kind: NodeKind::Flexbox,
            style: LayoutStyle::default(),
            cache: Cache::new(),
            unrounded_layout: Layout::with_order(0),
            final_layout: Layout::with_order(0),
            absolute_pos: Point::zero(),
            children: Vec::new(),
            interactions: Vec::new(),
            debug_label: None,
        }
    }
}

#[allow(dead_code)]
impl<'a> Node<'a> {
    pub fn debug(mut self, label: &str) -> Self {
        self.debug_label = Some(label.to_string());
        self
    }

    pub fn new_flex(style: LayoutStyle) -> Node<'a> {
        Node {
            kind: NodeKind::Flexbox,
            style: LayoutStyle { ..style },
            ..Node::default()
        }
    }

    pub fn new_row(style: LayoutStyle) -> Node<'a> {
        Node {
            kind: NodeKind::Flexbox,
            style: LayoutStyle {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                ..style
            },
            ..Node::default()
        }
    }
    pub fn new_column(style: LayoutStyle) -> Node<'a> {
        Node {
            kind: NodeKind::Flexbox,
            style: LayoutStyle {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..style
            },
            ..Node::default()
        }
    }
    pub fn new_grid(style: LayoutStyle) -> Node<'a> {
        Node {
            kind: NodeKind::Grid,
            style: LayoutStyle {
                display: Display::Grid,
                ..style
            },
            ..Node::default()
        }
    }

    pub fn new_text<T>(style: LayoutStyle, text: T) -> Node<'a>
    where
        T: Into<Text<'a>>,
    {
        Node {
            kind: NodeKind::Text(text.into()),
            style,
            ..Node::default()
        }
    }

    pub fn new_scrollview<T>(style: LayoutStyle, text: T, scroll: i64) -> Node<'a>
    where
        T: Into<Text<'a>>,
    {
        Node {
            kind: NodeKind::ScrollView(text.into(), scroll),
            style,
            ..Node::default()
        }
    }

    pub fn new_span(style: LayoutStyle, span: Span<'a>) -> Node<'a> {
        Node {
            kind: NodeKind::Span(span),
            style,
            ..Node::default()
        }
    }

    pub fn new_block(style: LayoutStyle, block: Block<'a>) -> Node<'a> {
        Node {
            kind: NodeKind::Block(block),
            style: LayoutStyle {
                border: LayoutRect {
                    left: length(1.0),
                    right: length(1.0),
                    top: length(1.0),
                    bottom: length(1.0),
                },
                ..style
            },
            ..Node::default()
        }
    }

    pub fn new_titled_block<T>(style: LayoutStyle, title: T) -> Node<'a>
    where
        T: Into<Title<'a>>,
    {
        Node {
            kind: NodeKind::Block(
                Block::new()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(title),
            ),
            style: LayoutStyle {
                border: LayoutRect {
                    left: length(1.0),
                    right: length(1.0),
                    top: length(1.0),
                    bottom: length(1.0),
                },
                ..style
            },
            ..Node::default()
        }
    }

    pub fn append_child(&mut self, node: Node<'a>) {
        self.children.push(node);
    }

    #[inline(always)]
    pub fn node_from_id(&self, node_id: NodeId) -> &Node {
        if usize::from(node_id) == usize::MAX {
            return self;
        }
        &self.children[usize::from(node_id)]
    }

    fn update_node_layout(&'a mut self, node_id: NodeId, layout: Layout) {
        if usize::from(node_id) == usize::MAX {
            self.final_layout = layout;
            return;
        }

        self.node_from_id_mut(node_id).final_layout = layout;
    }

    #[inline(always)]
    fn node_from_id_mut(&mut self, node_id: NodeId) -> &mut Node<'a> {
        if usize::from(node_id) == usize::MAX {
            return self;
        }

        &mut self.children[usize::from(node_id)]
    }

    pub fn compute_layout(&mut self, available_space: LayoutSize<AvailableSpace>) {
        compute_root_layout(self, NodeId::from(usize::MAX), available_space);
        self.round_layout();
    }

    fn get_debug_label(&self) -> &'static str {
        match &self.kind {
            NodeKind::Flexbox => "FLEX",
            NodeKind::Grid => "GRID",
            NodeKind::Text(_) => "TEXT",
            NodeKind::Block(_) => "BLOCK",
            NodeKind::Span(_) => "SPAN",
            NodeKind::ScrollView(_, _) => "SCROLL",
            // NodeKind::Touchable(_) => "TOUCH",
        }
    }

    // forked from taffy::PrintTree because the original logic stack overflows given how nodes are referenced in this implementation
    pub fn print_tree(&self) {
        print_node_custom(self, NodeId::from(usize::MAX), false, String::new(), 0);

        /// Recursive function that prints each node in the tree
        fn print_node_custom(
            tree: &Node,
            node_id: NodeId,
            has_sibling: bool,
            lines_string: String,
            level: usize,
        ) {
            let layout = tree.final_layout;
            let display = tree.get_debug_label();
            let num_children = tree.children.len();
            let debug_name = tree.debug_label.clone().unwrap_or_default();

            let fork_string = if has_sibling {
                "├── "
            } else {
                "└── "
            };

            println!(
                "{lines}{fork} {display} [x: {x:<4} y: {y:<4} w: {width:<4} h: {height:<4} content_w: {content_width:<4} content_h: {content_height:<4} border: l:{bl} r:{br} t:{bt} b:{bb}, padding: l:{pl} r:{pr} t:{pt} b:{pb}] ({key:?}) {debug_name}",
                lines = lines_string,
                fork = fork_string,
                display = display,
                x = layout.location.x,
                y = layout.location.y,
                width = layout.size.width,
                height = layout.size.height,
                content_width = layout.content_size.width,
                content_height = layout.content_size.height,
                bl = layout.border.left,
                br = layout.border.right,
                bt = layout.border.top,
                bb = layout.border.bottom,
                pl = layout.padding.left,
                pr = layout.padding.right,
                pt = layout.padding.top,
                pb = layout.padding.bottom,
                key = node_id,
                debug_name = debug_name
            );

            let bar = if has_sibling { "│   " } else { "    " };
            let new_string = lines_string + bar;

            // Recurse into children
            for (index, id) in tree.child_ids(node_id).enumerate() {
                // println!("Looking at #{} ID: {:?}", index, child);
                let has_sibling = index < num_children - 1;
                print_node_custom(
                    tree.node_from_id(id),
                    id,
                    has_sibling,
                    new_string.clone(),
                    level + 1,
                );
            }
        }
    }

    // forked from taffy::RoundTree because the original logic stack overflows given how nodes are referenced in this implementation
    pub fn round_layout(&mut self) {
        return round_layout_inner(self, NodeId::from(usize::MAX), 0.0, 0.0);

        fn round(value: f32) -> f32 {
            value.round()
        }

        /// Recursive function to apply rounding to all descendents
        fn round_layout_inner(
            tree: &mut Node,
            node_id: NodeId,
            cumulative_x: f32,
            cumulative_y: f32,
        ) {
            let unrounded_layout = *tree.get_unrounded_layout(node_id);
            let mut layout = unrounded_layout;

            let cumulative_x = cumulative_x + unrounded_layout.location.x;
            let cumulative_y = cumulative_y + unrounded_layout.location.y;

            layout.location.x = round(unrounded_layout.location.x);
            layout.location.y = round(unrounded_layout.location.y);
            layout.size.width =
                round(cumulative_x + unrounded_layout.size.width) - round(cumulative_x);
            layout.size.height =
                round(cumulative_y + unrounded_layout.size.height) - round(cumulative_y);
            layout.scrollbar_size.width = round(unrounded_layout.scrollbar_size.width);
            layout.scrollbar_size.height = round(unrounded_layout.scrollbar_size.height);
            layout.border.left =
                round(cumulative_x + unrounded_layout.border.left) - round(cumulative_x);
            layout.border.right = round(cumulative_x + unrounded_layout.size.width)
                - round(cumulative_x + unrounded_layout.size.width - unrounded_layout.border.right);
            layout.border.top =
                round(cumulative_y + unrounded_layout.border.top) - round(cumulative_y);
            layout.border.bottom = round(cumulative_y + unrounded_layout.size.height)
                - round(
                    cumulative_y + unrounded_layout.size.height - unrounded_layout.border.bottom,
                );
            layout.padding.left =
                round(cumulative_x + unrounded_layout.padding.left) - round(cumulative_x);
            layout.padding.right = round(cumulative_x + unrounded_layout.size.width)
                - round(
                    cumulative_x + unrounded_layout.size.width - unrounded_layout.padding.right,
                );
            layout.padding.top =
                round(cumulative_y + unrounded_layout.padding.top) - round(cumulative_y);
            layout.padding.bottom = round(cumulative_y + unrounded_layout.size.height)
                - round(
                    cumulative_y + unrounded_layout.size.height - unrounded_layout.padding.bottom,
                );
            tree.set_final_layout(node_id, &layout);
            tree.absolute_pos = Point {
                x: cumulative_x,
                y: cumulative_y,
            };

            let child_count = tree.child_count(node_id);
            for index in 0..child_count {
                let child = tree.get_child_id(node_id, index);
                let node = tree.node_from_id_mut(child);
                round_layout_inner(node, NodeId::from(usize::MAX), cumulative_x, cumulative_y);
            }
        }
    }

    pub fn append_layout<F>(self, layout_fn: F) -> Node<'a>
    where
        F: Fn(LayoutStyle) -> LayoutStyle,
    {
        Node {
            style: layout_fn(self.style),
            ..self
        }
    }

    pub fn collect_bindings<EventType: Clone + 'static>(&self) -> Vec<Binding<EventType>> {
        let mut bindings = vec![];
        let node = self;

        let layout = node.final_layout;
        let absolute_pos = node.absolute_pos;

        for (interaction_kind, action) in self.interactions.iter().filter_map(|interaction| {
            interaction
                .payload
                .downcast_ref::<EventType>()
                .map(|event| (interaction.kind.clone(), event.clone()))
        }) {
            match interaction_kind {
                InteractionKind::Click => {
                    bindings.push(Binding::MouseClick {
                        action,
                        click_rect: Rect {
                            x: absolute_pos.x as u16,
                            y: absolute_pos.y as u16,
                            width: layout.size.width as u16,
                            height: layout.size.height as u16,
                        },
                    });
                }

                InteractionKind::Keyboard(keybinding) => {
                    bindings.push(Binding::Keyboard {
                        key_code: keybinding,
                        action,
                    });
                }

                InteractionKind::Scroll(direction) => {
                    bindings.push(Binding::Scroll {
                        direction,
                        action,
                        scroll_rect: Rect {
                            x: absolute_pos.x as u16,
                            y: absolute_pos.y as u16,
                            width: layout.size.width as u16,
                            height: layout.size.height as u16,
                        },
                    });
                }
            }
        }

        for child_id in node.child_ids(NodeId::from(usize::MAX)) {
            let child = node.node_from_id(child_id);
            bindings.extend(child.collect_bindings());
        }
        bindings
    }
}

impl<'a> WidgetRef for Node<'a> {
    fn render_ref(&self, area: ratatui::prelude::Rect, buf: &mut Buffer) {
        let layout_rect = self.final_layout;

        let layout_area = Rect {
            x: area.x + layout_rect.location.x as u16,
            y: area.y + layout_rect.location.y as u16,
            width: layout_rect.size.width as u16,
            height: layout_rect.size.height as u16,
        };

        let render_area = layout_area.intersection(buf.area);

        if !buf.area.contains(render_area.as_position()) || render_area.area() == 0 {
            println!("Warning: Skipping render of node {:?}", self.kind);
            return;
        }

        match &self.kind {
            // NodeKind::Touchable(_) => {}
            NodeKind::Flexbox => {}
            NodeKind::Grid => {}
            NodeKind::Text(text_context) => {
                Paragraph::new(text_context.clone())
                    .alignment(
                        text_context
                            .alignment
                            .unwrap_or(ratatui::layout::Alignment::Left),
                    )
                    .wrap(Wrap { trim: false })
                    .render(render_area, buf);
            }
            NodeKind::Span(span) => span.render_ref(render_area, buf),
            NodeKind::Block(block) => block.render_ref(render_area, buf),
            NodeKind::ScrollView(text_context, scroll_offset) => {
                // Note: For some reason, ratatui lets you scroll the content a full "view port height" beyond the content length
                // These adjustements are a workaround to that for better UX.
                let adjusted_content_length = text_context
                    .lines
                    .len()
                    .saturating_sub(render_area.height as usize);

                let max_scrolling = text_context
                    .lines
                    .len()
                    .saturating_sub(render_area.height as usize);

                let scroll_offset_adjusted =
                    (text_context
                        .lines
                        .len()
                        .saturating_sub(render_area.height as usize) as i64
                        - *scroll_offset)
                        .max(0)
                        .min(max_scrolling as i64) as usize;

                let mut scrollbar_state = ScrollbarState::new(adjusted_content_length)
                    .position(scroll_offset_adjusted)
                    .viewport_content_length(render_area.height as usize);
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("↑"))
                    .end_symbol(Some("↓"));
                let paragraph = Paragraph::new(text_context.clone())
                    .alignment(
                        text_context
                            .alignment
                            .unwrap_or(ratatui::layout::Alignment::Left),
                    )
                    .wrap(Wrap { trim: false })
                    .scroll((scroll_offset_adjusted as u16, 0))
                    .block(Block::new().borders(Borders::RIGHT));

                paragraph.render(render_area, buf);
                scrollbar.render(
                    render_area.inner(&Margin {
                        // using an inner vertical margin of 1 unit makes the scrollbar inside the block
                        vertical: 1,
                        horizontal: 0,
                    }),
                    buf,
                    &mut scrollbar_state,
                );
            }
        }

        for child_node in &self.children {
            child_node.render_ref(layout_area, buf);
        }
    }
}

pub struct ChildIter(std::ops::Range<usize>);
impl Iterator for ChildIter {
    type Item = NodeId;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|idx| NodeId::from(idx))
    }
}

impl<'a> taffy::TraversePartialTree for Node<'a>
where
    Self: 'a,
{
    type ChildIter<'b> = ChildIter   where
    Self: 'b;

    fn child_ids(&self, _node_id: NodeId) -> Self::ChildIter<'_> {
        ChildIter(0..self.children.len())
    }

    fn child_count(&self, _node_id: NodeId) -> usize {
        self.children.len()
    }

    fn get_child_id(&self, _node_id: NodeId, index: usize) -> NodeId {
        NodeId::from(index)
    }
}

impl<'a> taffy::TraverseTree for Node<'a> {}

impl<'a> taffy::LayoutPartialTree for Node<'a> {
    fn get_style(&self, node_id: NodeId) -> &LayoutStyle {
        &self.node_from_id(node_id).style
    }

    fn set_unrounded_layout(&mut self, node_id: NodeId, layout: &Layout) {
        self.node_from_id_mut(node_id).unrounded_layout = *layout;
    }

    fn get_cache_mut(&mut self, node_id: NodeId) -> &mut Cache {
        &mut self.node_from_id_mut(node_id).cache
    }

    fn compute_child_layout(
        &mut self,
        node_id: NodeId,
        inputs: taffy::tree::LayoutInput,
    ) -> taffy::tree::LayoutOutput {
        // println!(
        //     "[{}] Computing layout for node {:?} {:?}",
        //     self.debug_label.clone().unwrap_or_default(),
        //     self.get_debug_label(),
        //     node_id
        // );
        compute_cached_layout(self, node_id, inputs, |parent, node_id, inputs| {
            let node = parent.node_from_id_mut(node_id);
            let font_metrics = FontMetrics {
                char_width: 1.0,
                char_height: 1.0,
            };

            match &node.kind {
                // NodeKind::Touchable(_) => {
                //     compute_flexbox_layout(node, NodeId::from(usize::MAX), inputs)
                // }
                NodeKind::Flexbox => compute_flexbox_layout(node, NodeId::from(usize::MAX), inputs),
                NodeKind::Grid => compute_grid_layout(node, NodeId::from(usize::MAX), inputs),
                NodeKind::ScrollView(_paragraph, _) => {
                    // compute_leaf_layout(inputs, &node.style, |known_dimensions, available_space| {
                    //     let text_content = paragraph
                    //         .lines
                    //         .iter()
                    //         .map(|l| {
                    //             l.spans
                    //                 .iter()
                    //                 .map(|s| s.content.clone().into_owned())
                    //                 .collect_vec()
                    //                 .join("")
                    //         })
                    //         .collect_vec()
                    //         .join("\n");

                    //     text_measure_function(
                    //         known_dimensions,
                    //         available_space,
                    //         &TextContext {
                    //             text_content: text_content,
                    //             writing_mode: WritingMode::Horizontal,
                    //         },
                    //         &font_metrics,
                    //     )
                    // })
                    compute_flexbox_layout(node, NodeId::from(usize::MAX), inputs)
                }
                NodeKind::Text(paragraph) => {
                    compute_leaf_layout(inputs, &node.style, |known_dimensions, available_space| {
                        let text_content = paragraph
                            .lines
                            .iter()
                            .map(|l| {
                                l.spans
                                    .iter()
                                    .map(|s| s.content.clone().into_owned())
                                    .collect_vec()
                                    .join("")
                            })
                            .collect_vec()
                            .join("\n");

                        text_measure_function(
                            known_dimensions,
                            available_space,
                            &TextContext {
                                text_content: text_content,
                                writing_mode: WritingMode::Horizontal,
                            },
                            &font_metrics,
                        )
                    })
                }

                NodeKind::Span(span) => compute_leaf_layout(
                    inputs,
                    &node.style,
                    |_known_dimensions, _available_space| LayoutSize {
                        width: length(span.width() as f32),
                        height: length(1.),
                    },
                ),
                NodeKind::Block(_) => {
                    compute_flexbox_layout(node, NodeId::from(usize::MAX), inputs)
                }
            }
        })
    }
}

impl<'a> taffy::RoundTree for Node<'a> {
    fn get_unrounded_layout(&self, node_id: NodeId) -> &Layout {
        &self.node_from_id(node_id).unrounded_layout
    }

    fn set_final_layout(&mut self, node_id: NodeId, layout: &Layout) {
        self.node_from_id_mut(node_id).final_layout = *layout;
    }
}

impl<'a> taffy::PrintTree for Node<'a> {
    fn get_debug_label(&self, node_id: NodeId) -> &'static str {
        match self.node_from_id(node_id).kind {
            NodeKind::Flexbox => "FLEX",
            NodeKind::Grid => "GRID",
            NodeKind::Text(_) => "TEXT",
            NodeKind::Block(_) => "BLOCK",
            NodeKind::Span(_) => "SPAN",
            NodeKind::ScrollView(_, _) => "SCROLL",
            // NodeKind::Touchable(_) => "TOUCH",
        }
    }

    fn get_final_layout(&self, node_id: NodeId) -> &Layout {
        &self.node_from_id(node_id).final_layout
    }
}

// impl<'a> IntoIter<Node<'a>> for NodeBuilder<'a> {
//     fn into_iter(self) -> IntoIter<Node<'a>> {
//         self.child_nodes.into_iter()
//     }
// }

// impl Into<IntoIterator<Item = Node<'_>>> for Node<'_> {
//     fn into(self) -> IntoIter<Node<'_>> {
//         self.child_nodes.into_iter()
//     }
// }

pub trait ScrollViewBuilder<'a> {
    fn into_scroll(self, layout: LayoutStyle, scroll_offset: i64) -> Node<'a>;

    fn y_scroll(self, y_offset: i64) -> Node<'a>
    where
        Self: Sized,
    {
        self.into_scroll(
            LayoutStyle {
                overflow: taffy::Point {
                    x: taffy::Overflow::Visible,
                    y: taffy::Overflow::Scroll,
                },
                ..LayoutStyle::default()
            },
            y_offset,
        )
    }

    fn x_scroll(self, x_offset: i64) -> Node<'a>
    where
        Self: Sized,
    {
        self.into_scroll(
            LayoutStyle {
                overflow: taffy::Point {
                    x: taffy::Overflow::Scroll,
                    y: taffy::Overflow::Visible,
                },
                ..LayoutStyle::default()
            },
            x_offset,
        )
    }

    // fn x_scroll(self, y_offset: usize) -> Node<'a>
    // where
    //     Self: Sized,
    // {
    //     let mut node: Node = self.into();
    //     node.kind = NodeKind::ScrollView(Text::default(), offset);
    //     node
    // }
}

impl<'a> ScrollViewBuilder<'a> for Text<'a> {
    fn into_scroll(self, layout: LayoutStyle, offset: i64) -> Node<'a> {
        Node::new_scrollview(layout, self, offset)
    }
}

pub trait NodeBuilder<'a>: Into<Node<'a>> {
    fn default_layout() -> LayoutStyle {
        LayoutStyle::default()
    }

    fn node(self) -> Node<'a>
    where
        Self: Sized,
    {
        self.into()
    }

    fn layout(self, style: LayoutStyle) -> Node<'a>
    where
        Self: Sized,
    {
        let mut node: Node = self.into();
        node.style = style;
        node
    }

    fn childs<T>(self, children: T) -> Node<'a>
    where
        T: IntoIterator<Item = Node<'a>, IntoIter = std::vec::IntoIter<Node<'a>>>,
        // Self: Sized,
    {
        let mut node: Node = self.into();
        node.children.extend(children);
        node
    }

    fn childs_iter(self, children: impl Iterator<Item = impl Into<Node<'a>>>) -> Node<'a> {
        let mut node: Node = self.into();
        node.children.extend(children.map(|c| c.into()));
        node
    }

    fn child<T>(self, child: T) -> Node<'a>
    where
        T: Into<Node<'a>>,
        Self: Sized,
    {
        let mut node: Node = self.into();
        node.children.push(child.into());
        node
    }

    fn children<T>(self, style: LayoutStyle, children: T) -> Node<'a>
    where
        T: Into<Vec<Node<'a>>>,
        Self: Sized,
    {
        let mut node: Node = self.into();
        node.style = style;
        node.children.extend(children.into());
        node
    }

    fn touchable<UntypedEvent: Any + Sized>(self, event: UntypedEvent) -> Node<'a> {
        let mut node: Node = self.into();

        node.interactions.push(Interaction {
            kind: InteractionKind::Click,
            payload: Box::new(event),
        });
        node
    }

    fn keybinding<UntypedEvent: Any + Sized>(
        self,
        keybing: KeyCode,
        event: UntypedEvent,
    ) -> Node<'a> {
        let mut node: Node = self.into();

        node.interactions.push(Interaction {
            kind: InteractionKind::Keyboard(keybing),
            payload: Box::new(event),
        });
        node
    }

    fn scrollable<UntypedEvent: Any + Sized>(
        self,
        scroll_backward_event: UntypedEvent,
        scroll_forward_event: UntypedEvent,
    ) -> Node<'a> {
        let mut node: Node = self.into();

        node.interactions.push(Interaction {
            kind: InteractionKind::Scroll(ScrollDirection::Backward),
            payload: Box::new(scroll_backward_event),
        });

        node.interactions.push(Interaction {
            kind: InteractionKind::Scroll(ScrollDirection::Forward),
            payload: Box::new(scroll_forward_event),
        });

        node
    }
}

impl<'a> From<Block<'a>> for Node<'a> {
    fn from(value: Block<'a>) -> Self {
        Node::new_block(Block::default_layout(), value)
    }
}

impl<'a> From<Span<'a>> for Node<'a> {
    fn from(value: Span<'a>) -> Self {
        Node::new_span(Span::default_layout(), value)
    }
}

impl<'a> From<Text<'a>> for Node<'a> {
    fn from(value: Text<'a>) -> Self {
        Node::new_text(Text::default_layout(), value)
    }
}

impl<'a> NodeBuilder<'a> for ratatui::widgets::Block<'a> {
    fn default_layout() -> LayoutStyle {
        LayoutStyle {
            border: LayoutRect {
                left: length(1.0),
                right: length(1.0),
                top: length(1.0),
                bottom: length(1.0),
            },
            ..LayoutStyle::default()
        }
    }
}

impl<'a> NodeBuilder<'a> for Text<'a> {}

impl<'a> NodeBuilder<'a> for Span<'a> {}

// impl Iterator<Item = impl Iterator<Item = I::Item>>
// where I: Iterator + Clone,
//       I::Item: Clone,

pub struct Stack;
impl Stack {
    pub fn new() -> Self {
        Stack
    }
}

pub struct HStack;
impl HStack {
    pub fn new() -> Self {
        HStack
    }
}
pub struct VStack;
impl VStack {
    pub fn new() -> Self {
        VStack
    }
}

pub struct GridStack;
impl GridStack {
    pub fn new() -> Self {
        GridStack
    }
}

impl<'a> From<Stack> for Node<'a> {
    fn from(_: Stack) -> Self {
        Node::new_flex(Stack::default_layout())
    }
}

impl<'a> From<HStack> for Node<'a> {
    fn from(_: HStack) -> Self {
        Node::new_row(Stack::default_layout())
    }
}

impl<'a> From<VStack> for Node<'a> {
    fn from(_: VStack) -> Self {
        Node::new_column(Stack::default_layout())
    }
}

impl<'a> From<GridStack> for Node<'a> {
    fn from(_: GridStack) -> Self {
        Node::new_grid(Stack::default_layout())
    }
}

impl<'a> NodeBuilder<'a> for Stack {}

impl<'a> NodeBuilder<'a> for HStack {
    fn default_layout() -> LayoutStyle {
        LayoutStyle {
            flex_direction: FlexDirection::Row,
            ..LayoutStyle::default()
        }
    }
}

impl<'a> NodeBuilder<'a> for GridStack {
    fn default_layout() -> LayoutStyle {
        LayoutStyle {
            display: Display::Grid,
            ..LayoutStyle::default()
        }
    }
}

impl<'a> NodeBuilder<'a> for VStack {
    fn default_layout() -> LayoutStyle {
        LayoutStyle {
            flex_direction: FlexDirection::Column,
            ..LayoutStyle::default()
        }
    }
}

impl<'a> NodeBuilder<'a> for Node<'a> {
    fn layout(self, style: LayoutStyle) -> Node<'a>
    where
        Self: Sized,
    {
        let mut node: Node = self;
        node.style = style;
        node
    }

    fn children<T>(self, style: LayoutStyle, children: T) -> Node<'a>
    where
        T: Into<Vec<Node<'a>>>,
        Self: Sized,
    {
        let mut node: Node = self;
        node.style = style;
        node.children.extend(children.into());
        node
    }

    fn childs<T>(self, children: T) -> Node<'a>
    where
        T: IntoIterator<Item = Node<'a>, IntoIter = std::vec::IntoIter<Node<'a>>>,
        // Self: Sized,
    {
        let mut node: Node = self;
        node.children.extend(children);
        node
    }

    fn childs_iter(self, children: impl Iterator<Item = impl Into<Node<'a>>>) -> Node<'a> {
        let mut node: Node = self;
        node.children.extend(children.map(|c| c.into()));
        node
    }

    fn child<T>(self, child: T) -> Node<'a>
    where
        T: Into<Node<'a>>,
        Self: Sized,
    {
        let mut node: Node = self;
        node.children.push(child.into());
        node
    }

    fn touchable<UntypedEvent: Any + Sized>(self, event: UntypedEvent) -> Node<'a> {
        let mut node: Node = self.into();

        node.interactions.push(Interaction {
            kind: InteractionKind::Click,
            payload: Box::new(event),
        });
        node
    }

    fn keybinding<UntypedEvent: Any + Sized>(
        self,
        keybing: KeyCode,
        event: UntypedEvent,
    ) -> Node<'a> {
        let mut node: Node = self.into();

        node.interactions.push(Interaction {
            kind: InteractionKind::Keyboard(keybing),
            payload: Box::new(event),
        });
        node
    }
}

pub fn root_node(area: Rect, child: Node<'static>) -> Node<'static> {
    use taffy::prelude::*;
    let mut tree = Node::new_flex(LayoutStyle {
        size: Size {
            width: length(area.width as f32),
            height: length(area.height as f32),
        },
        padding: Rect {
            left: length(1.),
            right: length(1.),
            top: length(1.),
            bottom: length(1.),
        },
        ..VStack::default_layout()
    })
    .child(child)
    .debug("root");

    println!("Root tree widget: {:#?}", tree);

    tree.compute_layout(Size {
        width: length(area.width),
        height: length(area.height),
    });
    tree.print_tree();

    tree
}

#[cfg(test)]
mod tests {
    use ratatui::style::Stylize;

    use super::*;

    fn assert_buffer_eq<T>(buf: &Buffer, expected: T)
    where
        T: IntoIterator<Item = &'static str>,
    {
        let buffer_content = buf
            .content()
            .iter()
            .chunks(buf.area.width as usize)
            .into_iter()
            .map(|chunk| {
                chunk
                    .map(|cell| cell.symbol().to_string())
                    .collect::<String>()
            })
            .collect::<Vec<String>>();

        assert_eq!(
            buffer_content,
            expected.into_iter().collect_vec(),
            "Buffer mismatch: actual"
        );

        // for (cell, expected) in expected.chars().zip(buf.content().iter()) {
        //     assert_eq!(cell.to_string(), expected.symbol().to_string(), "Buffer mismatch: expected");
        // }
    }

    #[test]
    fn test_hstack_layout() {
        let mut root = HStack::new().child("hello".bold()).child("world".bold());

        let mut buf = Buffer::empty(Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 1,
        });

        root.compute_layout(LayoutSize {
            width: length(buf.area.width as f32),
            height: length(buf.area.height as f32),
        });

        root.print_tree();

        root.render_ref(buf.area, &mut buf);

        assert_buffer_eq(&buf, ["helloworld"]);
    }

    #[test]
    fn test_vstack_layout() {
        let mut root = VStack::new().child("hello".bold()).child("world".bold());

        let mut buf = Buffer::empty(Rect {
            x: 0,
            y: 0,
            width: 5,
            height: 2,
        });

        root.compute_layout(LayoutSize {
            width: length(buf.area.width as f32),
            height: length(buf.area.height as f32),
        });

        root.print_tree();

        root.render_ref(buf.area, &mut buf);

        assert_buffer_eq(&buf, ["hello", "world"]);
    }

    #[test]
    fn test_gridstack_layout() {
        let mut root = GridStack::new()
            .layout(LayoutStyle {
                grid_template_columns: vec![length(1.), length(2.)],
                grid_template_rows: vec![length(2.), length(2.)],
                ..GridStack::default_layout()
            })
            .childs(vec![
                Span::raw("A").layout(LayoutStyle {
                    grid_column: line(1),
                    grid_row: line(1),
                    ..LayoutStyle::default()
                }),
                Span::raw("BC").layout(LayoutStyle {
                    grid_column: line(2),
                    grid_row: line(1),
                    ..LayoutStyle::default()
                }),
                Span::raw("DEF").layout(LayoutStyle {
                    grid_column: span(2),
                    grid_row: line(2),
                    ..LayoutStyle::default()
                }),
            ]);

        let mut buf = Buffer::empty(Rect {
            x: 0,
            y: 0,
            width: 3,
            height: 4,
        });

        root.compute_layout(LayoutSize {
            width: length(buf.area.width as f32),
            height: length(buf.area.height as f32),
        });

        root.print_tree();

        root.render_ref(buf.area, &mut buf);

        assert_buffer_eq(&buf, ["ABC", "   ", "DEF", "   "]);
    }

    #[test]
    fn test_child_iters() {
        let mut root = HStack::new().childs_iter(
            (0..10)
                .into_iter()
                .map(|i| HStack::new().child(format!("{}", i).bold())),
        );

        let mut buf = Buffer::empty(Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 1,
        });

        root.compute_layout(LayoutSize {
            width: length(buf.area.width as f32),
            height: length(buf.area.height as f32),
        });

        root.print_tree();

        root.render_ref(buf.area, &mut buf);

        assert_buffer_eq(&buf, ["0123456789"]);
    }

    #[test]
    fn test_fractional_layout() {
        let mut root = GridStack::new()
            .layout(LayoutStyle {
                grid_template_columns: vec![fr(1.), fr(1.)],
                grid_template_rows: vec![fr(1.)],

                size: LayoutSize {
                    width: length(3.),
                    height: length(1.),
                },
                ..GridStack::default_layout()
            })
            .childs(vec![
                Span::raw("A").layout(LayoutStyle {
                    grid_column: line(1),
                    grid_row: line(1),
                    ..LayoutStyle::default()
                }),
                Span::raw("B").layout(LayoutStyle {
                    grid_column: line(2),
                    grid_row: line(1),
                    ..LayoutStyle::default()
                }),
            ]);

        let mut buf = Buffer::empty(Rect {
            x: 0,
            y: 0,
            width: 3,
            height: 1,
        });

        root.compute_layout(LayoutSize {
            width: length(buf.area.width as f32),
            height: length(buf.area.height as f32),
        });

        root.print_tree();

        root.render_ref(buf.area, &mut buf);

        assert_buffer_eq(&buf, ["A B"]);
    }

    #[test]
    fn test_border_layout() {
        let mut root = Block::new()
            .borders(Borders::ALL)
            .title("A")
            .child(Span::raw("B"));

        let mut buf = Buffer::empty(Rect {
            x: 0,
            y: 0,
            width: 3,
            height: 3,
        });

        root.compute_layout(LayoutSize {
            width: length(buf.area.width as f32),
            height: length(buf.area.height as f32),
        });

        root.print_tree();

        root.render_ref(buf.area, &mut buf);

        assert_buffer_eq(&buf, ["┌A┐", "│B│", "└─┘"]);
    }

    #[test]
    fn test_root_node() {
        println!("TESTING ROOT NODE");

        let mut buf = Buffer::empty(Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        });

        let root = root_node(
            buf.area,
            VStack::new()
                .child(
                    HStack::new()
                        .child("1".bold().node().debug(format!("child{}-1", 1).as_str()))
                        .child("2".bold().node().debug(format!("child{}-2", 1).as_str()))
                        .debug("hstack1"),
                )
                .child(
                    HStack::new()
                        .child("1".bold().node().debug(format!("child{}-1", 2).as_str()))
                        .child("2".bold().node().debug(format!("child{}-2", 2).as_str()))
                        .debug("hstack2"),
                )
                .child(
                    HStack::new()
                        .child("1".bold().node().debug(format!("child{}-1", 3).as_str()))
                        .child("2".bold().node().debug(format!("child{}-2", 3).as_str()))
                        .debug("hstack3"),
                )
                .debug("vstack"),
        )
        .debug("root");

        println!("PRINTING {:#?}", root);

        root.print_tree();

        root.render_ref(buf.area, &mut buf);
    }
}
//     let mut row_node = Node::new_row(NodeId::from(id_count), Style::default());

//     id_count = id_count + 1;
//     let text_node = Node::new_text(
//         NodeId::from(id_count),
//         Style::default(),
//         TextContext {
//             text_content: LOREM_IPSUM.into(),
//             writing_mode: WritingMode::Horizontal,
//         },
//     );
//     row_node.append_child(text_node);

//     id_count = id_count + 1;
//     let image_node = Node::new_image(
//         NodeId::from(id_count),
//         Style::default(),
//         ImageContext {
//             width: 400.0,
//             height: 300.0,
//         },
//     );
//     row_node.append_child(image_node);

//     id_count = id_count + 1;
//     let image_node_sm = Node::new_image(
//         NodeId::from(id_count),
//         Style::default(),
//         ImageContext {
//             width: 10.0,
//             height: 10.0,
//         },
//     );
//     row_node.append_child(image_node_sm);

//     root.append_child(row_node);

//     id_count = id_count + 1;
//     let image_node_lg = Node::new_image(
//         NodeId::from(id_count),
//         Style::default(),
//         ImageContext {
//             width: 1000.0,
//             height: 1000.0,
//         },
//     );
//     root.append_child(image_node_lg);

//     // Compute layout
//     root.compute_layout(Size::MAX_CONTENT);

//     print_tree_custom(&root, NodeId::from(0 as usize))
// }

// #[derive(Debug)]
// pub enum TreeView<'a> {
//     Container(Node<'a>, Vec<TreeView<'a>>),
//     View(Node<'a>),
// }

// pub trait TreeBuilder<'a>: Into<TreeView<'a>> {
//     fn default_layout() -> LayoutStyle {
//         LayoutStyle::default()
//     }

//     fn into_node(self) -> Node<'a>
//     where
//         Self: Sized;

//     fn view(self) -> TreeView<'a>
//     where
//         Self: Sized,
//     {
//         TreeView::View(self.into_node())
//     }

//     fn layout_style(self, style: LayoutStyle) -> TreeView<'a>
//     where
//         Self: Sized,
//     {
//         TreeView::View(Node {
//             style,
//             ..self.into_node()
//         })
//     }

//     fn children<T>(self, style: LayoutStyle, children: T) -> TreeView<'a>
//     where
//         T: Into<Vec<TreeView<'a>>>,
//         Self: Sized,
//     {
//         TreeView::Container(
//             Node {
//                 style,
//                 ..self.into_node()
//             },
//             children.into(),
//         )
//     }
// }

// impl<'a> From<Block<'a>> for TreeView<'a> {
//     fn from(value: Block<'a>) -> Self {
//         TreeView::new_block(value, Block::default_layout())
//     }
// }

// impl<'a> From<Span<'a>> for TreeView<'a> {
//     fn from(value: Span<'a>) -> Self {
//         TreeView::new_span(value, Span::default_layout())
//     }
// }

// impl<'a> From<Text<'a>> for TreeView<'a> {
//     fn from(value: Text<'a>) -> Self {
//         TreeView::new_text(value, Text::default_layout())
//     }
// }

// impl<'a> TreeBuilder<'a> for ratatui::widgets::Block<'a> {
//     fn default_layout() -> LayoutStyle {
//         LayoutStyle {
//             border: Rect {
//                 left: length(1.0),
//                 right: length(1.0),
//                 top: length(1.0),
//                 bottom: length(1.0),
//             },
//             ..LayoutStyle::default()
//         }
//     }

//     fn into_node(self: ratatui::widgets::Block<'a>) -> Node<'a> {
//         Node::new_block(ratatui::widgets::Block::default_layout(), self)
//     }
// }

// impl<'a> TreeBuilder<'a> for Text<'a> {
//     fn into_node(self: Text<'a>) -> Node<'a> {
//         Node::new_text(Text::default_layout(), self)
//     }
// }

// impl<'a> TreeBuilder<'a> for Span<'a> {
//     fn into_node(self: Span<'a>) -> Node<'a> {
//         Node::new_span(Span::default_layout(), self)
//     }
// }

// pub struct ScrollView;
// impl ScrollView {
//     pub fn new() -> Self {
//         ScrollView
//     }
// }

// impl<'a> From<Stack> for TreeView<'a> {
//     fn from(_: Stack) -> Self {
//         TreeView::View(Node::new_flex(Stack::default_layout()))
//     }
// }

// impl<'a> From<HStack> for TreeView<'a> {
//     fn from(_: HStack) -> Self {
//         TreeView::View(Node::new_flex(Stack::default_layout()))
//     }
// }

// impl<'a> From<VStack> for TreeView<'a> {
//     fn from(_: VStack) -> Self {
//         TreeView::View(Node::new_flex(Stack::default_layout()))
//     }
// }

// impl<'a> From<GridStack> for TreeView<'a> {
//     fn from(_: GridStack) -> Self {
//         TreeView::View(Node::new_grid(Stack::default_layout()))
//     }
// }

// impl<'a> TreeBuilder<'a> for Stack {
//     fn into_node(self) -> Node<'a>
//     where
//         Self: Sized,
//     {
//         Node::new_flex(Stack::default_layout())
//     }
// }

// impl<'a> TreeBuilder<'a> for HStack {
//     fn default_layout() -> LayoutStyle {
//         LayoutStyle {
//             flex_direction: FlexDirection::Row,
//             ..LayoutStyle::default()
//         }
//     }

//     fn into_node(self) -> Node<'a>
//     where
//         Self: Sized,
//     {
//         Node::new_row(HStack::default_layout())
//     }
// }

// impl<'a> TreeBuilder<'a> for GridStack {
//     fn default_layout() -> LayoutStyle {
//         LayoutStyle {
//             display: Display::Grid,
//             ..LayoutStyle::default()
//         }
//     }

//     fn into_node(self) -> Node<'a>
//     where
//         Self: Sized,
//     {
//         Node::new_grid(GridStack::default_layout())
//     }
// }

// impl<'a> TreeBuilder<'a> for VStack {
//     fn default_layout() -> LayoutStyle {
//         LayoutStyle {
//             flex_direction: FlexDirection::Column,
//             ..LayoutStyle::default()
//         }
//     }

//     fn into_node(self) -> Node<'a>
//     where
//         Self: Sized,
//     {
//         Node::new_column(VStack::default_layout())
//     }
// }

// impl<'a> TreeView<'a> {
//     pub fn new_scrollview(layout: LayoutStyle, text: Text<'static>, scroll: usize) -> TreeView {
//         TreeView::View(Node::new_scrollview(layout, text, scroll))
//     }

//     pub fn new_stack(layout: LayoutStyle, children: Vec<TreeView>) -> TreeView {
//         TreeView::Container(Node::new_flex(layout), children)
//     }

//     pub fn new_hstack(children: Vec<TreeView>) -> TreeView {
//         TreeView::Container(Node::new_row(LayoutStyle::default()), children)
//     }

//     pub fn new_vstack(children: Vec<TreeView>) -> TreeView {
//         TreeView::Container(Node::new_column(LayoutStyle::default()), children)
//     }

//     pub fn new_block(block: Block<'a>, style: LayoutStyle) -> TreeView {
//         TreeView::View(Node::new_block(style, block))
//     }

//     pub fn new_span(span: Span<'a>, style: LayoutStyle) -> TreeView {
//         TreeView::View(Node::new_span(style, span))
//     }

//     pub fn new_text(text: Text<'a>, style: LayoutStyle) -> TreeView {
//         TreeView::View(Node::new_text(style, text))
//     }

//     pub fn new_block_stack(
//         block: Block<'a>,
//         style: LayoutStyle,
//         children: Vec<TreeView<'a>>,
//     ) -> TreeView<'a> {
//         TreeView::Container(Node::new_block(style, block), children)
//     }

//     pub fn text<T>(text: T) -> TreeView<'a>
//     where
//         T: Into<Text<'static>>,
//     {
//         TreeView::View(Node::new_text(LayoutStyle::default(), text))
//     }

//     pub fn with_children<T>(self, children: T) -> TreeView<'a>
//     where
//         T: Into<Vec<TreeView<'a>>>,
//         Self: Sized,
//     {
//         match self {
//             TreeView::Container(node, existing_children) => {
//                 panic!("multiple with children")
//             }
//             TreeView::View(node) => TreeView::Container(node, children.into()),
//         }
//     }

//     pub fn layout_style(self, layout: LayoutStyle) -> TreeView<'a> {
//         match self {
//             TreeView::Container(node, existing_children) => TreeView::Container(
//                 Node {
//                     style: layout,
//                     ..node
//                 },
//                 existing_children,
//             ),
//             TreeView::View(node) => TreeView::View(Node {
//                 style: layout,
//                 ..node
//             }),
//         }
//     }

//     pub fn append_layout<F>(self, layout_fn: F) -> TreeView<'a>
//     where
//         F: Fn(LayoutStyle) -> LayoutStyle,
//     {
//         match self {
//             TreeView::Container(node, existing_children) => TreeView::Container(
//                 Node {
//                     style: layout_fn(node.style),
//                     ..node
//                 },
//                 existing_children,
//             ),
//             TreeView::View(node) => TreeView::View(Node {
//                 style: layout_fn(node.style),
//                 ..node
//             }),
//         }
//     }
// }

// pub struct TreeWidget<'a> {
//     nodes: Vec<Node<'a>>,
//     children: Vec<TreeWidget<'a>>,
// }

// impl<'a> TreeWidget<'a> {
//     pub fn new() -> TreeWidget<'a> {
//         TreeWidget {
//             nodes: Vec::new(),
//             children: Vec::new(),
//         }
//     }

//     pub fn add_tree(&mut self, tree_view: TreeView<'a>) -> NodeId {
//         match tree_view {
//             TreeView::Container(node, children) => {
//                 let mut child_ids = vec![];
//                 for child in children {
//                     let child_id = self.add_tree(child);
//                     child_ids.push(child_id);
//                 }

//                 let node_id = self.add_node(node);

//                 for child_id in child_ids {
//                     self.append_child(node_id, usize::from(child_id));
//                 }

//                 NodeId::from(node_id)
//             }
//             TreeView::View(node) => {
//                 let node_id = self.add_node(node);

//                 NodeId::from(node_id)
//             }
//         }
//     }

//     pub fn add_node(&mut self, node: Node<'a>) -> usize {
//         self.nodes.push(node);
//         self.nodes.len() - 1
//     }

//     pub fn append_child(&mut self, parent: usize, child: usize) {
//         self.nodes[parent].children.push(child);
//         self.nodes[child].parent = Some(parent);
//     }

//     #[inline(always)]
//     fn node_from_id(&self, node_id: NodeId) -> &Node {
//         &self.nodes[usize::from(node_id)]
//     }

//     fn update_node_layout(&'a mut self, node_id: NodeId, layout: Layout) {
//         self.node_from_id_mut(node_id).final_layout = layout;
//     }

//     #[inline(always)]
//     fn node_from_id_mut(&mut self, node_id: NodeId) -> &mut Node<'a> {
//         &mut self.nodes[usize::from(node_id)]
//         // &mut self.nodes[usize::from(node_id)]
//     }

//     pub fn compute_layout(
//         &mut self,
//         root: usize,
//         available_space: Size<AvailableSpace>,
//         use_rounding: bool,
//     ) {
//         compute_root_layout(self, NodeId::from(root), available_space);
//         if use_rounding {
//             round_layout(self, NodeId::from(root))
//         }
//     }

//     pub fn print_tree(&mut self, root: usize) {
//         print_tree(self, NodeId::from(root));
//     }

//     pub fn get_absolute_rect(&self, node_id: NodeId) -> ratatui::prelude::Rect {
//         let node = self.node_from_id(node_id);
//         let layout = node.final_layout;
//         let parent_rect = node
//             .parent
//             .and_then(|id| Some(self.get_absolute_rect(NodeId::from(id))))
//             .unwrap_or(ratatui::prelude::Rect::default());

//         return ratatui::prelude::Rect {
//             x: parent_rect.x + layout.location.x as u16,
//             y: parent_rect.y + layout.location.y as u16,
//             width: layout.size.width as u16,
//             height: layout.size.height as u16,
//         };
//     }

//     fn render_node(
//         &self,
//         node_id: NodeId,
//         area: ratatui::prelude::Rect,
//         offset: Offset,
//         buf: &mut ratatui::buffer::Buffer,
//     ) {
//         let node = self.node_from_id(node_id);
//         let node_area = self
//             .get_absolute_rect(NodeId::from(node_id))
//             .intersection(area)
//             .intersection(buf.area);

//         node.render_ref(node_area, buf);

//         let child_area = area.intersection(ratatui::prelude::Rect {
//             x: node_area.x + node.final_layout.border.left as u16,
//             y: node_area.y + node.final_layout.border.top as u16,
//             width: node_area.width
//                 - (node.final_layout.border.left + node.final_layout.border.right) as u16,
//             height: node_area.height
//                 - (node.final_layout.border.top + node.final_layout.border.bottom) as u16,
//         });

//         let childs = self.child_ids(node_id).collect_vec().clone();
//         for child in childs {
//             self.render_node(child, area, offset, buf)
//         }
//     }
// }

// impl<'a> WidgetRef for TreeWidget<'a> {
//     fn render_ref(&self, area: ratatui::prelude::Rect, buf: &mut ratatui::buffer::Buffer) {
//         let nodes_to_render: Vec<NodeId> = self
//             .nodes
//             .iter()
//             .enumerate()
//             .filter(|(_, n)| n.parent.is_none())
//             .map(|(index, _)| NodeId::from(index))
//             .collect();

//         for node_id in nodes_to_render {
//             self.render_node(node_id, area, Offset::default(), buf);
//         }
//     }
// }

// pub struct ChildIter2<'a>(std::slice::Iter<'a, usize>);

// impl<'a> Iterator for ChildIter2<'a> {
//     type Item = NodeId;
//     fn next(&mut self) -> Option<Self::Item> {
//         self.0.next().copied().map(NodeId::from)
//     }
// }

// trait Captures<'a> {}
// impl<'a, T: ?Sized> Captures<'a> for T {}

// // fn bar_to_foo<'a, 'b: 'a>(
// //     bar: &'a mut TreeWidget<'b>,
// // ) -> impl TraversePartialTree + Captures<'b> + 'a {
// //     bar
// // }

// impl<'a> taffy::TraversePartialTree for TreeWidget<'a>
// where
//     Self: 'a,
// {
//     type ChildIter<'b> = ChildIter   where
//     Self: 'b;

//     fn child_ids(&self, _node_id: NodeId) -> Self::ChildIter<'_> {
//         ChildIter(0..self.children.len())

//         //ChildIter2(self.node_from_id(node_id).children.iter())
//     }

//     fn child_count(&self, _node_id: NodeId) -> usize {
//         self.children.len()
//     }

//     fn get_child_id(&self, _node_id: NodeId, index: usize) -> NodeId {
//         NodeId::from(index)
//     }
// }

// impl<'a> taffy::TraverseTree for TreeWidget<'a> {}

// impl<'a> taffy::LayoutPartialTree for TreeWidget<'a> {
//     fn get_style(&self, node_id: NodeId) -> &Style {
//         &self.node_from_id(node_id).style
//     }

//     fn set_unrounded_layout(&mut self, node_id: NodeId, layout: &Layout) {
//         self.node_from_id_mut(node_id).unrounded_layout = *layout;
//     }

//     fn get_cache_mut(&mut self, node_id: NodeId) -> &mut Cache {
//         &mut self.node_from_id_mut(node_id).cache
//     }

//     fn compute_child_layout(
//         &mut self,
//         node_id: NodeId,
//         inputs: taffy::tree::LayoutInput,
//     ) -> taffy::tree::LayoutOutput {
//         compute_cached_layout(self, node_id, inputs, |tree, node_id, inputs| {
//             let node = tree.node_from_id(node_id);
//             let font_metrics = FontMetrics {
//                 char_width: 1.0,
//                 char_height: 1.0,
//             };

//             match &node.kind {
//                 NodeKind::Flexbox => compute_flexbox_layout(tree, node_id, inputs),
//                 NodeKind::Grid => compute_grid_layout(tree, node_id, inputs),
//                 NodeKind::Text(paragraph) | NodeKind::ScrollView(paragraph, _) => {
//                     compute_leaf_layout(inputs, &node.style, |known_dimensions, available_space| {
//                         let text_content = paragraph
//                             .lines
//                             .iter()
//                             .map(|l| {
//                                 l.spans
//                                     .iter()
//                                     .map(|s| s.content.clone().into_owned())
//                                     .collect_vec()
//                                     .join("")
//                             })
//                             .collect_vec()
//                             .join("\n");

//                         text_measure_function(
//                             known_dimensions,
//                             available_space,
//                             &TextContext {
//                                 text_content: text_content,
//                                 writing_mode: WritingMode::Horizontal,
//                             },
//                             &font_metrics,
//                         )
//                     })
//                 }

//                 NodeKind::Span(span) => compute_leaf_layout(
//                     inputs,
//                     &node.style,
//                     |_known_dimensions, _available_space| Size {
//                         width: length(span.width() as f32),
//                         height: length(1.),
//                     },
//                 ),
//                 NodeKind::Block(_) => compute_flexbox_layout(tree, node_id, inputs),
//             }
//         })
//     }
// }

// impl<'a> taffy::RoundTree for TreeWidget<'a> {
//     fn get_unrounded_layout(&self, node_id: NodeId) -> &Layout {
//         &self.node_from_id(node_id).unrounded_layout
//     }

//     fn set_final_layout(&mut self, node_id: NodeId, layout: &Layout) {
//         self.node_from_id_mut(node_id).final_layout = *layout;
//     }
// }

// impl<'a> taffy::PrintTree for TreeWidget<'a> {
//     fn get_debug_label(&self, node_id: NodeId) -> &'static str {
//         match self.node_from_id(node_id).kind {
//             NodeKind::Flexbox => "FLEX",
//             NodeKind::Grid => "GRID",
//             NodeKind::Text(_) => "TEXT",
//             NodeKind::Block(_) => "BLOCK",
//             NodeKind::Span(_) => "SPAN",
//             NodeKind::ScrollView(_, _) => "SCROLL",
//         }
//     }

//     fn get_final_layout(&self, node_id: NodeId) -> &Layout {
//         &self.node_from_id(node_id).final_layout
//     }
// }
