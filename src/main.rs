mod engine;
mod game;
mod ui;

use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};

use chess::{Square, Rank, File};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};

use engine::ChessEngine;
use game::ChessGame;

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create game and engine
    let mut game = ChessGame::new();
    let mut engine = ChessEngine::new();
    
    // Start the Stockfish chess engine
    engine.start("/usr/games/stockfish")?;

    let res = run_app(&mut terminal, &mut game, &mut engine);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    game: &mut ChessGame,
    engine: &mut ChessEngine,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(250);

    loop {
        terminal.draw(|f| ui::draw_ui::<CrosstermBackend<io::Stdout>>(f, game))?;

        // Check for engine moves
        if let Some(best_move) = engine.try_receive_move() {
            if game.make_engine_move(&best_move) {
                // Check game status
                if let Some(result) = game.game_result() {
                    game.set_message(format!("Game over: {:?}", result));
                }
            }
        }

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char(c) if c >= 'a' && c <= 'h' => {
                        // File selection
                        let file = File::from_index((c as u8 - b'a') as usize);
                        
                        // Wait for rank selection
                        if let Event::Key(key) = event::read()? {
                            if let KeyCode::Char(r) = key.code {
                                if r >= '1' && r <= '8' {
                                    let rank = Rank::from_index((r as u8 - b'1') as usize);
                                    let square = Square::make_square(rank, file);
                                    
                                    // Make the move if possible
                                    if game.select_square(square) {
                                        // Check game status
                                        if let Some(result) = game.game_result() {
                                            game.set_message(format!("Game over: {:?}", result));
                                        } else {
                                            // Get engine move
                                            game.set_thinking(true);
                                            let fen = game.current_position().to_string();
                                            if let Err(e) = engine.get_move(&fen) {
                                                game.set_message(format!("Engine error: {}", e));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}
