use hanabi_app::HanabiApp;
use model::{CardFace, CardSuit, GameState, HintAction, PlayerAction, PlayerIndex};
use ratatui::style::Stylize;
use std::io;

use crate::model::{GameOutcome, SlotIndex};
mod client_logic;
mod hanabi_app;
mod logic;
mod model;

fn run_hanabi() -> Result<GameOutcome, String> {
    let num_players: usize = 5;

    let mut game = GameState::start(num_players)?;
    println!("> Starting Game!");

    let mut game_outcome: Option<GameOutcome> = None;

    while let None = game_outcome {
        loop {
            let action_effects = game.play(player_turn(&game)?);
            match action_effects {
                Ok(effects) => {
                    game.run_effects(effects)?;
                    break;
                }
                Err(msg) => println!("Disallowed action: {}", msg),
            }
        }
        game_outcome = game.check_game_outcome();
    }

    match game_outcome {
        Some(game_outcome) => Ok(game_outcome),
        None => Err("Error".to_string()),
    }
}

fn player_turn(game: &GameState) -> Result<PlayerAction, String> {
    loop {
        println!("> What is your move? [play: p (card_index), discard: d (card_index), hint: h (player_index) (suit:RGYWB|face:12345)]");
        let player_action = get_player_input();
        match player_action {
            Ok(player_action) => return Ok(player_action),
            Err(msg) => println!("Failed to parsse action: {}", msg),
        };
    }
}

fn get_player_input() -> Result<PlayerAction, String> {
    todo!()
}

fn init() {
    println!("{}", "Hanabi Simulator v0.1.0".blue());

    let result = run_hanabi();
    print!("Game ended: ");
    match result {
        Ok(GameOutcome::Win) => println!("Won!"),
        Ok(GameOutcome::Fail { score }) => println!("Finished with score: {}", score),
        Err(msg) => println!("Error: {}", msg),
    }
}

use std::{
    error::Error,
    io::{stdout, Stdout},
};

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

// These type aliases are used to make the code more readable by reducing repetition of the generic
// types. They are not necessary for the functionality of the code.
type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;
type BoxedResult<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> BoxedResult<()> {
    let mut terminal = setup_terminal()?;
    let mut app = HanabiApp::new();

    let result = app.run(&mut terminal);
    restore_terminal(terminal)?;

    if let Err(err) = result {
        eprintln!("{err:?}  ");
    }
    Ok(())
}

fn setup_terminal() -> BoxedResult<Terminal> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(mut terminal: Terminal) -> BoxedResult<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
