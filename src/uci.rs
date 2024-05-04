use std::io::{self, BufRead as _};
use std::str::FromStr;
use chess::*;
use super::bot::*;

pub struct UciClient {
}

impl UciClient {
    pub fn new() -> Self {
        UciClient {}
    }

    pub fn start(self) {
        let mut lines = io::stdin().lock().lines();

        let mut game = super::bot::Game::new(Board::default(), Vec::new());
        let mut game_hash = game.board.get_hash();

        while let Some(Ok(l)) = lines.next() {
            info!("uci: `{}`", l);
            let tokens = l.split_whitespace();
            match parse_command(tokens) {
                Some(UciCommand::Uci) => {
                    println!("id name funn's bot");
                    println!("uciok");
                },
                Some(UciCommand::IsReady) => println!("readyok"),
                Some(UciCommand::Stop) => std::process::exit(0),
                Some(UciCommand::UciNewGame) => {},
                Some(UciCommand::Position { mut position, moves }) => {
                    if moves.len() == 0 || game_hash != position.get_hash() {
                        game_hash = position.get_hash();

                        for m in moves.iter() {
                            position = position.make_move_new(*m);
                        }

                        game.board = position;
                        game.moves = moves;
                    } else {
                        game.board = game.board.make_move_new(*moves.last().unwrap());
                    }
                },
                Some(UciCommand::Go { wtime, btime }) => {
                    game.time_ctrl = if matches!(game.board.side_to_move(), Color::White) {
                        wtime
                    } else {
                        btime
                    };
                    println!("bestmove {}", game.play());
                },
                None => {
                    warn!("got unknown uci command");
                },
            }
        }
    }
}

#[derive(Debug)]
enum UciCommand {
    Uci,
    // Debug(bool),
    IsReady,
    UciNewGame,
    Position {
        position: Board,
        moves: Vec<ChessMove>,
    },
    Go {
        wtime: TimeControl,
        btime: TimeControl,
    },
    Stop,
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

fn parse_command<'a, T: Iterator<Item = &'a str>>(mut token: T) -> Option<UciCommand> {
    match token.next() {
        Some("uci") => Some(UciCommand::Uci),
        Some("isready") => Some(UciCommand::IsReady),
        Some("ucinewgame") => Some(UciCommand::UciNewGame),
        Some("position") => {
            let mut moves = Vec::new();
            let next = token.next();
            let board = if matches!(next, Some("fen")) {
                let mut fen = String::new();

                while let Some(t) = token.next() {
                    if t == "moves" {
                        break;
                    }

                    fen += t;
                    fen += " ";
                }

                Board::from_str(fen.trim()).ok()?
            } else if matches!(next, Some("startpos")) {
                token.next();
                Board::default()
            } else {
                return None;
            };

            while let Some(m) = token.next() {
                moves.push(move_from_uci(m));
            }

            Some(UciCommand::Position {
                position: board,
                moves,
            })
        },
        Some("go") => {
            let mut wtime = u32::MAX as usize;
            let mut btime = u32::MAX as usize;
            let mut winc = 0;
            let mut binc = 0;

            while let Some(t) = token.next() {
                match t {
                    "wtime" => wtime = token.next()?.parse().ok()?,
                    "btime" => btime = token.next()?.parse().ok()?,
                    "winc" => winc = token.next()?.parse().ok()?,
                    "binc" => binc = token.next()?.parse().ok()?,
                    _ => {},
                }
            }

            Some(UciCommand::Go {
                wtime: TimeControl {
                    time_left: wtime,
                    time_incr: winc,
                },
                btime: TimeControl {
                    time_left: btime,
                    time_incr: binc,
                },
            })
        },
        Some("stop") => Some(UciCommand::Stop),
        Some(_) => parse_command(token),
        None => None,
    }
}
