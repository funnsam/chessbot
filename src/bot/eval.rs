use crate::lichess::LichessGame;
use chess::*;

pub fn evaluate(game: &LichessGame) -> i32 {
    let white_piece_value = piece_value(game, Color::White) as i32;
    let black_piece_value = piece_value(game, Color::Black) as i32;

    let perspective = if game.color == Color::White { 1 } else { -1 };

    (white_piece_value - black_piece_value) * perspective
}

pub fn piece_value(game: &LichessGame, color: Color) -> u32 {
    let color = game.board.color_combined(color);
    (color & game.board.pieces(Piece::Pawn)).0.count_ones() * 100
    + (color & game.board.pieces(Piece::Knight)).0.count_ones() * 320
    + (color & game.board.pieces(Piece::Bishop)).0.count_ones() * 330
    + (color & game.board.pieces(Piece::Rook)).0.count_ones() * 500
    + (color & game.board.pieces(Piece::Queen)).0.count_ones() * 900
    + (color & game.board.pieces(Piece::King)).0.count_ones() * 20000
}

// a1 ----> a8
// |
// v
// h1
const PIECE_SQUARE_TABLE: &[i32] = &[
    // pawn
      0,   0,   0,   0,   0,   0,   0,   0,
      5,  10,  10, -20, -20,  10,  10,   5,
      5,  -5, -10,   0,   0, -10,  -5,   5,
      0,   0,   0,  20,  20,   0,   0,   0,
      5,   5,  10,  25,  25,  10,   5,   5,
     10,  10,  20,  30,  30,  20,  10,  10,
     50,  50,  50,  50,  50,  50,  50,  50,
      0,   0,   0,   0,   0,   0,   0,   0,
    // knight
    -50, -40, -30, -30, -30, -30, -40, -50,
    -40, -20,   0,   0,   0,   0, -20, -40,
    -30,   0,  10,  15,  15,  10,   0, -30,
    -30,   5,  15,  20,  20,  15,   5, -30,
    -30,   0,  15,  20,  20,  15,   0, -30,
    -30,   5,  10,  15,  15,  10,   5, -30,
    -40, -20,   0,   5,   5,   0, -20, -40,
    -50, -40, -30, -30, -30, -30, -40, -50,
    // bishop
    -20, -10, -10, -10, -10, -10, -10, -20,
    -10,   0,   0,   0,   0,   0,   0, -10,
    -10,   0,   5,  10,  10,   5,   0, -10,
    -10,   5,   5,  10,  10,   5,   5, -10,
    -10,   0,  10,  10,  10,  10,   0, -10,
    -10,  10,  10,  10,  10,  10,  10, -10,
    -10,   5,   0,   0,   0,   0,   5, -10,
    -20, -10, -10, -10, -10, -10, -10, -20,
];
