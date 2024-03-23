use chess::*;
use super::config::*;
use super::eval::*;
use rayon::prelude::*;
use std::sync::atomic::*;

const NON_ZERO_WINDOW: usize = 1 << 31;
const PV_NODE: usize = 1 << 30;

macro_rules! eq {
    ($a: expr, $b: expr) => { $a == $b };
    ($a: expr, $b: expr, $($rest: tt)+) => {
        $a == $b && eq!($b, $($rest)+)
    }
}

impl super::Game {
    pub fn search(&mut self) -> (ChessMove, i32) {
        let gen = MoveGen::new_legal(&self.board);
        let mut moves = Vec::with_capacity(gen.len());

        for m in gen {
            let board = self.board.make_move_new(m);
            let eval = super::eval::evaluate(&board);
            moves.push((m, eval));
        }

        // reducing move time if there is a single move left
        if moves.len() == 1 {
            dbg!("only move is {}", moves[0].0);
            return moves[0];
        }

        moves.sort_by_key(|a| -a.1);

        for i in 1..=MAX_SEARCH_DEPTH {
            let max_eval = AtomicI32::new(MIN_EVAL);
            let start = std::time::Instant::now();

            moves.par_iter_mut().enumerate().for_each(|(j, (m, e))| {
                let board = self.board.make_move_new(*m);

                let mut depth = i;
                depth -= (j >= REDUCED_SEARCH_DEPTH) as usize;

                let mut moves = self.moves.clone();
                moves.push(*m);

                let mut eval = -self.alpha_beta_search(
                    board,
                    &mut moves,
                    depth,
                    SEARCH_EXTENSION_LIMIT,
                    MIN_EVAL,
                    MAX_EVAL,
                    // -max_eval.load(Ordering::Relaxed),
                    true,
                    false,
                );

                if self.times_up() {
                    return;
                }

                if j >= REDUCED_SEARCH_DEPTH && eval > max_eval.load(Ordering::Relaxed) {
                    let new_eval = -self.alpha_beta_search(
                        board,
                        &mut moves,
                        depth + 1,
                        SEARCH_EXTENSION_LIMIT,
                        MIN_EVAL,
                        -eval,
                        true,
                        false,
                    );

                    if !self.times_up() {
                        eval = new_eval;
                    }
                }

                if eval > max_eval.load(Ordering::Relaxed) {
                    max_eval.store(eval, Ordering::Relaxed);
                }

                *e = eval;
            });

            moves.sort_by_key(|a| -a.1);

            let elapsed = start.elapsed().as_secs_f32();
            let nodes = self.searched.swap(0, Ordering::Relaxed);
            info!("depth {} searched in {:.2}s (nodes: {}, MN/s: {:.2})", i, elapsed, nodes, (nodes as f32 / 1_000_000.0) / elapsed);

            if moves.iter().any(|a| a.1 == MAX_EVAL) {
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

        *moves.first().unwrap()
    }

    fn alpha_beta_search(
        &self,
        current: Board,
        moves: &mut Vec<ChessMove>, // reuse the same vec to avoid alloc
        depth: usize,
        ext_depth: usize,
        mut alpha: i32,
        beta: i32,
        is_pv: bool,
        zero_window: bool,
    ) -> i32 {
        self.searched.fetch_add(1, Ordering::Relaxed);

        if matches!(current.status(), BoardStatus::Checkmate) {
            return MIN_EVAL;
        } else if matches!(current.status(), BoardStatus::Stalemate) {
            return 0;
        }

        let tt_depth = depth + NON_ZERO_WINDOW * !zero_window as usize;

        if let Some(t_e) = self.trans_table.get(current.get_hash()) {
            if (!is_pv || (alpha < t_e.eval && t_e.eval < beta)) && t_e.depth >= tt_depth {
                return t_e.eval;
            }
        }

        if self.times_up() {
            return 0;
        }

        if depth == 0 {
            return Self::quiescene_search(current, alpha, beta);
        }

        // null move pruning
        if let Some(board) = current.null_move() {
            let eval = -self.zero_window_search(
                board,
                moves,
                depth.saturating_sub(4),
                ext_depth,
                1 - beta,
            );

            if self.times_up() {
                return 0;
            }

            if eval >= beta {
                return eval;
            }
        }

        let mut max_eval = if zero_window { alpha } else { MIN_EVAL };
        let mut alpha_raised = false;

        for (i, m) in self.move_in_order(&current).into_iter().enumerate() {
            let mc = moves.len();
            let eval = if mc < 11 || !(
                eq!(moves.get(mc - 11), moves.get(mc - 7), moves.get(mc - 3)) && // chain of 3 fold
                eq!(moves.get(mc - 10), moves.get(mc - 6), moves.get(mc - 2)) && // detection
                eq!(moves.get(mc - 9), moves.get(mc - 5), moves.get(mc - 1)) &&
                eq!(moves.get(mc - 8), moves.get(mc - 4), Some(&m))
            ) {
                let after = current.make_move_new(m);
                let mut ext = 0;
                ext += (after.checkers().0 != 0) as usize;
                ext += m.get_promotion().is_some() as usize;
                let ext = ext.min(ext_depth);

                moves.push(m);

                let mut do_pvs = |depth: isize| if !zero_window {
                    if !alpha_raised {
                        -self.alpha_beta_search(
                            after,
                            moves,
                            depth.max(0) as usize,
                            ext_depth - ext,
                            -beta,
                            -alpha,
                            is_pv,
                            false,
                        )
                    } else {
                        let eval = -self.zero_window_search(
                            after,
                            moves,
                            depth.max(0) as usize,
                            ext_depth - ext,
                            -alpha,
                        );

                        if max_eval < eval && eval < beta {
                            -self.alpha_beta_search(
                                after,
                                moves,
                                depth.max(0) as usize,
                                ext_depth - ext,
                                -beta,
                                -alpha,
                                true,
                                false,
                            )
                        } else {
                            eval
                        }
                    }
                } else {
                    -self.zero_window_search(
                        after,
                        moves,
                        depth.max(0) as usize,
                        ext_depth - ext,
                        1 - beta,
                    )
                };

                let mut next_depth = depth as isize - 1 + ext as isize;
                next_depth -= (i >= REDUCED_SEARCH_DEPTH) as isize;

                let mut eval = do_pvs(next_depth);

                if self.times_up() {
                    return 0;
                }

                if !zero_window && eval > max_eval && i >= REDUCED_SEARCH_DEPTH {
                    let new_eval = do_pvs(next_depth + 1);

                    if !self.times_up() {
                        eval = new_eval;
                    }
                }

                moves.pop();

                // capture bonus
                // if current.color_on(m.get_dest()) == Some(!current.side_to_move()) {
                //     eval += PIECE_VALUE[current.piece_on(m.get_dest()).unwrap().to_index()] / 100;
                // }

                self.trans_table.insert(current.get_hash(),
                    super::trans_table::TransTableEntry {
                        depth: tt_depth,
                        eval,
                        age: self.age,
                    }
                );

                eval
            } else {
                0
            };

            if eval >= beta {
                return eval;
            } else if !zero_window && eval > max_eval {
                max_eval = eval;

                if eval > alpha {
                    alpha = eval;
                    alpha_raised = true;
                }
            }
        }

        max_eval
    }

    pub fn quiescene_search(
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
            let eval = -Self::quiescene_search(board, -beta, -alpha);

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

    fn zero_window_search(
        &self,
        current: Board,
        moves: &mut Vec<ChessMove>,
        depth: usize,
        ext_depth: usize,
        beta: i32
    ) -> i32 {
        self.alpha_beta_search(current, moves, depth, ext_depth, beta - 1, beta, false, true)
    }

    fn move_in_order(&self, board: &Board) -> Vec<ChessMove> {
        let gen = MoveGen::new_legal(board);
        let mut buf = Vec::with_capacity(gen.len());

        buf.extend(gen);

        buf.sort_by_cached_key(|a| {
            let a = board.make_move_new(*a);

            self.trans_table.get(a.get_hash()).map_or_else(|| evaluate(&a), |a| a.eval)
        });

        buf
    }
}
