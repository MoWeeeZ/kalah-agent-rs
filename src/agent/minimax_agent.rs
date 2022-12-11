use std::time::Duration;

use crate::Board;

use crate::agent::Agent;
use crate::minimax::minimax_search;

pub struct MinimaxAgent {
    board: Board,
    thinking_dur: Duration,
}

impl MinimaxAgent {
    #[allow(dead_code)]
    pub fn new(h: u8, s: u16, thinking_dur: Duration) -> Self {
        MinimaxAgent {
            board: Board::new(h, s),
            thinking_dur,
        }
    }
}

impl Agent for MinimaxAgent {
    fn inform_move(&mut self, move_: crate::Move) {
        self.board.apply_move(move_);
    }

    fn get_move(&mut self) -> crate::Move {
        minimax_search(&self.board, self.thinking_dur)
    }
}
