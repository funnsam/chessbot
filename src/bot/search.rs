use crate::lichess::LichessGame;
use chess::*;

pub fn search(game: &LichessGame) -> ChessMove {
    let mut max_eval = i32::MIN;
    let mut max_move = None;

    for m in MoveGen::new_legal(&game.board) {
        let eval = search_alpha_beta(game, game.board.clone(), 3, i32::MIN + 1, i32::MAX);

        if eval >= max_eval {
            max_eval = eval;
            max_move = Some(m);
        }
    }

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
        return super::eval::evaluate(&LichessGame {
            board: current,
            ..game.clone()
        });
    }

    let movegen = MoveGen::new_legal(&current);

    for m in movegen {
        let after = current.make_move_new(m);
        let eval = -search_alpha_beta(game, after, depth - 1, beta, alpha);

        if eval >= beta {
            return beta;
        } else if eval > alpha {
            alpha = eval;
        }
    }

    return alpha;
}
