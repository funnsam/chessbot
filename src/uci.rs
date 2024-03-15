use std::io::{self, BufRead as _};
use std::str::FromStr;
use std::sync::mpsc::*;
use chess::{Square, Board, ChessMove, Piece};
use super::bot::*;

pub struct UciClient {
}

impl UciClient {
    pub fn new() -> Self {
        UciClient {}
    }

    pub async fn start(self) {
        let mut lines = io::stdin().lock().lines();

        let mut game = None;
        let mut last_move = None;

        while let Some(Ok(l)) = lines.next() {
            let tokens = l.split_whitespace();
            match parse_command(tokens) {
                Some(UciCommand::Uci) => {
                    println!("id name funn's bot");
                    println!("uciok");
                },
                Some(UciCommand::IsReady) => println!("readyok"),
                Some(UciCommand::Stop) => std::process::exit(0),
                Some(UciCommand::UciNewGame) => game = None,
                Some(UciCommand::Position { position, moves }) => {
                    last_move = moves.last().cloned();

                    if game.is_none() {
                        let (moves_t, moves_r) = channel();
                        let (event_t, event_r) = channel();

                        let mut new_game = Game::new("".to_string(), position.side_to_move(), position, moves_t);

                        game = Some(event_t);

                        let moves_len = moves.len();
                        new_game.moves = moves.into_iter().take(moves_len - 1).collect();

                        tokio::spawn(async move {
                            new_game.run(event_r);
                        });

                        tokio::spawn(async move {
                            while let Ok(m) = moves_r.recv() {
                                println!("bestmove {m}");
                            }
                        });
                    }
                },
                Some(UciCommand::Go { wtime, btime }) => if let Some(game) = game.as_ref() {
                    if let Some(new_move) = last_move {
                        _ = game.send(GameEvent::NextGameState {
                            new_move,
                            wtime,
                            btime,
                        });
                    } else {
                        _ = game.send(GameEvent::FullGameState {
                            moves: vec![],
                            wtime,
                            btime,
                        });
                    }
                },
                None => {},
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
        moves: Vec<chess::ChessMove>,
    },
    Go {
        wtime: crate::bot::TimeControl,
        btime: crate::bot::TimeControl,
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
                let fen = token.next();
                token.next();
                Board::from_str(fen?).ok()?
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
            let mut wtime = None;
            let mut btime = None;
            let mut winc = None;
            let mut binc = None;

            while let Some(t) = token.next() {
                match t {
                    "wtime" => wtime = Some(token.next()?.parse().ok()?),
                    "btime" => btime = Some(token.next()?.parse().ok()?),
                    "winc" => winc = Some(token.next()?.parse().ok()?),
                    "binc" => binc = Some(token.next()?.parse().ok()?),
                    _ => {},
                }
            }

            Some(UciCommand::Go {
                wtime: TimeControl {
                    time_left: wtime?,
                    time_incr: winc?,
                },
                btime: TimeControl {
                    time_left: btime?,
                    time_incr: binc?,
                },
            })
        },
        Some("stop") => Some(UciCommand::Stop),
        Some(_) => parse_command(token),
        None => None,
    }
}
