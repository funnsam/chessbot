use chess::*;
use super::config::*;
use super::eval::*;
use rayon::prelude::*;
use std::sync::atomic::*;

const NON_ZERO_WINDOW: usize = 1 << 31;

impl super::Game {
    pub fn search(&mut self) -> (ChessMove, i32) {
        let gen = MoveGen::new_legal(&self.board);
        let mut moves = Vec::with_capacity(gen.len());

        for m in gen {
            let board = self.board.make_move_new(m);
            let eval = super::eval::evaluate(&board);
            moves.push((m, eval));
        }

        if moves.len() == 1 {
            dbg!("only move is {}", moves[0].0);
            return moves[0];
        }

        moves.sort_by_key(|a| -a.1);

        for i in 1..=MAX_SEARCH_DEPTH {
            let max_eval = AtomicI32::new(MIN_EVAL);
            let start = std::time::Instant::now();

            moves.par_iter_mut().enumerate().for_each(|(j, (m, e))| {
                let mut moves = self.moves.clone();
                moves.push(*m);

                if three_fold(self.init_board.clone(), &moves) {
                    *e = 0;
                    return;
                }

                let board = self.board.make_move_new(*m);

                let mut depth = i;
                depth -= (j >= REDUCED_SEARCH_DEPTH) as usize;

                let mut eval = -self.alpha_beta_search(
                    board,
                    &mut moves,
                    depth,
                    SEARCH_EXTENSION_LIMIT,
                    MIN_EVAL,
                    MAX_EVAL,
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

                moves.pop();
                debug_assert_eq!(moves, self.moves);

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


        self.searched.fetch_add(1, Ordering::Relaxed);

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
        let mut found_fail_high = false;

        moves.push(ChessMove::default());
        let moves = MoveRemover(moves);
        struct MoveRemover<'a>(&'a mut Vec<ChessMove>);
        impl<'a> Drop for MoveRemover<'a> {
            fn drop(&mut self) {
                self.0.pop();
            }
        }

        for (i, m) in self.move_in_order(&current).into_iter().enumerate() {
            *moves.0.last_mut().unwrap() = m;
            let after = current.make_move_new(m);

            let mut eval = || {
                let mut ext = 0;

                let checks = after.checkers().0 != 0;
                let promotes = m.get_promotion().is_some();

                ext += checks as usize;
                ext += promotes as usize;
                let ext = ext.min(ext_depth);

                let mut eval = |depth: isize| if !zero_window {
                    if !alpha_raised {
                        -self.alpha_beta_search(
                            after,
                            moves.0,
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
                            moves.0,
                            depth.max(0) as usize,
                            ext_depth - ext,
                            -alpha,
                        );

                        if max_eval < eval && eval < beta {
                            -self.alpha_beta_search(
                                after,
                                moves.0,
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
                        moves.0,
                        depth.max(0) as usize,
                        ext_depth - ext,
                        1 - beta,
                    )
                };
                let mut do_pvs = |depth: isize| if i < 3 || found_fail_high || checks || promotes {
                    eval(depth)
                } else {
                    eval((2 * depth / 3).min(depth - 1))
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

                    if new_eval <= -beta {
                        found_fail_high = true;
                    }
                }

                self.trans_table.insert(after.get_hash(),
                    super::trans_table::TransTableEntry {
                        depth: tt_depth,
                        eval,
                        age: self.age,
                    }
                );

                eval
            };

            let eval = eval();

            if self.times_up() {
                return 0;
            }

            if eval >= beta {
                return eval;
            } else if eval > max_eval {
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

fn three_fold(mut b: Board, m: &[ChessMove]) -> bool {
    let mut cl = [ChessMove::default(); 24];
    let mut c = 0;

    let mut revbl = Vec::with_capacity(m.len());

    for m2 in m.iter() {
        let rev = !(matches!(b.piece_on(m2.get_source()), Some(Piece::Pawn)) || b.piece_on(m2.get_dest()).is_some());
        let crw = b.castle_rights(Color::White);
        let crb = b.castle_rights(Color::Black);

        b = b.make_move_new(*m2);

        revbl.push(rev && b.castle_rights(Color::White) == crw && b.castle_rights(Color::Black) == crb);
    }

    'a: for (m, rev) in m.iter().rev().zip(revbl.iter().rev()) {
        if *rev {
            for cm in cl.iter_mut() {
                if m.get_dest() == cm.get_source() {
                    if m.get_source() == cm.get_dest() {
                        c -= 1;
                        if c == 0 { return true; }
                        *cm = ChessMove::default();
                        continue 'a;
                    }

                    *cm = ChessMove::new(m.get_source(), cm.get_dest(), None);
                    continue 'a;
                }
            }

            for cm in cl.iter_mut() {
                if *cm == ChessMove::default() {
                    *cm = *m;
                    c += 1;
                    continue 'a;
                }
            }
        }
    }

    false
}
