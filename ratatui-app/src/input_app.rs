/// This example is taken from https://raw.githubusercontent.com/fdehau/tui-rs/master/examples/user_input.rs
use ratatui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::{error::Error, iter, ops::ControlFlow};
type BoxedResult<T> = std::result::Result<T, Box<dyn Error>>;
use crate::key_code::KeyCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Done,
    EditingDisplayName,
    EditingSessionId,
    EditingServerAddress,
    CreateGame,
}

/// App holds the state of the application
pub struct AppInput {
    /// Current input mode
    pub input_mode: InputMode,
    /// History of recorded messages
    ///
    pub display_name: String,
    pub session_id: Option<String>,
    pub session_join_url: Option<String>,
    pub server_address: String,
}

impl Default for AppInput {
    fn default() -> AppInput {
        AppInput {
            display_name: String::new(),
            input_mode: InputMode::EditingDisplayName,
            session_id: None,
            session_join_url: None,
            server_address: String::new(),
        }
    }
}

#[derive(Debug)]
struct InputRowLayout {
    label: Rect,
    input: Rect,
}

#[derive(Debug)]
struct InputLayout {
    inputs: [[Rect; 2]; 3],
    footer_head_row: Rect,
    footer_row: Rect,
}

fn layout(frame: Rect) -> InputLayout {
    use taffy::prelude::*;

    // First create an instance of TaffyTree
    let mut tree: TaffyTree<()> = TaffyTree::new();

    // Create a tree of nodes using `TaffyTree.new_leaf` and `TaffyTree.new_with_children`.
    // These functions both return a node id which can be used to refer to that node
    // The Style struct is used to specify styling information

    fn row(tree: &mut TaffyTree<()>) -> (NodeId, [NodeId; 2]) {
        let label_leaf = tree
            .new_leaf(Style {
                size: Size {
                    width: percent(1.),
                    height: length(1.),
                },

                ..Default::default()
            })
            .unwrap();

        let input_leaf = tree
            .new_leaf(Style {
                size: Size {
                    width: percent(1.),
                    height: length(3.),
                },
                ..Default::default()
            })
            .unwrap();

        let container = tree
            .new_with_children(
                Style {
                    flex_direction: FlexDirection::Column,
                    align_items: Some(AlignItems::Stretch),
                    size: Size {
                        width: percent(1.),
                        height: auto(),
                    },
                    ..Default::default()
                },
                &[label_leaf, input_leaf],
            )
            .unwrap();

        (container, [label_leaf, input_leaf])
    }

    let input_rows = &[row(&mut tree), row(&mut tree), row(&mut tree)];

    let footer_leaf = tree
        .new_leaf(Style {
            size: Size {
                width: percent(1.),
                height: length(1.),
            },

            ..Default::default()
        })
        .unwrap();

    let footer_head = tree
        .new_leaf(Style {
            size: Size {
                width: percent(0.3),
                height: length(3.),
            },

            ..Default::default()
        })
        .unwrap();

    let footer_head_2 = tree
        .new_leaf(Style {
            size: Size {
                width: percent(0.3),
                height: length(3.),
            },

            ..Default::default()
        })
        .unwrap();

    let footer = tree
        .new_with_children(
            Style {
                flex_direction: FlexDirection::Column,
                justify_content: Some(JustifyContent::SpaceBetween),
                align_items: Some(AlignItems::Center),
                size: Size {
                    width: percent(1.),
                    height: auto(),
                },
                flex_grow: 1.,
                ..Default::default()
            },
            &[footer_head, footer_leaf],
        )
        .unwrap();

    let containers = input_rows
        .iter()
        .map(|r| r.0)
        .chain(iter::once(footer))
        .collect::<Vec<NodeId>>();

    let input_rows_container = tree
        .new_with_children(
            Style {
                flex_direction: FlexDirection::Column,
                align_items: Some(AlignItems::Stretch),
                size: Size {
                    width: percent(1.),
                    height: percent(1.),
                },
                gap: length(2.),
                padding: Rect {
                    top: length(1.),
                    left: percent(0.2),
                    right: percent(0.2),
                    bottom: length(1.),
                },
                ..Default::default()
            },
            containers.as_slice(),
        )
        .unwrap();

    // Call compute_layout on the root of your tree to run the layout algorithm
    tree.compute_layout(
        input_rows_container,
        Size {
            width: length(frame.width as f32),
            height: length(frame.height as f32),
        },
    )
    .unwrap();

    tree.print_tree(input_rows_container);

    InputLayout {
        inputs: input_rows.map(|(container, [label, input])| {
            let container = tree.layout(container).unwrap();
            let label = tree.layout(label).unwrap();
            let input = tree.layout(input).unwrap();
            [
                ratatui::layout::Rect {
                    x: label.location.x as u16 + container.location.x as u16,
                    y: label.location.y as u16 + container.location.y as u16,
                    width: label.size.width as u16,
                    height: label.size.height as u16,
                },
                ratatui::layout::Rect {
                    x: input.location.x as u16 + container.location.x as u16,
                    y: input.location.y as u16 + container.location.y as u16,
                    width: input.size.width as u16,
                    height: input.size.height as u16,
                },
            ]
        }),
        footer_row: tree
            .layout(footer_leaf)
            .map(|b| {
                let parent_footer = tree.layout(footer).unwrap();
                ratatui::layout::Rect {
                    x: b.location.x as u16 + parent_footer.location.x as u16,
                    y: b.location.y as u16 + parent_footer.location.y as u16,
                    width: b.size.width as u16,
                    height: b.size.height as u16,
                }
            })
            .unwrap(),

        footer_head_row: tree
            .layout(footer_head)
            .map(|b| {
                let parent_footer = tree.layout(footer).unwrap();
                ratatui::layout::Rect {
                    x: b.location.x as u16 + parent_footer.location.x as u16,
                    y: b.location.y as u16 + parent_footer.location.y as u16,
                    width: b.size.width as u16,
                    height: b.size.height as u16,
                }
            })
            .unwrap(),
    }

    // GameLayout {
    //     players: player_nodes
    //         .iter()
    //         .map(|p| {
    //             let layout = tree.layout(*p).unwrap();
    //             ratatui::layout::Rect {
    //                 x: layout.location.x as u16,
    //                 y: layout.location.y as u16,
    //                 width: layout.size.width as u16,
    //                 height: layout.size.height as u16,
    //             }
    //         })
    //         .collect(),
    //     board: tree
    //         .layout(board_node)
    //         .map(|b| ratatui::layout::Rect {
    //             x: b.location.x as u16,
    //             y: b.location.y as u16,
    //             width: b.size.width as u16,
    //             height: b.size.height as u16,
    //         })
    //         .unwrap(),
    //     game_log: tree
    //         .layout(game_log)
    //         .map(|b| ratatui::layout::Rect {
    //             x: b.location.x as u16,
    //             y: b.location.y as u16,
    //             width: b.size.width as u16,
    //             height: b.size.height as u16,
    //         })
    //         .unwrap(),
    // }
}

