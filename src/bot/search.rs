use crate::lichess::LichessGame;
use chess::*;

pub fn search(game: &LichessGame) -> ChessMove {
    let mut max_eval = i32::MIN;
    let mut max_move = None;

    for m in MoveGen::new_legal(&game.board) {
        // dbg!("search move {}", m);
        let board = game.board.make_move_new(m);
        let eval = -search_alpha_beta(game, board, 2, i32::MIN + 1, i32::MAX);
        // dbg!("move {} eval {}", m, eval);

        if eval >= max_eval {
            max_eval = eval;
            max_move = Some(m);
        }
    }

        // dbg!("best move is {} eval {}", max_move.unwrap(), max_eval);

    max_move.unwrap()
}

fn search_alpha_beta(
    game: &LichessGame,
    current: Board,
    depth: usize,
    mut alpha: i32,
    beta: i32,
) -> i32 {
    if depth == 0 {
        return super::eval::evaluate(&current);
    }

    let movegen = MoveGen::new_legal(&current);

    for m in movegen {
        let after = current.make_move_new(m);
        // dbg!("search move {} depth {}", m, depth);
        let eval = -search_alpha_beta(game, after, depth - 1, -beta, -alpha);
        // dbg!("eval {}", eval);

        if eval >= beta {
            // dbg!("prune branch because of {}", m);
            return beta;
        } else if eval > alpha {
            alpha = eval;
        }
    }

    return alpha;
}
