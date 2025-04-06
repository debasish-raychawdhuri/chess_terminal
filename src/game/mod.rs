use chess::{ChessMove, Color, Game, MoveGen, Piece, Square, Rank, File};

pub struct ChessGame {
    game: Game,
    selected_square: Option<Square>,
    possible_moves: Vec<ChessMove>,
    message: String,
    thinking: bool,
}

impl ChessGame {
    pub fn new() -> Self {
        ChessGame {
            game: Game::new(),
            selected_square: None,
            possible_moves: Vec::new(),
            message: String::from("Welcome to Chess Terminal! You play as White."),
            thinking: false,
        }
    }
    
    pub fn current_position(&self) -> chess::Board {
        self.game.current_position()
    }
    
    pub fn side_to_move(&self) -> Color {
        self.game.side_to_move()
    }
    
    pub fn selected_square(&self) -> Option<Square> {
        self.selected_square
    }
    
    pub fn possible_moves(&self) -> &Vec<ChessMove> {
        &self.possible_moves
    }
    
    pub fn message(&self) -> &str {
        &self.message
    }
    
    pub fn is_thinking(&self) -> bool {
        self.thinking
    }
    
    pub fn set_thinking(&mut self, thinking: bool) {
        self.thinking = thinking;
        if thinking {
            self.message = "Engine is thinking...".to_string();
        }
    }
    
    pub fn set_message(&mut self, message: String) {
        self.message = message;
    }
    
    pub fn select_square(&mut self, square: Square) -> bool {
        let board = self.game.current_position();
        
        if let Some(_selected) = self.selected_square {
            // If a square is already selected, try to make a move
            let possible_move = self.possible_moves.iter().find(|m| m.get_dest() == square);
            
            if let Some(chess_move) = possible_move {
                if self.game.make_move(*chess_move) {
                    self.message = format!("Move: {}", chess_move);
                    self.selected_square = None;
                    self.possible_moves.clear();
                    return true;
                }
            } else {
                // Select a new square if it has a piece of the current player's color
                if let Some(_piece) = board.piece_on(square) {
                    if board.color_on(square) == Some(self.game.side_to_move()) {
                        self.selected_square = Some(square);
                        self.update_possible_moves();
                    } else {
                        self.selected_square = None;
                        self.possible_moves.clear();
                    }
                } else {
                    self.selected_square = None;
                    self.possible_moves.clear();
                }
            }
        } else {
            // Select the square if it has a piece of the current player's color
            if let Some(_piece) = board.piece_on(square) {
                if board.color_on(square) == Some(self.game.side_to_move()) {
                    self.selected_square = Some(square);
                    self.update_possible_moves();
                }
            }
        }
        
        false
    }
    
    pub fn update_possible_moves(&mut self) {
        self.possible_moves.clear();
        
        if let Some(square) = self.selected_square {
            let board = self.game.current_position();
            let move_gen = MoveGen::new_legal(&board);
            
            for m in move_gen {
                if m.get_source() == square {
                    self.possible_moves.push(m);
                }
            }
        }
    }
    
    pub fn make_engine_move(&mut self, uci_move: &str) -> bool {
        if uci_move.len() < 4 {
            return false;
        }
        
        let from_file = (uci_move.chars().nth(0).unwrap() as u8 - b'a') as usize;
        let from_rank = (uci_move.chars().nth(1).unwrap() as u8 - b'1') as usize;
        let to_file = (uci_move.chars().nth(2).unwrap() as u8 - b'a') as usize;
        let to_rank = (uci_move.chars().nth(3).unwrap() as u8 - b'1') as usize;
        
        if from_file >= 8 || from_rank >= 8 || to_file >= 8 || to_rank >= 8 {
            return false;
        }
        
        let from_square = Square::make_square(
            Rank::from_index(from_rank),
            File::from_index(from_file)
        );
        let to_square = Square::make_square(
            Rank::from_index(to_rank),
            File::from_index(to_file)
        );
        
        // Find the move in legal moves
        let board = self.game.current_position();
        let move_gen = MoveGen::new_legal(&board);
        
        for m in move_gen {
            if m.get_source() == from_square && m.get_dest() == to_square {
                // Handle promotion if needed
                let promotion = if uci_move.len() >= 5 {
                    match uci_move.chars().nth(4).unwrap() {
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
                    if self.game.make_move(m) {
                        self.message = format!("Engine moved: {}", uci_move);
                        self.thinking = false;
                        return true;
                    }
                }
            }
        }
        
        false
    }
    
    pub fn game_result(&self) -> Option<chess::GameResult> {
        self.game.result()
    }
}
