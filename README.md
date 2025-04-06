# Chess Terminal

A terminal-based chess application written in Rust that uses Unicode chess characters and can connect to standard UCI chess engines.

## Features

- Terminal-based UI with Unicode chess pieces
- Play against UCI-compatible chess engines (like Stockfish)
- Highlight available moves
- Simple keyboard-based interface

## Requirements

- Rust and Cargo
- A UCI-compatible chess engine (like Stockfish) for AI opponent

## Installation

1. Clone this repository
2. Build the project:
   ```
   cd chess_terminal
   cargo build --release
   ```
3. Run the application:
   ```
   cargo run --release
   ```

## Usage

- Select a square by typing its coordinates (e.g., `e2` for the e2 square)
- After selecting a piece, select a destination square to move
- Press `q` to quit the application

## Connecting a Chess Engine

To play against a chess engine, you need to:

1. Download a UCI-compatible chess engine (like Stockfish)
2. Modify the `main.rs` file to point to your engine:
   ```rust
   // In the main function
   app.start_engine("/path/to/your/engine")?;
   ```

## License

MIT
