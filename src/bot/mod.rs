mod eval;

use std::sync::mpsc::*;
use crate::lichess::LichessGame;
use chess::*;

pub struct Game {
    pub lichess: LichessGame,
    pub incoming_events: Receiver<GameEvent>,
    pub outgoing_moves: Sender<ChessMove>,
}

pub enum GameEvent {
    NextGameState {
        moves: String,
        wtime: TimeControl,
        btime: TimeControl,
    },
    OpponentGone,
    OpponentBack,
}

// ms
pub struct TimeControl {
    time_left: usize,
    time_incr: usize,
}