impl AppInput {
    pub fn new(
        url: String,
        session_id: Option<String>,
        session_join_url: Option<String>,
        display_name: String,
    ) -> AppInput {
        AppInput {
            display_name,
            input_mode: InputMode::EditingDisplayName,
            session_id,
            session_join_url,
            server_address: url,
        }
    }

    pub fn draw<B: Backend>(&mut self, terminal: &mut Terminal<B>) {
        terminal.draw(|f| self.ui(f));
    }

    pub fn handle_event(&mut self, key_code: KeyCode) -> BoxedResult<ControlFlow<Option<String>>> {
        use KeyCode::*;

        match self.input_mode {
            InputMode::Done => match key_code {
                _ => {}
            },
            _ => match key_code {
                KeyCode::Enter => {
                    self.input_mode = InputMode::Done;
                    return Ok(ControlFlow::Break(Some(self.display_name.clone())));
                }
                KeyCode::Esc => {
                    self.input_mode = InputMode::Done;
                }
                KeyCode::Backspace => match self.input_mode {
                    InputMode::EditingDisplayName => {
                        self.display_name.pop();
                    }
                    InputMode::EditingSessionId => {
                        // self.session_id.pop();
                    }
                    InputMode::EditingServerAddress => {
                        // self.server_address.pop();
                    }
                    _ => {}
                },
                KeyCode::Char(c) => match self.input_mode {
                    InputMode::EditingDisplayName => self.display_name.push(c),
                    // InputMode::EditingSessionId => self.session_id.push(c),
                    // InputMode::EditingServerAddress => self.server_address.push(c),
                    _ => {}
                },
                // KeyCode::Up => match self.input_mode {
                //     InputMode::EditingDisplayName => {
                //         self.input_mode = InputMode::EditingServerAddress
                //     }
                //     InputMode::EditingSessionId => self.input_mode = InputMode::EditingDisplayName,
                //     InputMode::EditingServerAddress => {
                //         self.input_mode = InputMode::EditingSessionId
                //     }
                //     _ => {}
                // },
                // KeyCode::Down => match self.input_mode {
                //     InputMode::EditingDisplayName => self.input_mode = InputMode::EditingSessionId,
                //     InputMode::EditingSessionId => {
                //         self.input_mode = InputMode::EditingServerAddress
                //     }
                //     InputMode::EditingServerAddress => {
                //         self.input_mode = InputMode::EditingDisplayName
                //     }
                //     _ => {}
                // },
                _ => {}
            },
        }
        Ok(ControlFlow::Continue(()))
    }

