use std::vec::IntoIter;

use taffy::{
    compute_cached_layout, compute_flexbox_layout, compute_grid_layout, compute_leaf_layout,
    compute_root_layout, prelude::*, Cache, Layout, Style,
};

use taffy::geometry::Size;

pub struct ImageContext {
    pub width: f32,
    pub height: f32,
}

pub const LOREM_IPSUM : &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.";

pub struct FontMetrics {
    pub char_width: f32,
    pub char_height: f32,
}

#[allow(dead_code)]
pub enum WritingMode {
    Horizontal,
    Vertical,
}

pub struct TextContext {
    pub text_content: String,
    pub writing_mode: WritingMode,
}

pub fn text_measure_function(
    known_dimensions: taffy::geometry::Size<Option<f32>>,
    available_space: taffy::geometry::Size<taffy::style::AvailableSpace>,
    text_context: &TextContext,
    font_metrics: &FontMetrics,
) -> taffy::geometry::Size<f32> {
    use taffy::geometry::AbsoluteAxis;
    use taffy::prelude::*;

    let inline_axis = match text_context.writing_mode {
        WritingMode::Horizontal => AbsoluteAxis::Horizontal,
        WritingMode::Vertical => AbsoluteAxis::Vertical,
    };
    let block_axis = inline_axis.other_axis();
    let words: Vec<&str> = text_context.text_content.split_whitespace().collect();

    if words.is_empty() {
        return Size::ZERO;
    }

    let min_line_length: usize = words.iter().map(|line| line.len()).max().unwrap_or(0);
    let max_line_length: usize = words.iter().map(|line| line.len()).sum();
    let inline_size =
        known_dimensions.get_abs(inline_axis).unwrap_or_else(|| {
            match available_space.get_abs(inline_axis) {
                AvailableSpace::MinContent => min_line_length as f32 * font_metrics.char_width,
                AvailableSpace::MaxContent => max_line_length as f32 * font_metrics.char_width,
                AvailableSpace::Definite(inline_size) => inline_size
                    .min(max_line_length as f32 * font_metrics.char_width)
                    .max(min_line_length as f32 * font_metrics.char_width),
            }
        });
    let block_size = known_dimensions.get_abs(block_axis).unwrap_or_else(|| {
        let inline_line_length = (inline_size / font_metrics.char_width).floor() as usize;
        let mut line_count = 1;
        let mut current_line_length = 0;
        for word in &words {
            if current_line_length == 0 {
                // first word
                current_line_length = word.len();
            } else if current_line_length + word.len() + 1 > inline_line_length {
                // every word past the first needs to check for line length including the space between words
                // note: a real implementation of this should handle whitespace characters other than ' '
                // and do something more sophisticated for long words
                line_count += 1;
                current_line_length = word.len();
            } else {
                // add the word and a space
                current_line_length += word.len() + 1;
            };
        }
        (line_count as f32) * font_metrics.char_height
    });

    match text_context.writing_mode {
        WritingMode::Horizontal => Size {
            width: inline_size,
            height: block_size,
        },
        WritingMode::Vertical => Size {
            width: block_size,
            height: inline_size,
        },
    }
}

pub fn image_measure_function(
    known_dimensions: taffy::geometry::Size<Option<f32>>,
    image_context: &ImageContext,
) -> taffy::geometry::Size<f32> {
    match (known_dimensions.width, known_dimensions.height) {
        (Some(width), Some(height)) => Size { width, height },
        (Some(width), None) => Size {
            width,
            height: (width / image_context.width) * image_context.height,
        },
        (None, Some(height)) => Size {
            width: (height / image_context.height) * image_context.width,
            height,
        },
        (None, None) => Size {
            width: image_context.width,
            height: image_context.height,
        },
    }
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
enum NodeKind {
    Flexbox,
    Grid,
    Text,
    Image,
}

struct Node {
    id: NodeId,
    kind: NodeKind,
    style: Style,
    text_data: Option<TextContext>,
    image_data: Option<ImageContext>,
    cache: Cache,
    layout: Layout,
    children: Vec<Node>,
}

impl Default for Node {
    fn default() -> Self {
        Node {
            id: NodeId::from(usize::MAX),
            kind: NodeKind::Flexbox,
            style: Style::default(),
            text_data: None,
            image_data: None,
            cache: Cache::new(),
            layout: Layout::with_order(0),
            children: Vec::new(),
        }
    }
}

#[allow(dead_code)]
impl Node {
    pub fn new_row(id: NodeId, style: Style) -> Node {
        Node {
            id,
            kind: NodeKind::Flexbox,
            style: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                ..style
            },
            ..Node::default()
        }
    }
    pub fn new_column(id: NodeId, style: Style) -> Node {
        Node {
            id,
            kind: NodeKind::Flexbox,
            style: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..style
            },
            ..Node::default()
        }
    }
    pub fn new_grid(id: NodeId, style: Style) -> Node {
        Node {
            id,
            kind: NodeKind::Grid,
            style: Style {
                display: Display::Grid,
                ..style
            },
            ..Node::default()
        }
    }
    pub fn new_text(id: NodeId, style: Style, text_data: TextContext) -> Node {
        Node {
            id,
            kind: NodeKind::Text,
            style,
            text_data: Some(text_data),
            ..Node::default()
        }
    }
    pub fn new_image(id: NodeId, style: Style, image_data: ImageContext) -> Node {
        Node {
            id,
            kind: NodeKind::Image,
            style,
            image_data: Some(image_data),
            ..Node::default()
        }
    }
    pub fn append_child(&mut self, node: Node) {
        self.children.push(node);
    }

    pub fn compute_layout(&mut self, available_space: Size<AvailableSpace>) {
        compute_root_layout(self, NodeId::from(self.id), available_space);
    }

    pub fn index_for_id(&self, id: NodeId) -> Option<usize> {
        self.children.iter().position(|child| child.id == id)
    }

    /// The methods on LayoutPartialTree need to be able to access:
    ///
    ///  - The node being laid out
    ///  - Direct children of the node being laid out
    ///
    /// Each must have an ID. For children we simply use it's index. For the node itself
    /// we use usize::MAX on the assumption that there will never be that many children.
    fn node_from_id(&self, node_id: NodeId) -> &Node {
        if node_id == self.id {
            self
        } else {
            let idx = self.index_for_id(node_id).unwrap();
            &self.children[idx]
        }
    }

    fn node_from_id_mut(&mut self, node_id: NodeId) -> &mut Node {
        if node_id == self.id {
            self
        } else {
            let idx = self.index_for_id(node_id).unwrap();
            &mut self.children[idx]
        }
    }
}

