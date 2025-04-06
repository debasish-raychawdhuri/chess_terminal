use std::{
    error::Error,
    io::{BufRead, BufReader, Write},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
};

pub struct ChessEngine {
    process: Option<Child>,
    move_receiver: mpsc::Receiver<String>,
    move_sender: mpsc::Sender<String>,
}

impl ChessEngine {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        
        ChessEngine {
            process: None,
            move_receiver: rx,
            move_sender: tx,
        }
    }
    
    pub fn start(&mut self, engine_path: &str) -> Result<(), Box<dyn Error>> {
        let process = Command::new(engine_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        self.process = Some(process);
        
        // Initialize UCI engine
        if let Some(ref mut process) = self.process {
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
            let tx_clone = self.move_sender.clone();
            
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
    
    pub fn get_move(&mut self, fen: &str) -> Result<(), Box<dyn Error>> {
        if let Some(ref mut process) = self.process {
            if let Some(stdin) = process.stdin.as_mut() {
                // Send position to engine
                let position_cmd = format!("position fen {}\n", fen);
                stdin.write_all(position_cmd.as_bytes())?;
                
                // Ask engine to think
                stdin.write_all(b"go movetime 2000\n")?;
                stdin.flush()?;
            }
        }
        
        Ok(())
    }
    
    pub fn try_receive_move(&self) -> Option<String> {
        match self.move_receiver.try_recv() {
            Ok(best_move) => Some(best_move),
            Err(_) => None,
        }
    }
}

impl Drop for ChessEngine {
    fn drop(&mut self) {
        if let Some(ref mut process) = self.process {
            if let Some(stdin) = process.stdin.as_mut() {
                let _ = stdin.write_all(b"quit\n");
                let _ = stdin.flush();
            }
            let _ = process.kill();
        }
    }
}
