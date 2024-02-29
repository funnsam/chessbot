mod eval;
mod search;
pub mod trans_table;
pub mod config;

use std::sync::mpsc::*;
use crate::lichess::LichessGame;
use chess::*;
use std::time::*;

pub struct Game {
    pub lichess: LichessGame,
    pub incoming_events: Receiver<GameEvent>,
    pub outgoing_moves: Sender<ChessMove>,

    pub trans_table: trans_table::TransTable,
    pub age: usize,

    pub time_ctrl: TimeControl,
    pub time_ref: Instant
}

#[derive(Debug)]
pub enum GameEvent {
    FullGameState {
        moves: String,
        wtime: TimeControl,
        btime: TimeControl,
        status: String,
    },
    NextGameState {
        moves: String,
        wtime: TimeControl,
        btime: TimeControl,
        status: String,
    },
    OpponentGone,
    OpponentBack,
}

// ms
#[derive(Debug, Default)]
pub struct TimeControl {
    pub time_left: usize,
    pub time_incr: usize,
}

impl Game {
    pub fn play(&mut self) {
        let eval = self.quiescene_search(self.lichess.board.clone(), i32::MIN + 1, i32::MAX);
        info!(
            "game `{}` eval {} (for {})",
            &self.lichess.id,
            eval,
            match self.lichess.board.side_to_move() {
                Color::White => "white",
                Color::Black => "black",
            }
        );

        if self.lichess.board.side_to_move() == self.lichess.color {
            self.age += 1;
            info!("start search");
            let next = self.search();
            info!("next move: {}", next);
            info!("tt usage: {:.01}%", self.trans_table.usage() * 100.0);
            self.outgoing_moves.send(next).unwrap();
        }
    }

    pub fn run(mut self) {
        while let Ok(event) = self.incoming_events.recv() {
            match event {
                GameEvent::FullGameState { wtime, btime, .. } => {
                    self.time_ctrl = if matches!(self.lichess.color, Color::White) {
                        wtime
                    } else {
                        btime
                    };
                    self.time_ref = Instant::now();

                    self.play()
                },
                GameEvent::NextGameState { moves, wtime, btime, .. } => {
                    self.time_ctrl = if matches!(self.lichess.color, Color::White) {
                        wtime
                    } else {
                        btime
                    };
                    self.time_ref = Instant::now();

                    let m = move_from_uci(moves.split_whitespace().last().unwrap());
                    self.lichess.board = self.lichess.board.make_move_new(m);

                    self.play();
                },
                _ => {},
            }
        }

        info!("no more events (id: `{}`)", self.lichess.id);
    }

    pub fn times_up(&self) -> bool {
        // https://github.com/SebLague/Chess-Coding-Adventure/blob/Chess-V2-UCI/Chess-Coding-Adventure/src/Bot.cs#L64

        // +100 to est time needed for searching with more depth
        let since_start = self.time_ref.elapsed().as_millis() as isize + 100;
        let left = self.time_ctrl.time_left as isize;
        let incr = self.time_ctrl.time_incr as isize;

        let mut think_time = left / 40;

        if left > incr << 2 {
            think_time += incr * 4 / 5;
        }

        let min_think = (left / 4).min(50);
        since_start > min_think.max(think_time)
    }
}

fn move_from_uci(m: &str) -> ChessMove {
    let src = &m[0..2];
    let src = unsafe {
        Square::new(((src.as_bytes()[1] as u8 - b'1') << 3) + (src.as_bytes()[0] as u8 - b'a'))
    };

    let dst = &m[2..4];
    let dst = unsafe {
        Square::new(((dst.as_bytes()[1] as u8 - b'1') << 3) + (dst.as_bytes()[0] as u8 - b'a'))
    };

    let piece = if let Some(p) = m.chars().nth(4) {
        match p {
            'n' => Some(Piece::Knight),
            'b' => Some(Piece::Bishop),
            'q' => Some(Piece::Queen),
            'r' => Some(Piece::Rook),
            _ => None,
        }
    } else {
        None
    };

    ChessMove::new(src, dst, piece)
}