struct ChildIter(IntoIter<taffy::NodeId>);
impl Iterator for ChildIter {
    type Item = NodeId;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl taffy::TraversePartialTree for Node {
    type ChildIter<'a> = ChildIter;

    fn child_ids(&self, _node_id: NodeId) -> Self::ChildIter<'_> {
        ChildIter(
            self.children
                .iter()
                .map(|child| child.id)
                .collect::<Vec<_>>()
                .into_iter(),
        )
        // ChildIter(0..self.children.len())
        // ChildIter(0..self.children.len())
    }

    fn child_count(&self, _node_id: NodeId) -> usize {
        // self.children.len()
        self.children.len()
    }

    fn get_child_id(&self, _node_id: NodeId, index: usize) -> NodeId {
        self.children[index].id
    }
}

impl taffy::LayoutPartialTree for Node {
    fn get_style(&self, node_id: NodeId) -> &Style {
        &self.node_from_id(node_id).style
    }

    fn set_unrounded_layout(&mut self, node_id: NodeId, layout: &Layout) {
        self.node_from_id_mut(node_id).layout = *layout
    }

    fn get_cache_mut(&mut self, node_id: NodeId) -> &mut Cache {
        &mut self.node_from_id_mut(node_id).cache
    }

    fn compute_child_layout(
        &mut self,
        node_id: NodeId,
        inputs: taffy::tree::LayoutInput,
    ) -> taffy::tree::LayoutOutput {
        compute_cached_layout(self, node_id, inputs, |parent, node_id, inputs| {
            let node = parent.node_from_id_mut(node_id);
            let font_metrics = FontMetrics {
                char_width: 10.0,
                char_height: 10.0,
            };

            match node.kind {
                NodeKind::Flexbox => compute_flexbox_layout(node, node_id, inputs),
                NodeKind::Grid => compute_grid_layout(node, node_id, inputs),
                NodeKind::Text => {
                    compute_leaf_layout(inputs, &node.style, |known_dimensions, available_space| {
                        text_measure_function(
                            known_dimensions,
                            available_space,
                            node.text_data.as_ref().unwrap(),
                            &font_metrics,
                        )
                    })
                }
                NodeKind::Image => compute_leaf_layout(
                    inputs,
                    &node.style,
                    |known_dimensions, _available_space| {
                        image_measure_function(known_dimensions, node.image_data.as_ref().unwrap())
                    },
                ),
            }
        })
    }
}
impl taffy::TraverseTree for Node {}

impl taffy::PrintTree for Node {
    fn get_debug_label(&self, node_id: NodeId) -> &'static str {
        match self.node_from_id(node_id).kind {
            NodeKind::Flexbox => match self.node_from_id(node_id) {
                Node {
                    style:
                        Style {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Row,
                            ..
                        },
                    ..
                } => "Row",
                Node {
                    style:
                        Style {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            ..
                        },
                    ..
                } => "Column",
                _ => "Flexbox",
            },
            NodeKind::Grid => "Grid",
            NodeKind::Text => "Text",
            NodeKind::Image => "Image",
        }
    }

    fn get_final_layout(&self, node_id: NodeId) -> &Layout {
        &self.node_from_id(node_id).layout
    }
}

#[cfg(test)]
mod tests {
    use taffy::print_tree;

    use super::*;

