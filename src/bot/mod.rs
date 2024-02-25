mod eval;
mod search;

use std::sync::mpsc::*;
use crate::lichess::LichessGame;
use chess::*;

pub struct Game {
    pub lichess: LichessGame,
    pub incoming_events: Receiver<GameEvent>,
    pub outgoing_moves: Sender<ChessMove>,
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
#[derive(Debug)]
pub struct TimeControl {
    pub time_left: usize,
    pub time_incr: usize,
}

impl Game {
    pub fn run(mut self) {
        while let Ok(event) = self.incoming_events.recv() {
            match event {
                GameEvent::FullGameState { moves, wtime, btime, status } => {
                    let eval = eval::evaluate(&self.lichess.board);
                    info!("game `{}` eval {}", &self.lichess.id, eval);

                    if self.lichess.board.side_to_move() == self.lichess.color {
                        info!("start search");
                        let next = search::search(&self.lichess);
                        info!("next move: {}", next);
                        self.outgoing_moves.send(next).unwrap();
                    }
                },
                GameEvent::NextGameState { moves, wtime, btime, status } => {
                    let m = move_from_uci(moves.split_whitespace().last().unwrap());
                    self.lichess.board = self.lichess.board.make_move_new(m);

                    let eval = eval::evaluate(&self.lichess.board);
                    info!("game `{}` eval {}", &self.lichess.id, eval);

                    if self.lichess.board.side_to_move() == self.lichess.color {
                        info!("start search");
                        let next = search::search(&self.lichess);
                        info!("next move: {}", next);
                        self.outgoing_moves.send(next).unwrap();
                    }
                },
                _ => {},
            }
        }

        info!("no more events (id: `{}`)", self.lichess.id);
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
