use crate::Board;

use crate::agent::Agent;
use crate::minimax::minimax_search;

pub struct MinimaxAgent {
    board: Board,
    max_depth: u64,
}

impl MinimaxAgent {
    pub fn new(h: u8, s: u16, max_depth: u64) -> Self {
        MinimaxAgent {
            board: Board::new(h, s),
            max_depth,
        }
    }
}

impl Agent for MinimaxAgent {
    fn inform_move(&mut self, move_: crate::Move) {
        self.board.apply_move(move_);
    }

    fn get_move(&mut self) -> crate::Move {
        minimax_search(&self.board, self.max_depth)
    }
}
