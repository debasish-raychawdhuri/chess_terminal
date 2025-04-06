use std::{
    error::Error,
    io,
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use chess::{Board, ChessMove, Color, Game, MoveGen, Piece, Square, Rank, File};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color as TuiColor, Style},
    text::{Span, Text},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io::{BufRead, BufReader, Write};

// App state
struct App {
    game: Game,
    selected_square: Option<Square>,
    possible_moves: Vec<ChessMove>,
    engine_process: Option<std::process::Child>,
    engine_move: Option<ChessMove>,
    player_color: Color,
    message: String,
    thinking: bool,
    engine_move_receiver: Option<mpsc::Receiver<String>>,
    engine_move_sender: Option<mpsc::Sender<String>>,
}

impl App {
    fn new() -> Self {
        // Create a channel for engine moves
        let (tx, rx) = mpsc::channel();
        
        App {
            game: Game::new(),
            selected_square: None,
            possible_moves: Vec::new(),
            engine_process: None,
            engine_move: None,
            player_color: Color::White,
            message: String::from("Welcome to Chess Terminal! You play as White."),
            thinking: false,
            engine_move_receiver: Some(rx),
            engine_move_sender: Some(tx),
        }
    }

    fn start_engine(&mut self, engine_path: &str) -> Result<(), Box<dyn Error>> {
        let process = Command::new(engine_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        self.engine_process = Some(process);
        
        // Initialize UCI engine
        if let Some(ref mut process) = self.engine_process {
            let mut stdin = process.stdin.take().unwrap();
            stdin.write_all(b"uci\n")?;
            stdin.write_all(b"isready\n")?;
            stdin.write_all(b"setoption name Skill Level value 10\n")?; // Set skill level (1-20)
            stdin.write_all(b"setoption name Threads value 4\n")?; // Use 4 threads
            stdin.write_all(b"setoption name Hash value 128\n")?; // Use 128MB hash
            stdin.write_all(b"setoption name UCI_AnalyseMode value false\n")?;
            stdin.write_all(b"setoption name UCI_LimitStrength value false\n")?;
            stdin.flush()?;
            
            // Read engine output in a separate thread
            let stdout = process.stdout.take().unwrap();
            let reader = BufReader::new(stdout);
            
            // Get a clone of the sender to pass to the thread
            let tx_clone = self.engine_move_sender.as_ref().unwrap().clone();
            
            thread::spawn(move || {
                for line in reader.lines() {
                    if let Ok(line) = line {
                        if line.starts_with("bestmove") {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if parts.len() >= 2 {
                                tx_clone.send(parts[1].to_string()).unwrap_or(());
                            }
                        }
                    }
                }
            });
            
            // Return stdin to the process
            process.stdin = Some(stdin);
        }
        
        Ok(())
    }

    fn get_engine_move(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(ref mut process) = self.engine_process {
            if let Some(stdin) = process.stdin.as_mut() {
                // Send position to engine
                let fen = self.game.current_position().to_string();
                let position_cmd = format!("position fen {}\n", fen);
                stdin.write_all(position_cmd.as_bytes())?;
                
                // Ask engine to think
                stdin.write_all(b"go movetime 2000\n")?;
                stdin.flush()?;
                
                self.thinking = true;
                self.message = "Engine is thinking...".to_string();
            }
        }
        
        Ok(())
    }

    fn select_square(&mut self, square: Square) {
        if let Some(_selected) = self.selected_square {
            // If a square is already selected, try to make a move
            let possible_move = self.possible_moves.iter().find(|m| m.get_dest() == square);
            
            if let Some(chess_move) = possible_move {
                if self.game.make_move(*chess_move) {
                    self.message = format!("Move: {}", chess_move);
                    self.selected_square = None;
                    self.possible_moves.clear();
                    
                    // Check game status
                    match self.game.result() {
                        Some(result) => {
                            self.message = format!("Game over: {:?}", result);
                        }
                        None => {
                            // Get engine move
                            if let Err(e) = self.get_engine_move() {
                                self.message = format!("Engine error: {}", e);
                            }
                        }
                    }
                }
            } else {
                // Select a new square
                self.selected_square = Some(square);
                self.update_possible_moves();
            }
        } else {
            // Select the square if it has a piece of the current player's color
            let board = self.game.current_position();
            if let Some(_piece) = board.piece_on(square) {
                if board.color_on(square) == Some(self.game.side_to_move()) {
                    self.selected_square = Some(square);
                    self.update_possible_moves();
                }
            }
        }
    }

    fn update_possible_moves(&mut self) {
        self.possible_moves.clear();
        
        if let Some(square) = self.selected_square {
            let board = self.game.current_position();
            let move_gen = MoveGen::new_legal(&board);
            
            for chess_move in move_gen {
                if chess_move.get_source() == square {
                    self.possible_moves.push(chess_move);
                }
            }
        }
    }
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(250);

    loop {
        terminal.draw(|f| ui(f, &app))?;

        // Check for engine moves
        if let Some(ref rx) = app.engine_move_receiver {
            if let Ok(best_move) = rx.try_recv() {
                // Try to parse UCI format (e.g., "d2d4")
                if best_move.len() >= 4 {
                    let from_file = (best_move.chars().nth(0).unwrap() as u8 - b'a') as usize;
                    let from_rank = (best_move.chars().nth(1).unwrap() as u8 - b'1') as usize;
                    let to_file = (best_move.chars().nth(2).unwrap() as u8 - b'a') as usize;
                    let to_rank = (best_move.chars().nth(3).unwrap() as u8 - b'1') as usize;
                    
                    if from_file < 8 && from_rank < 8 && to_file < 8 && to_rank < 8 {
                        let from_square = Square::make_square(
                            Rank::from_index(from_rank),
                            File::from_index(from_file)
                        );
                        let to_square = Square::make_square(
                            Rank::from_index(to_rank),
                            File::from_index(to_file)
                        );
                        
                        // Find the move in legal moves
                        let board = app.game.current_position();
                        let move_gen = MoveGen::new_legal(&board);
                        
                        for m in move_gen {
                            if m.get_source() == from_square && m.get_dest() == to_square {
                                // Handle promotion if needed
                                let promotion = if best_move.len() >= 5 {
                                    match best_move.chars().nth(4).unwrap() {
                                        'q' => Some(Piece::Queen),
                                        'r' => Some(Piece::Rook),
                                        'b' => Some(Piece::Bishop),
                                        'n' => Some(Piece::Knight),
                                        _ => None
                                    }
                                } else {
                                    None
                                };
                                
                                if promotion.is_none() || m.get_promotion() == promotion {
                                    if app.game.make_move(m) {
                                        app.message = format!("Engine moved: {}", best_move);
                                        app.thinking = false;
                                        break;
                                    }
                                }
                            }
                        }
                    }
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
                                    app.select_square(square);
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

fn ui(f: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),  // Status line
            Constraint::Min(10),    // Chess board
            Constraint::Length(3),  // Message area
        ])
        .split(f.size());

    // Status line
    let status = format!("Turn: {}", if app.game.side_to_move() == Color::White { "White" } else { "Black" });
    let status_widget = Paragraph::new(status)
        .style(Style::default());
    f.render_widget(status_widget, chunks[0]);

    // Chess board
    let board = app.game.current_position();
    let mut board_text = Vec::new();
    
    // Add column labels
    let mut header_spans = Vec::new();
    header_spans.push(Span::raw("  "));
    for file in 0..8 {
        header_spans.push(Span::raw(format!(" {} ", (b'a' + file) as char)));
    }
    board_text.push(header_spans);
    
    // Add board with rank labels
    for rank in (0..8).rev() {
        let mut row_spans = Vec::new();
        row_spans.push(Span::raw(format!("{} ", rank + 1)));
        
        for file in 0..8 {
            let square = Square::make_square(
                Rank::from_index(rank as usize),
                File::from_index(file as usize)
            );
            let is_dark = (rank + file) % 2 == 1;
            let is_selected = app.selected_square == Some(square);
            let is_possible_move = app.possible_moves.iter().any(|m| m.get_dest() == square);
            
            let bg_color = if is_selected {
                TuiColor::Yellow
            } else if is_possible_move {
                TuiColor::LightGreen
            } else if is_dark {
                TuiColor::Rgb(101, 67, 33) // Dark brown
            } else {
                TuiColor::Rgb(210, 180, 140) // Light brown (tan)
            };
            
            let piece_char = match board.piece_on(square) {
                Some(Piece::Pawn) => {
                    if board.color_on(square) == Some(Color::White) { '♟' } else { '♟' }
                },
                Some(Piece::Knight) => {
                    if board.color_on(square) == Some(Color::White) { '♞' } else { '♞' }
                },
                Some(Piece::Bishop) => {
                    if board.color_on(square) == Some(Color::White) { '♝' } else { '♝' }
                },
                Some(Piece::Rook) => {
                    if board.color_on(square) == Some(Color::White) { '♜' } else { '♜' }
                },
                Some(Piece::Queen) => {
                    if board.color_on(square) == Some(Color::White) { '♛' } else { '♛' }
                },
                Some(Piece::King) => {
                    if board.color_on(square) == Some(Color::White) { '♚' } else { '♚' }
                },
                None => ' ',
            };
            
            let fg_color = if board.color_on(square) == Some(Color::White) {
                TuiColor::Rgb(255, 255, 255) // Bright white for better visibility
            } else {
                TuiColor::Black
            };
            
            let piece_span = Span::styled(
                format!(" {} ", piece_char),
                Style::default().fg(fg_color).bg(bg_color),
            );
            row_spans.push(piece_span);
        }
        
        board_text.push(row_spans);
    }
    
    // Convert Vec<Vec<Span>> to Text for Paragraph
    let text_content = Text::from(
        board_text.into_iter()
            .map(|spans| ratatui::text::Line::from(spans))
            .collect::<Vec<ratatui::text::Line>>()
    );
    
    let board_widget = Paragraph::new(text_content)
        .block(Block::default().borders(Borders::ALL).title("Chess"));
    f.render_widget(board_widget, chunks[1]);

    // Message area
    let message_widget = Paragraph::new(app.message.clone())
        .block(Block::default().borders(Borders::ALL).title("Messages"));
    f.render_widget(message_widget, chunks[2]);
}

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new();
    
    // Start the Stockfish chess engine
    app.start_engine("/usr/games/stockfish")?;

    let res = run_app(&mut terminal, app);

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
