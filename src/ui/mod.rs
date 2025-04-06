use chess::{Color, Piece, Square, Rank, File};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color as TuiColor, Style},
    text::{Span, Text, Line},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::game::ChessGame;

pub fn draw_ui<B: Backend>(f: &mut Frame, game: &ChessGame) {
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
    let status = format!("Turn: {}", if game.side_to_move() == Color::White { "White" } else { "Black" });
    let status_widget = Paragraph::new(status)
        .style(Style::default());
    f.render_widget(status_widget, chunks[0]);

    // Chess board
    let board = game.current_position();
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
            let is_selected = game.selected_square() == Some(square);
            let is_possible_move = game.possible_moves().iter().any(|m| m.get_dest() == square);
            
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
                Some(Piece::Pawn) => '♟',
                Some(Piece::Knight) => '♞',
                Some(Piece::Bishop) => '♝',
                Some(Piece::Rook) => '♜',
                Some(Piece::Queen) => '♛',
                Some(Piece::King) => '♚',
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
            .map(|spans| Line::from(spans))
            .collect::<Vec<Line>>()
    );
    
    let board_widget = Paragraph::new(text_content)
        .block(Block::default().borders(Borders::ALL).title("Chess"));
    f.render_widget(board_widget, chunks[1]);

    // Message area
    let message_widget = Paragraph::new(game.message())
        .block(Block::default().borders(Borders::ALL).title("Messages"));
    f.render_widget(message_widget, chunks[2]);
}
