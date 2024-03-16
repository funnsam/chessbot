mod eval;
mod search;
pub mod trans_table;
pub mod config;

use std::sync::mpsc::*;
use chess::*;
use std::time::*;

pub struct Game {
    pub id: String,
    pub color: chess::Color,
    pub board: chess::Board,

    pub moves: Vec<ChessMove>,

    pub outgoing_moves: Sender<ChessMove>,

    pub trans_table: trans_table::TransTable,
    pub age: usize,

    pub time_ctrl: TimeControl,
    pub time_ref: Instant,
    pub time_usable: Duration,
}

#[derive(Debug)]
pub enum GameEvent {
    FullGameState {
        // moves: String,
        moves: Vec<ChessMove>,
        wtime: TimeControl,
        btime: TimeControl,
        // status: String,
    },
    NextGameState {
        // moves: String,
        new_move: ChessMove,
        wtime: TimeControl,
        btime: TimeControl,
        // status: String,
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
    pub fn new(id: String, color: chess::Color, board: chess::Board, outgoing_moves: Sender<ChessMove>) -> Self {
        Game {
            id,
            color,
            board,
            moves: Vec::new(),

            outgoing_moves,

            trans_table: crate::bot::trans_table::TransTable::new(),
            age: 1,

            time_ctrl: TimeControl::default(),
            time_ref: std::time::Instant::now(),
            time_usable: std::time::Duration::from_secs(0),
        }
    }

    pub fn play(&mut self) {
        let eval = Self::quiescene_search(
            self.board,
            eval::MIN_EVAL,
            eval::MAX_EVAL
        );

        info!(
            "game `{}` eval {:.1} (for {})",
            &self.id,
            eval,
            match self.board.side_to_move() {
                Color::White => "white",
                Color::Black => "black",
            }
        );

        if self.board.side_to_move() == self.color {
            self.age += 1;
            info!("start search");
            self.reserve_time();
            let (next, eval) = self.search();
            info!("next move: {} (eval: {})", next, eval);
            self.outgoing_moves.send(next).unwrap();

            self.board = self.board.make_move_new(next);
        }
    }

    pub fn run(mut self, events: Receiver<GameEvent>) {
        while let Ok(event) = events.recv() {
            match event {
                GameEvent::FullGameState { moves, wtime, btime, .. } => {
                    self.moves = moves;

                    self.time_ctrl = if matches!(self.color, Color::White) {
                        wtime
                    } else {
                        btime
                    };
                    self.time_ref = Instant::now();

                    self.play();
                },
                GameEvent::NextGameState { new_move, wtime, btime, .. } => {
                    self.time_ctrl = if matches!(self.color, Color::White) {
                        wtime
                    } else {
                        btime
                    };
                    self.time_ref = Instant::now();

                    self.board = self.board.make_move_new(new_move);
                    self.moves.push(new_move);

                    self.play();
                },
                _ => {},
            }
        }

        info!("no more events (id: `{}`)", self.id);
    }

    pub fn reserve_time(&mut self) {
        // https://github.com/SebLague/Chess-Coding-Adventure/blob/Chess-V2-UCI/Chess-Coding-Adventure/src/Bot.cs#L64

        let left = self.time_ctrl.time_left as u64;
        let incr = self.time_ctrl.time_incr as u64;

        let mut think_time = left / 40;

        if left > incr << 2 {
            think_time += incr * 4 / 5;
        }

        let min_think = (left / 4).min(50);
        self.time_usable = Duration::from_millis(min_think.max(think_time));
    }

    pub fn times_up(&self) -> bool {
        self.time_ref.elapsed() > self.time_usable
    }
}

fn move_from_uci(m: &str) -> ChessMove {
    let src = &m[0..2];
    let src = unsafe {
        Square::new(((src.as_bytes()[1] - b'1') << 3) + (src.as_bytes()[0] - b'a'))
    };

    let dst = &m[2..4];
    let dst = unsafe {
        Square::new(((dst.as_bytes()[1] - b'1') << 3) + (dst.as_bytes()[0] - b'a'))
    };

    let piece = m.as_bytes().get(4).and_then(|p| match p {
        b'n' => Some(Piece::Knight),
        b'b' => Some(Piece::Bishop),
        b'q' => Some(Piece::Queen),
        b'r' => Some(Piece::Rook),
        _ => None,
    });

    ChessMove::new(src, dst, piece)
}