    fn ui(&mut self, f: &mut Frame) {
        // let chunks = Layout::default()
        //     .direction(Direction::Vertical)
        //     .margin(2)
        //     .constraints(
        //         [
        //             Constraint::Length(1),
        //             Constraint::Length(3),
        //             Constraint::Min(1),
        //         ]
        //         .as_ref(),
        //     )
        //     .split(f.size());

        fn input_row(
            help: String,
            label: String,
            input: String,
            editing: bool,
        ) -> [Paragraph<'static>; 2] {
            let mut text = Text::from(Line::from(vec![Span::raw(help)]));
            text = text.patch_style(Style::default());

            let help_text = Paragraph::new(text);

            // let width = chunks[0].width.max(3) - 3; // keep 2 for borders and 1 for cursor

            let input_text = Paragraph::new(input.clone())
                .style(match editing {
                    true => Style::default().fg(Color::Yellow),
                    _ => Style::default(),
                })
                .block(Block::default().borders(Borders::ALL));

            [help_text, input_text]
        }

        let layout = layout(f.size());

        let display_name_text = input_row(
            "Enter your display name: ".to_string(),
            "Display Name".to_string(),
            self.display_name.clone(),
            self.input_mode == InputMode::EditingDisplayName,
        )
        .into_iter()
        .zip(layout.inputs[0]);

        let session_id_text = input_row(
            "Join URL: ".to_string(),
            "Game ID".to_string(),
            self.session_id.clone().unwrap_or("".to_string()),
            self.input_mode == InputMode::EditingSessionId,
        )
        .into_iter()
        .zip(layout.inputs[1]);

        // let server_address_text = input_row(
        //     "Enter the server URL: ".to_string(),
        //     "URL".to_string(),
        //     self.server_address.clone(),
        //     self.input_mode == InputMode::EditingServerAddress,
        // )
        // .into_iter()
        // .zip(layout.inputs[2]);

        match self.session_id {
            Some(_) => {
                display_name_text
                    .chain(session_id_text)
                    .for_each(|(text, rect)| {
                        f.render_widget(text, rect);
                    });
            }
            None => {
                display_name_text.for_each(|(text, rect)| {
                    f.render_widget(text, rect);
                });
            }
        }

        let join_game_button = Paragraph::new(match self.session_id {
            Some(_) => "Join Game",
            None => "Create Game",
        })
        .style(match false {
            true => Style::default().fg(Color::Yellow),
            _ => Style::default(),
        })
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(join_game_button, layout.footer_head_row);

        let footer_spans = vec![
            Span::raw("Press "),
            Span::styled("[Enter]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(match self.session_id {
                Some(_) => " to join the game",
                None => " to create a game",
            }),
        ];
        let mut footer_text = Text::from(Line::from(footer_spans));
        footer_text = footer_text.patch_style(Style::default());
        let footer_paragraph =
            Paragraph::new(footer_text).alignment(ratatui::layout::Alignment::Center);
        f.render_widget(footer_paragraph, layout.footer_row);

