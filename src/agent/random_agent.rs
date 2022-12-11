use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::agent::Agent;
use crate::{Board, Move, Player};

pub struct RandomAgent {
    board: Board,
}

impl RandomAgent {
    #[allow(dead_code)]
    pub fn new(h: u8, s: u16) -> Self {
        RandomAgent {
            board: Board::new(h, s),
        }
    }
}

impl Agent for RandomAgent {
    fn inform_move(&mut self, move_: Move) {
        self.board.apply_move(move_);
    }

    fn get_move(&mut self) -> Move {
        *self.board.legal_moves(Player::White).choose(&mut thread_rng()).unwrap()
    }
}
