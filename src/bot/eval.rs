use chess::*;

pub fn evaluate(board: &Board) -> i32 {
    let white_eval = eval_single(board, Color::White);
    let black_eval = eval_single(board, Color::Black);

    let perspective = if matches!(board.side_to_move(), Color::White) { 1 } else { -1 };

    (white_eval - black_eval) * perspective
}

#[inline(always)]
fn eval_single(board: &Board, color: Color) -> i32 {
    let mut eval = 0;

    let oppo_end_weight = end_game_weight(board, !color);

    eval += piece_value(board, color);
    eval += piece_square_table(board, color, oppo_end_weight);

    // bishop pair bonus
    if (board.color_combined(color) & board.pieces(Piece::Bishop)).popcnt() == 2 {
        eval += PIECE_VALUE[0] / 2;
    }

    eval
}

pub fn piece_value(board: &Board, color: Color) -> i32 {
    let color = board.color_combined(color);
    (color & board.pieces(Piece::Pawn)).popcnt() as i32 * PIECE_VALUE[0]
        + (color & board.pieces(Piece::Knight)).popcnt() as i32 * PIECE_VALUE[1]
        + (color & board.pieces(Piece::Bishop)).popcnt() as i32 * PIECE_VALUE[2]
        + (color & board.pieces(Piece::Rook)).popcnt() as i32 * PIECE_VALUE[3]
        + (color & board.pieces(Piece::Queen)).popcnt() as i32 * PIECE_VALUE[4]
}

pub fn end_game_weight(board: &Board, color: Color) -> f32 {
    let color = board.color_combined(color);
    let value = (color & board.pieces(Piece::Knight)).popcnt() as i32 * PIECE_VALUE[1]
        + (color & board.pieces(Piece::Bishop)).popcnt() as i32 * PIECE_VALUE[2]
        + (color & board.pieces(Piece::Rook)).popcnt() as i32 * PIECE_VALUE[3]
        + (color & board.pieces(Piece::Queen)).popcnt() as i32 * PIECE_VALUE[4];

    // value & formula from coding adventures
    1.0 - (value as f32 / 1650.0).min(1.0)
}

pub fn piece_square_table(board: &Board, color: Color, end_weight: f32) -> i32 {
    let mut value = 0.0;

    let our_pieces = board.color_combined(color);

    for mut square in our_pieces.into_iter() {
        let typ = board.piece_on(square).unwrap();

        // invert square based on perspective
        if matches!(color, Color::Black) {
            square = unsafe {
                Square::new(square.to_int() ^ 63)
            };
        }

        let idx = square.to_index() | (typ.to_index() << 6);

        value += (PIECE_SQUARE_TABLE_MID[idx] as f32).mul_add(
            1.0 - end_weight,
            PIECE_SQUARE_TABLE_END[idx] as f32 * end_weight
        );
    }

    value as i32
}

// bishop value will be +25 centipawn if in pair
pub const PIECE_VALUE: [i32; 6] = [100, 305, 333, 563, 950, 20000];

pub const MIN_EVAL: i32 = i32::MIN / 2;
pub const MAX_EVAL: i32 = -MIN_EVAL;

// a1 ----> h1
// |
// v
// a8
//
// value mostly from https://www.chessprogramming.org/Simplified_Evaluation_Function
// pawn endgame was added by me
const PIECE_SQUARE_TABLE_MID: [i32; 64 * 6] = [
    // Pawn
      0,   0,   0,   0,   0,   0,   0,   0,
      5,  10,  10, -20, -20,  10,  10,   5,
      5,  -5, -10,   0,   0, -10,  -5,   5,
      0,   0,   0,  20,  20,   0,   0,   0,
      5,   5,  10,  25,  25,  10,   5,   5,
     10,  10,  20,  30,  30,  20,  10,  10,
     50,  50,  50,  50,  50,  50,  50,  50,
      0,   0,   0,   0,   0,   0,   0,   0,
    // Knight
    -50, -40, -30, -30, -30, -30, -40, -50,
    -40, -20,   0,   5,   5,   0, -20, -40,
    -30,   5,  10,  15,  15,  10,   5, -30,
    -30,   0,  15,  20,  20,  15,   0, -30,
    -30,   5,  15,  20,  20,  15,   5, -30,
    -30,   0,  10,  15,  15,  10,   0, -30,
    -40, -20,   0,   0,   0,   0, -20, -40,
    -50, -40, -30, -30, -30, -30, -40, -50,
    // Bishop
    -20, -10, -10, -10, -10, -10, -10, -20,
    -10,   5,   0,   0,   0,   0,   5, -10,
    -10,  10,  10,  10,  10,  10,  10, -10,
    -10,   0,  10,  10,  10,  10,   0, -10,
    -10,   5,   5,  10,  10,   5,   5, -10,
    -10,   0,   5,  10,  10,   5,   0, -10,
    -10,   0,   0,   0,   0,   0,   0, -10,
    -20, -10, -10, -10, -10, -10, -10, -20,
    // Rook
      0,   0,   0,   5,   5,   0,   0,   0,
     -5,   0,   0,   0,   0,   0,   0,  -5,
     -5,   0,   0,   0,   0,   0,   0,  -5,
     -5,   0,   0,   0,   0,   0,   0,  -5,
     -5,   0,   0,   0,   0,   0,   0,  -5,
     -5,   0,   0,   0,   0,   0,   0,  -5,
      5,  10,  10,  10,  10,  10,  10,   5,
      0,   0,   0,   0,   0,   0,   0,   0,
    // Queen
    -20, -10, -10,  -5,  -5, -10, -10, -20,
    -10,   0,   5,   0,   0,   0,   0, -10,
    -10,   5,   5,   5,   5,   5,   0, -10,
      0,   0,   5,   5,   5,   5,   0,  -5,
     -5,   0,   5,   5,   5,   5,   0,  -5,
    -10,   0,   5,   5,   5,   5,   0, -10,
    -10,   0,   0,   0,   0,   0,   0, -10,
    -20, -10, -10,  -5,  -5, -10, -10, -20,
    // King
     20,  30,  10,   0,   0,  10,  30,  20,
     20,  20,   0,   0,   0,   0,  20,  20,
    -10, -20, -20, -20, -20, -20, -20, -10,
    -20, -30, -30, -40, -40, -30, -30, -20,
    -30, -40, -40, -50, -50, -40, -40, -30,
    -30, -40, -40, -50, -50, -40, -40, -30,
    -30, -40, -40, -50, -50, -40, -40, -30,
    -30, -40, -40, -50, -50, -40, -40, -30,
];

