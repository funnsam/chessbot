use chess::*;
use super::config::*;
use super::eval::*;
use rayon::prelude::*;
use std::sync::*;

impl super::Game {
    pub fn search(&mut self) -> (ChessMove, i32) {
        let max_eval = Mutex::new(MIN_EVAL);

        let gen = MoveGen::new_legal(&self.lichess.board);
        let mut moves = Vec::with_capacity(gen.len());

        for m in gen {
            let board = self.lichess.board.make_move_new(m);
            let eval = super::eval::evaluate(&board);
            moves.push((m, eval));
        }

        moves.sort_by_key(|a| -a.1);

        for i in 1..=MAX_SEARCH_DEPTH {
            let start = std::time::Instant::now();

            moves.par_iter_mut().enumerate().for_each(|(j, (m, e))| {
                let board = self.lichess.board.make_move_new(*m);

                let mut depth = i;
                depth -= (j >= REDUCED_SEARCH_DEPTH) as usize;

                let mut eval = -self.search_alpha_beta(
                    board,
                    depth,
                    SEARCH_EXTENSION_LIMIT,
                    MIN_EVAL,
                    -*max_eval.lock().unwrap(),
                );

                if self.times_up() {
                    return;
                }

                if eval > *max_eval.lock().unwrap() && j >= REDUCED_SEARCH_DEPTH {
                    let new_eval = -self.search_alpha_beta(
                        board,
                        depth + 1,
                        SEARCH_EXTENSION_LIMIT,
                        MIN_EVAL,
                        -eval,
                    );

                    if !self.times_up() {
                        eval = new_eval;
                    }
                }

                if eval > *max_eval.lock().unwrap() {
                    *max_eval.lock().unwrap() = eval;
                }

                *e = eval;
            });

            moves.sort_by_key(|a| -a.1);

            info!("depth {} searched in {:.2}s", i, start.elapsed().as_secs_f32());

            if moves.iter().filter(|a| a.1 == MAX_EVAL).next().is_some() {
                info!("found checkmate");
                break;
            } else if self.times_up() {
                break;
            }
        }

        moves.sort_by_key(|a| -a.1);

        for m in moves.iter() {
            dbg!("{} {}", m.0, m.1);
        }

        moves.first().unwrap().clone()
    }

    fn search_alpha_beta(
        &self,
        current: Board,
        depth: usize,
        ext_depth: usize,
        mut alpha: i32,
        beta: i32,
    ) -> i32 {
        if matches!(current.status(), BoardStatus::Checkmate) {
            return MIN_EVAL;
        } else if matches!(current.status(), BoardStatus::Stalemate) {
            return 0;
        }

        if let Some(t_e) = self.trans_table.lock().unwrap().get(current.get_hash()) {
            if t_e.depth >= depth {
                return t_e.eval;
            }
        }

        if self.times_up() {
            return 0;
        }

        if depth == 0 {
            return self.quiescene_search(current, alpha, beta);
        }

        // let mut max_eval = MIN_EVAL;

        for (i, m) in move_in_order(&current).into_iter().enumerate() {
            let after = current.make_move_new(m);
            let mut ext = 0;
            ext += (after.checkers().0 != 0) as usize;
            ext += m.get_promotion().is_some() as usize;

            let mut next_depth = depth as isize - 1 + ext.min(ext_depth) as isize;
            next_depth -= (i >= REDUCED_SEARCH_DEPTH) as isize;

            let mut eval = -self.search_alpha_beta(
                after,
                next_depth.max(0) as usize,
                ext_depth - ext,
                -beta,
                -alpha
            );

            if self.times_up() {
                return 0;
            }

            if eval > alpha && i >= REDUCED_SEARCH_DEPTH {
                let new_eval = -self.search_alpha_beta(
                    after,
                    next_depth.max(0) as usize + 1,
                    ext_depth - ext,
                    -beta,
                    -alpha
                );

                if !self.times_up() {
                    eval = new_eval;
                }
            }

            // capture bonus
            // if current.color_on(m.get_dest()) == Some(!current.side_to_move()) {
            //     eval += PIECE_VALUE[current.piece_on(m.get_dest()).unwrap().to_index()] / 100;
            // }

            self.trans_table.lock().unwrap().insert(current.get_hash(),
                super::trans_table::TransTableEntry {
                    depth,
                    eval,
                    age: self.age,
                }
            );

            if eval >= beta {
                // return eval;
                return beta;
            // } else if eval > max_eval {
                // max_eval = eval;

                // if eval > alpha {
            } else if eval > alpha {
                alpha = eval;
                // }
            }
        }

        // max_evar
        alpha
    }

    pub fn quiescene_search(
        &self,
        current: Board,
        mut alpha: i32,
        beta: i32,
    ) -> i32 {
        if matches!(current.status(), BoardStatus::Checkmate) {
            return MIN_EVAL;
        } else if matches!(current.status(), BoardStatus::Stalemate) {
            return 0;
        }

        let eval = evaluate(&current);
        // let mut max_eval = eval;

        if eval >= beta {
            // return eval;
            return beta;
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
            // } else if eval > max_eval {
            //     max_eval = eval;

            //     if eval > alpha {
            //         alpha = eval;
            //     }
            // }
            } else if eval > alpha {
                alpha = eval;
            }
        }

        // max_eval
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
