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
            moves.push((m, eval, vec![]));
        }

        moves.sort_by_key(|a| -a.1);

        for m in moves.iter() {
            dbg!("{} {}", m.0, m.1);
        }

        for i in 1..=MAX_SEARCH_DEPTH {
            let start = std::time::Instant::now();

            moves.par_iter_mut().enumerate().for_each(|(j, (m, e, best_moves))| {
                let board = self.lichess.board.make_move_new(*m);

                let mut depth = i;
                depth -= (j >= REDUCED_SEARCH_DEPTH) as usize;

                let (mut eval, mut moves) = self.search_alpha_beta(
                    board,
                    depth,
                    SEARCH_EXTENSION_LIMIT,
                    MIN_EVAL,
                    -*max_eval.lock().unwrap(),
                );

                eval = -eval;

                if self.times_up() {
                    return;
                }

                if eval > *max_eval.lock().unwrap() && j >= REDUCED_SEARCH_DEPTH {
                    let (new_eval, new_moves) = self.search_alpha_beta(
                        board,
                        depth + 1,
                        SEARCH_EXTENSION_LIMIT,
                        MIN_EVAL,
                        -eval,
                    );

                    if !self.times_up() {
                        eval = -new_eval;
                        moves = new_moves;
                    }
                }

                if eval > *max_eval.lock().unwrap() {
                    *max_eval.lock().unwrap() = eval;
                }

                *e = eval;
                moves.push(*m);
                *best_moves = moves;
            });

            moves.sort_by_key(|a| -a.1);

            info!("depth {} searched in {:.2}s", i, start.elapsed().as_secs_f32());

            if moves.iter().filter(|a| a.1 == MAX_EVAL).next().is_some() {
                info!("found mate");
                break;
            } else if self.times_up() {
                break;
            }
        }

        moves.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        for m in moves.iter() {
            dbg!(
                "{} {} ({}...)",
                m.0,
                m.1,
                m.2.iter()
                    .rev()
                    .map(|a| format!("{a} "))
                    .collect::<Vec<String>>()
                    .join("")
            );
        }

        let m = moves.first().unwrap().clone();
        (m.0, m.1)
    }

    fn search_alpha_beta(
        &self,
        current: Board,
        depth: usize,
        ext_depth: usize,
        mut alpha: i32,
        beta: i32,
    ) -> (i32, Vec<ChessMove>) {
        if matches!(current.status(), BoardStatus::Checkmate) {
            return (MIN_EVAL, vec![]);
        } else if matches!(current.status(), BoardStatus::Stalemate) {
            return (0, vec![]);
        }

        if let Some(t_e) = self.trans_table.lock().unwrap().get(current.get_hash()) {
            if t_e.depth >= depth {
                return (t_e.eval, vec![]);
            }
        }

        if self.times_up() {
            return (0, vec![]);
        }

        if depth == 0 {
            return (self.quiescene_search(current, alpha, beta), vec![]);
        }

        let mut max_eval = MIN_EVAL;
        let mut best_moves = vec![];

        for (i, m) in move_in_order(&current).into_iter().enumerate() {
            let after = current.make_move_new(m);
            let mut ext = 0;
            ext += (after.checkers().0 != 0) as usize;
            ext += m.get_promotion().is_some() as usize;

            let mut next_depth = depth as isize - 1 + ext.min(ext_depth) as isize;
            next_depth -= (i >= REDUCED_SEARCH_DEPTH) as isize;

            let (mut eval, mut moves) = self.search_alpha_beta(
                after,
                next_depth.max(0) as usize,
                ext_depth - ext,
                -beta,
                -alpha
            );

            eval = -eval;

            if self.times_up() {
                return (0, vec![]);
            }

            if eval > max_eval && i >= REDUCED_SEARCH_DEPTH {
                (eval, moves) = self.search_alpha_beta(
                    after,
                    next_depth.max(0) as usize + 1,
                    ext_depth - ext,
                    -beta,
                    -alpha
                );

                eval = -eval;
            }

            moves.push(m);

            if self.times_up() {
                return (0, moves);
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
                return (eval, moves);
            } else if eval > max_eval {
                max_eval = eval;
                best_moves = moves;

                if eval > alpha {
                    alpha = eval;
                }
            }
        }

        (max_eval, best_moves)
    }

    pub fn quiescene_search(
        &self,
        current: Board,
        mut alpha: i32,
        beta: i32,
    ) -> i32 {
        let eval = evaluate(&current);
        let mut max_eval = eval;

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
            } else if eval > max_eval {
                max_eval = eval;

                if eval > alpha {
                    alpha = eval;
                }
            }
        }

        max_eval
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
