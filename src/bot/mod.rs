mod eval;
mod search;
pub mod trans_table;
pub mod config;

use std::sync::atomic::*;
use chess::*;
use std::time::*;

pub struct Game {
    pub board: chess::Board,
    pub init_board: chess::Board,
    pub moves: Vec<ChessMove>,

    pub trans_table: trans_table::TransTable,
    pub age: usize,

    pub time_ctrl: TimeControl,
    pub time_ref: Instant,
    pub time_usable: Duration,

    pub searched: AtomicUsize,
}

// #[derive(Debug)]
// pub enum GameEvent {
//     FullGameState {
//         // moves: String,
//         moves: Vec<ChessMove>,
//         wtime: TimeControl,
//         btime: TimeControl,
//         // status: String,
//     },
//     NextGameState {
//         // moves: String,
//         new_move: ChessMove,
//         wtime: TimeControl,
//         btime: TimeControl,
//         // status: String,
//     },
//     OpponentGone,
//     OpponentBack,
// }

// ms
#[derive(Debug, Default)]
pub struct TimeControl {
    pub time_left: usize,
    pub time_incr: usize,
}

impl Game {
    pub fn new(board: Board, init_board: Board, moves: Vec<ChessMove>) -> Self {
        Game {
            board,
            init_board,
            moves,

            trans_table: trans_table::TransTable::new(),
            age: 1,

            time_ctrl: TimeControl::default(),
            time_ref: Instant::now(),
            time_usable: Duration::from_secs(0),

            searched: AtomicUsize::new(0),
        }
    }

    pub fn play(&mut self) -> ChessMove {
        self.time_ref = Instant::now();
        self.age += 1;
        info!("start search");
        self.reserve_time();
        let (next, eval) = self.search();
        info!("next move: {} (eval: {})", next, eval);
        self.moves.push(next);
        self.board = self.board.make_move_new(next);

        next
    }

    // pub fn play(&mut self) {
    //     let eval = Self::quiescene_search(
    //         self.board,
    //         eval::MIN_EVAL,
    //         eval::MAX_EVAL
    //     );

    //     info!(
    //         "game `{}` eval {:.1} (for {})",
    //         &self.id,
    //         eval,
    //         match self.board.side_to_move() {
    //             Color::White => "white",
    //             Color::Black => "black",
    //         }
    //     );

    //     if self.board.side_to_move() == self.color {
    //         self.time_ref = Instant::now();
    //         self.age += 1;
    //         info!("start search");
    //         self.reserve_time();
    //         let (next, eval) = self.search();
    //         info!("next move: {} (eval: {})", next, eval);
    //         self.outgoing_moves.send(next).unwrap();

    //         self.board = self.board.make_move_new(next);
    //     }
    // }

    // pub fn run(mut self, events: Receiver<GameEvent>) {
    //     while let Ok(event) = events.recv() {
    //         match event {
    //             GameEvent::FullGameState { moves, wtime, btime, .. } => {
    //                 self.moves = moves;

    //                 self.time_ctrl = if matches!(self.color, Color::White) {
    //                     wtime
    //                 } else {
    //                     btime
    //                 };

    //                 self.play();
    //             },
    //             GameEvent::NextGameState { new_move, wtime, btime, .. } => {
    //                 self.time_ctrl = if matches!(self.color, Color::White) {
    //                     wtime
    //                 } else {
    //                     btime
    //                 };

    //                 self.board = self.board.make_move_new(new_move);
    //                 self.moves.push(new_move);

    //                 self.play();
    //             },
    //             _ => {},
    //         }
    //     }

    //     info!("no more events (id: `{}`)", self.id);
    // }

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
