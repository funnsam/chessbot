use chess::*;
use super::config::*;
use super::eval::*;
use rayon::prelude::*;
use std::sync::*;

impl super::Game {
    pub fn search(&mut self) -> (ChessMove, f32) {
        let max_eval = Mutex::new(f32::NEG_INFINITY);

        let gen = MoveGen::new_legal(&self.lichess.board);
        let mut moves = Vec::with_capacity(gen.len());

        for m in gen {
            let board = self.lichess.board.make_move_new(m);
            let eval = super::eval::evaluate(&board);
            moves.push((m, eval));
        }

        moves.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        for i in 1..=MAX_SEARCH_DEPTH {
            let start = std::time::Instant::now();

            moves.par_iter_mut().enumerate().for_each(|(j, (m, e))| {
                let board = self.lichess.board.make_move_new(*m);

                let mut depth = i;
                depth -= (j >= REDUCED_SEARCH_DEPTH) as usize;

                let mut eval = -self.search_alpha_beta(
                    board,
                    depth,
                    EXTEND_SEARCH_LIMIT,
                    f32::NEG_INFINITY, // beta is initially -inf
                    -*max_eval.lock().unwrap(), // doing what -alpha does
                );

                if self.times_up() {
                    return;
                }

                if eval > *max_eval.lock().unwrap() && j >= REDUCED_SEARCH_DEPTH {
                    let new_eval = -self.search_alpha_beta(
                        board,
                        depth + 1,
                        EXTEND_SEARCH_LIMIT,
                        f32::NEG_INFINITY,
                        -eval, // best is now eval
                    );

                    if !self.times_up() {
                        dbg!("{} ({}th move) better than predicted (old: {}, new: {})", m, j, eval, new_eval);

                        eval = new_eval;
                    }
                }

                if eval > *max_eval.lock().unwrap() {
                    *max_eval.lock().unwrap() = eval;
                }

                *e = eval;
            });

            moves.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

            info!("depth {} searched in {:.2}s", i, start.elapsed().as_secs_f32());

            if self.times_up() {
                break;
            }
        }

        moves.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        for m in moves.iter(){
            dbg!("{} {}", m.0, m.1);
        }

        moves.first().unwrap().clone()
    }

    fn search_alpha_beta(
        &self,
        current: Board,
        depth: usize,
        ext_depth: usize,
        mut alpha: f32,
        beta: f32,
    ) -> f32 {
        if self.times_up() {
            return 0.0;
        }

        if matches!(current.status(), BoardStatus::Checkmate) {
            return f32::NEG_INFINITY;
        } else if matches!(current.status(), BoardStatus::Stalemate) {
            return 0.0;
        }

        if let Some(t_e) = self.trans_table.lock().unwrap().get(&current.get_hash()) {
            if t_e.depth >= depth {
                return t_e.eval;
            }
        }

        if depth == 0 {
            return self.quiescene_search(current, alpha, beta);
        }

        for m in move_in_order(&current).into_iter() {
            let after = current.make_move_new(m);
            let mut eval = 0.0;

            // capture bonus
            if current.color_on(m.get_dest()) == Some(!current.side_to_move()) {
                eval += PIECE_VALUE[current.piece_on(m.get_dest()).unwrap().to_index()];
                eval -= PIECE_VALUE[current.piece_on(m.get_source()).unwrap().to_index()] * 0.25;
            }

            let mut ext = 0;
            ext += (after.checkers().0 != 0) as usize;

            eval -= self.search_alpha_beta(
                after,
                depth - 1 + ext.min(ext_depth),
                ext_depth - ext,
                -beta,
                -alpha
            );

            if self.times_up() {
                return 0.0;
            }

            self.trans_table.lock().unwrap().insert(current.get_hash(),
                super::trans_table::TransTableEntry {
                    depth,
                    eval,
                    age: self.age,
                }
            );

            if eval >= beta {
                return eval;
            } else if eval > alpha {
                alpha = eval;
            }
        }

        alpha
    }

    pub fn quiescene_search(
        &self,
        current: Board,
        mut alpha: f32,
        beta: f32,
    ) -> f32 {
        let eval = evaluate(&current);

        if eval >= beta {
            return eval;
        } else if eval > alpha {
            alpha = eval;
        }

        let mut movegen = MoveGen::new_legal(&current);
        let mask = current.color_combined(!current.side_to_move());
        movegen.set_iterator_mask(*mask);

        for m in movegen {
            let board = current.make_move_new(m);
            let eval = -self.quiescene_search(board, -beta, -alpha);

            if eval >= beta {
                return eval;
            } else if eval > alpha {
                alpha = eval;
            }
        }

        alpha
    }
}

fn move_in_order(board: &Board) -> Vec<ChessMove> {
    let gen = MoveGen::new_legal(&board);
    let mut buf = Vec::with_capacity(gen.len());

    buf.extend(gen);

    buf.sort_by(|a, b| {
        let a = evaluate(&board.make_move_new(*a));
        let b = evaluate(&board.make_move_new(*b));

        a.partial_cmp(&b).unwrap()
    });

    buf
}
