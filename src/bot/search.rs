use chess::*;
use super::config::*;
use super::eval::*;

impl super::Game {
    pub fn search(&mut self) -> (ChessMove, f32) {
        let mut max_eval = f32::NEG_INFINITY;

        let gen = MoveGen::new_legal(&self.lichess.board);
        let mut moves = Vec::with_capacity(gen.len());

        for m in gen {
            let board = self.lichess.board.make_move_new(m);
            let eval = super::eval::evaluate(&board);
            moves.push((m, eval));
        }

        moves.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        'outer: for i in 1..=MAX_SEARCH_DEPTH {
            let start = std::time::Instant::now();

            for (j, (m, e)) in moves.iter_mut().enumerate() {
                let board = self.lichess.board.make_move_new(*m);

                let mut depth = i;
                depth -= (j <= 5) as usize;

                let mut eval = -self.search_alpha_beta(
                    board,
                    depth,
                    f32::NEG_INFINITY,
                    f32::INFINITY
                );

                if eval > max_eval && j <= 5 {
                    eval = -self.search_alpha_beta(
                        board,
                        depth + 1,
                        f32::NEG_INFINITY,
                        f32::INFINITY
                    );
                }

                if eval >= max_eval {
                    max_eval = eval;
                }

                if self.times_up() {
                    break 'outer;
                }

                *e = eval;
            }

            moves.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

            info!("depth {} searched in {:.2}s", i, start.elapsed().as_secs_f32());
        }

        moves.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        for m in moves.iter(){
            dbg!("{} {}", m.0, m.1);
        }

        moves.first().unwrap().clone()
    }

    fn search_alpha_beta(
        &mut self,
        current: Board,
        depth: usize,
        mut alpha: f32,
        beta: f32,
    ) -> f32 {
        if matches!(current.status(), BoardStatus::Checkmate) {
            return f32::NEG_INFINITY;
        } else if matches!(current.status(), BoardStatus::Stalemate) {
            return 0.0;
        } else if self.times_up() {
            return 0.0;
        }

        if let Some(t_e) = self.trans_table.get(&current.get_hash()) {
            if t_e.depth >= depth {
                return t_e.eval;
            }
        }

        if depth == 0 {
            let eval = self.quiescene_search(current, alpha, beta);

            self.trans_table.insert(current.get_hash(), super::trans_table::TransTableEntry {
                depth,
                eval,
                age: self.age,
            });

            return eval;
        }

        for m in move_in_order(&current).into_iter() {
            let after = current.make_move_new(m);
            let mut eval = -self.search_alpha_beta(after, depth - 1, -beta, -alpha);

            if self.times_up() {
                return 0.0;
            }

            // capture bonus
            // if current.color_on(m.get_dest()) == Some(!current.side_to_move()) {
            //     eval += PIECE_VALUE[current.piece_on(m.get_dest()).unwrap().to_index()] * 50.0;
            //     eval -= PIECE_VALUE[current.piece_on(m.get_source()).unwrap().to_index()];
            // }

            self.trans_table.insert(current.get_hash(), super::trans_table::TransTableEntry {
                depth,
                eval,
                age: self.age,
            });

            if eval >= beta {
                return eval;
            } else if eval > alpha {
                alpha = eval;
            }
        }

        alpha
    }

    pub fn quiescene_search(
        &mut self,
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
