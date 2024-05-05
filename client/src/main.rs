mod hanabi_app;

use std::{
    error::Error,
    io::{stdout, Stdout},
};

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use hanabi_app::HanabiApp;
use ratatui::prelude::*;
use shared::{
    client_logic::GameLog,
    model::{GameConfig, PlayerIndex},
};

// These type aliases are used to make the code more readable by reducing repetition of the generic
// types. They are not necessary for the functionality of the code.
type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;
type BoxedResult<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> BoxedResult<()> {
    let mut terminal = setup_terminal()?;

    let mut app = HanabiApp::new(GameLog::new(GameConfig {
        num_players: 4,
        hand_size: 4,
        num_fuses: 3,
        num_hints: 8,
        starting_player: PlayerIndex(0),
        seed: 0,
    }));

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
