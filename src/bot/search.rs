use chess::*;

impl super::Game {
    pub fn search(&mut self) -> ChessMove {
        let mut max_eval = i32::MIN;
        let mut max_move = None;

        for (i, m) in move_in_order(&self.lichess.board).into_iter().enumerate() {
            let board = self.lichess.board.make_move_new(m);

            let mut depth = super::config::MAX_SEARCH_DEPTH;

            let reduced = i >= super::config::REDUCED_DEPTH_THRESHOLD;
            depth -= reduced as usize;

            let mut eval = -self.search_alpha_beta(board, depth, i32::MIN + 1, i32::MAX);

            if eval >= max_eval && reduced {
                eval = -self.search_alpha_beta(board, depth + 1, i32::MIN + 1, i32::MAX);
            }

            if eval >= max_eval {
                max_eval = eval;
                max_move = Some(m);
            }
        }

        max_move.unwrap()
    }

    fn search_alpha_beta(
        &mut self,
        current: Board,
        depth: usize,
        mut alpha: i32,
        beta: i32,
    ) -> i32 {
        if let Some(t_e) = self.trans_table.get(&current.get_hash()) {
            if t_e.depth >= depth {
                return t_e.eval;
            }
        }

        if depth == 0 {
            let eval = self.quiescene_search(current, alpha, beta);

            return eval;
        }

        for m in move_in_order(&current).into_iter() {
            let after = current.make_move_new(m);
            let eval = -self.search_alpha_beta(after, depth - 1, -beta, -alpha);

            self.trans_table.insert(current.get_hash(), super::trans_table::TransTableEntry {
                depth,
                eval,
                age: self.age,
            });

            if eval >= beta {
                return beta;
            } else if eval > alpha {
                alpha = eval;
            }
        }

        alpha
    }

    pub fn quiescene_search(
        &mut self,
        current: Board,
        mut alpha: i32,
        beta: i32,
    ) -> i32 {
        let eval = super::eval::evaluate(&current);

        if eval >= beta {
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
                return beta;
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

    buf.sort_by_key(|m| {
        let board = board.make_move_new(*m);
        super::eval::evaluate(&board)
    });

    buf
}