const PIECE_SQUARE_TABLE_END: [i32; 64 * 6] = [
    // Pawn
      0,   0,   0,   0,   0,   0,   0,   0,
    -20, -20, -20, -20, -20, -20, -20, -20,
    -10, -10, -10, -10, -10, -10, -10, -10,
     20,  20,  20,  20,  20,  20,  20,  20,
     30,  30,  30,  30,  30,  30,  30,  30,
     40,  40,  40,  40,  40,  40,  40,  40,
     50,  50,  50,  50,  50,  50,  50,  50,
      0,   0,   0,   0,   0,   0,   0,   0,
    // Knight
    -50, -40, -30, -30, -30, -30, -40, -50,
    -40, -20,   0,   5,   5,   0, -20, -40,
    -30,   5,  10,  15,  15,  10,   5, -30,
    -30,   0,  15,  20,  20,  15,   0, -30,
    -30,   5,  15,  20,  20,  15,   5, -30,
    -30,   0,  10,  15,  15,  10,   0, -30,
    -40, -20,   0,   0,   0,   0, -20, -40,
    -50, -40, -30, -30, -30, -30, -40, -50,
    // Bishop
    -20, -10, -10, -10, -10, -10, -10, -20,
    -10,   5,   0,   0,   0,   0,   5, -10,
    -10,  10,  10,  10,  10,  10,  10, -10,
    -10,   0,  10,  10,  10,  10,   0, -10,
    -10,   5,   5,  10,  10,   5,   5, -10,
    -10,   0,   5,  10,  10,   5,   0, -10,
    -10,   0,   0,   0,   0,   0,   0, -10,
    -20, -10, -10, -10, -10, -10, -10, -20,
    // Rook
      0,   0,   0,   5,   5,   0,   0,   0,
     -5,   0,   0,   0,   0,   0,   0,  -5,
     -5,   0,   0,   0,   0,   0,   0,  -5,
     -5,   0,   0,   0,   0,   0,   0,  -5,
     -5,   0,   0,   0,   0,   0,   0,  -5,
     -5,   0,   0,   0,   0,   0,   0,  -5,
      5,  10,  10,  10,  10,  10,  10,   5,
      0,   0,   0,   0,   0,   0,   0,   0,
    // Queen
    -20, -10, -10,  -5,  -5, -10, -10, -20,
    -10,   0,   5,   0,   0,   0,   0, -10,
    -10,   5,   5,   5,   5,   5,   0, -10,
      0,   0,   5,   5,   5,   5,   0,  -5,
     -5,   0,   5,   5,   5,   5,   0,  -5,
    -10,   0,   5,   5,   5,   5,   0, -10,
    -10,   0,   0,   0,   0,   0,   0, -10,
    -20, -10, -10,  -5,  -5, -10, -10, -20,
    // King
    -50, -30, -30, -30, -30, -30, -30, -50,
    -30, -30,   0,   0,   0,   0, -30, -30,
    -30, -10,  20,  30,  30,  20, -10, -30,
    -30, -10,  30,  40,  40,  30, -10, -30,
    -30, -10,  30,  40,  40,  30, -10, -30,
    -30, -10,  20,  30,  30,  20, -10, -30,
    -30, -20, -10,   0,   0, -10, -20, -30,
    -50, -40, -30, -20, -20, -30, -40, -50,
];
