use chess::*;

impl super::Game {
    pub fn search(&mut self) -> ChessMove {
        let mut max_eval = i32::MIN;
        let mut max_move = None;

        for m in MoveGen::new_legal(&self.lichess.board) {
            let board = self.lichess.board.make_move_new(m);
            let eval = -self.search_alpha_beta(board, 4, i32::MIN + 1, i32::MAX);

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
            return super::eval::evaluate(&current);
        }

        let movegen = MoveGen::new_legal(&current);

        for m in movegen {
            let after = current.make_move_new(m);
            let eval = -self.search_alpha_beta(after, depth - 1, -beta, -alpha);

            if eval >= beta {
                self.trans_table.insert(current.get_hash(), super::trans_table::TransTableEntry {
                    depth,
                    eval: beta,
                });
                return beta;
            } else if eval > alpha {
                alpha = eval;
            }
        }

        self.trans_table.insert(current.get_hash(), super::trans_table::TransTableEntry {
            depth,
            eval: alpha,
        });
        return alpha;
    }
}