    fn test_example_unique() {
        let mut id_count: usize = 0;

        let mut root = Node::new_column(NodeId::from(id_count), Style::DEFAULT);

        id_count = id_count + 1;
        let mut row_node = Node::new_row(NodeId::from(id_count), Style::default());

        id_count = id_count + 1;
        let text_node = Node::new_text(
            NodeId::from(id_count),
            Style::default(),
            TextContext {
                text_content: LOREM_IPSUM.into(),
                writing_mode: WritingMode::Horizontal,
            },
        );
        row_node.append_child(text_node);

        id_count = id_count + 1;
        let image_node = Node::new_image(
            NodeId::from(id_count),
            Style::default(),
            ImageContext {
                width: 400.0,
                height: 300.0,
            },
        );
        row_node.append_child(image_node);

        id_count = id_count + 1;
        let image_node_sm = Node::new_image(
            NodeId::from(id_count),
            Style::default(),
            ImageContext {
                width: 10.0,
                height: 10.0,
            },
        );
        row_node.append_child(image_node_sm);

        root.append_child(row_node);

        id_count = id_count + 1;
        let image_node_lg = Node::new_image(
            NodeId::from(id_count),
            Style::default(),
            ImageContext {
                width: 1000.0,
                height: 1000.0,
            },
        );
        root.append_child(image_node_lg);

        // Compute layout
        root.compute_layout(Size::MAX_CONTENT);

        print_tree_custom(&root, NodeId::from(0 as usize))
    }

    fn test_example_same_id() {
        let mut root = Node::new_column(NodeId::from(0 as usize), Style::DEFAULT);

        let mut row_node = Node::new_row(NodeId::from(1 as usize), Style::default());

        let text_node = Node::new_text(
            NodeId::from(2 as usize),
            Style::default(),
            TextContext {
                text_content: LOREM_IPSUM.into(),
                writing_mode: WritingMode::Horizontal,
            },
        );
        row_node.append_child(text_node);

        let image_node = Node::new_image(
            NodeId::from(3 as usize),
            Style::default(),
            ImageContext {
                width: 400.0,
                height: 300.0,
            },
        );
        row_node.append_child(image_node);

        let image_node_sm = Node::new_image(
            NodeId::from(4 as usize),
            Style::default(),
            ImageContext {
                width: 10.0,
                height: 10.0,
            },
        );
        row_node.append_child(image_node_sm);

        root.append_child(row_node);

        let image_node_lg = Node::new_image(
            NodeId::from(2 as usize),
            Style::default(),
            ImageContext {
                width: 1000.0,
                height: 1000.0,
            },
        );
        root.append_child(image_node_lg);

        // Compute layout
        root.compute_layout(Size::MAX_CONTENT);

        print_tree_custom(&root, NodeId::from(0 as usize))
    }

    #[test]
    fn test_both() {
        println!("UNIQUE TEST");
        test_example_unique();

        println!("SAME ID TEST");
        test_example_same_id();
    }
}

pub fn print_tree_custom(tree: &Node, root: NodeId) {
    println!("TREE");
    print_node_custom(tree, root, false, String::new(), 0);

    /// Recursive function that prints each node in the tree
    fn print_node_custom(
        tree: &Node,
        node_id: NodeId,
        has_sibling: bool,
        lines_string: String,
        level: usize,
    ) {
        // println!(
        //     "Printing ID: {:?} #children = {:?} ",
        //     node_id,
        //     tree.child_count(node_id)
        // );

        let layout = &tree.get_final_layout(node_id);
        let display = tree.get_debug_label(node_id);
        let num_children = tree.child_count(node_id);

        let fork_string = if has_sibling {
            "├── "
        } else {
            "└── "
        };
        #[cfg(feature = "content_size")]
        println!(
                "{lines}{fork} {display} [x: {x:<4} y: {y:<4} w: {width:<4} h: {height:<4} content_w: {content_width:<4} content_h: {content_height:<4} border: l:{bl} r:{br} t:{bt} b:{bb}, padding: l:{pl} r:{pr} t:{pt} b:{pb}] ({key:?})",
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
            );
        #[cfg(not(feature = "content_size"))]
        println!(
            "{lines}{fork} {display} [x: {x:<4} y: {y:<4} width: {width:<4} height: {height:<4}] ({key:?})",
            lines = lines_string,
            fork = fork_string,
            display = display,
            x = layout.location.x,
            y = layout.location.y,
            width = layout.size.width,
            height = layout.size.height,
            key = node_id,
        );
        let bar = if has_sibling { "│   " } else { "    " };
        let new_string = lines_string + bar;

        if (level > 4) {
            return;
        }
        // Recurse into children
        for (index, child) in tree.child_ids(node_id).enumerate() {
            // println!("Looking at #{} ID: {:?}", index, child);
            let has_sibling = index < num_children - 1;
            print_node_custom(
                tree.node_from_id(child),
                child,
                has_sibling,
                new_string.clone(),
                level + 1,
            );
        }
    }
}
