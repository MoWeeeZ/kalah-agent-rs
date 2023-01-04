use crate::agent::RandomAgent;
use crate::{single_ply, Board, Player};

pub mod math;

pub fn advance_random(h: u8, s: u16, board: &mut Board, num_moves: usize) {
    let mut current_player = Player::White;
    let mut random_agent = RandomAgent::new(h, s);

    // make 10 random moves
    for _ in 0..num_moves {
        use Player::{Black, White};

        current_player = match current_player {
            White => single_ply(board, &mut random_agent, White, false),
            Black => single_ply(board, &mut random_agent, Black, false),
        };

        if !board.has_legal_move() {
            break;
        }
    }
}
