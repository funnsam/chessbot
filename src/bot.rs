use std::sync::mpsc::Receiver;
use crate::lichess::LichessGame;

pub struct Game {
    lichess: LichessGame,
    incoming_events: Receiver<GameEvent>
}

pub enum GameEvent {
    NextGameState {
    },
    OpponentGone,
}