        // let row_chunks = Layout::default().direction(Direction::Vertical)
        // let (msg, style) = match self.input_mode {
        //     InputMode::Done => (
        //         vec![
        //             Span::raw("Press "),
        //             Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
        //             Span::raw(" to exit, "),
        //             Span::styled("e", Style::default().add_modifier(Modifier::BOLD)),
        //             Span::raw(" to start editing."),
        //         ],
        //         Style::default(),
        //     ),
        //     InputMode::Editing => (
        //         vec![
        //             Span::raw("Enter your display name and press "),
        //             Span::styled("[Enter]", Style::default().add_modifier(Modifier::BOLD)),
        //             Span::raw(" to connect"),
        //         ],
        //         Style::default(),
        //     ),
        // };

        // let mut text = Text::from(Line::from(vec![Span::raw("Enter your display name: ")]));
        // text = text.patch_style(Style::default());
        // let help_message = Paragraph::new(text);
        // f.render_widget(help_message, chunks[0]);

        // let width = chunks[0].width.max(3) - 3; // keep 2 for borders and 1 for cursor

        // let input = Paragraph::new(self.display_name.clone())
        //     .style(match self.input_mode {
        //         InputMode::EditingDisplayName => Style::default().fg(Color::Yellow),
        //         _ => Style::default(),
        //     })
        //     .block(Block::default().borders(Borders::ALL).title("Display Name"));
        // f.render_widget(input, chunks[1]);

        // let mut text = Text::from(Line::from(vec![Span::raw("Enter the game ID:")]));
        // text = text.patch_style(Style::default());
        // let help_message = Paragraph::new(text);
        // f.render_widget(help_message, chunks[2]);

        // let width = chunks[3].width.max(3) - 3; // keep 2 for borders and 1 for cursor

        // let input = Paragraph::new(self.session_id.clone())
        //     .style(match self.input_mode {
        //         InputMode::EditingSessionId => Style::default().fg(Color::Yellow),
        //         _ => Style::default(),
        //     })
        //     .block(Block::default().borders(Borders::ALL).title("Game ID"));
        // f.render_widget(input, chunks[3]);

        // let mut text = Text::from(Line::from(vec![Span::raw("Enter the server URL: ")]));
        // text = text.patch_style(Style::default());
        // let help_message = Paragraph::new(text);
        // f.render_widget(help_message, chunks[4]);

        // let width = chunks[4].width.max(3) - 3; // keep 2 for borders and 1 for cursor

        // let input = Paragraph::new(self.server_address.clone())
        //     .style(match self.input_mode {
        //         InputMode::EditingServerAddress => Style::default().fg(Color::Yellow),
        //         _ => Style::default(),
        //     })
        //     .block(Block::default().borders(Borders::ALL).title("URL"));
        // f.render_widget(input, chunks[5]);

        // match self.input_mode {
        //     InputMode::Normal =>
        //         // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
        //         {}

        //     InputMode::Editing => {
        //         // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
        //         f.set_cursor(
        //             // Put cursor past the end of the input text
        //             chunks[1].x + ((self.input.visual_cursor()).max(scroll) - scroll) as u16 + 1,
        //             // Move one line down, from the border to the input line
        //             chunks[1].y + 1,
        //         )
        //     }
        // }

        // let messages: Vec<ListItem> = self
        //     .messages
        //     .iter()
        //     .enumerate()
        //     .map(|(i, m)| {
        //         let content = vec![Line::from(Span::raw(format!("{}: {}", i, m)))];
        //         ListItem::new(content)
        //     })
        //     .collect();
        // let messages =
        //     List::new(messages).block(Block::default().borders(Borders::ALL).title("Messages"));
        // f.render_widget(messages, chunks[2]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout() {
        let input_layout = layout(Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        });

        println!("{:#?}", input_layout);
    }
}
