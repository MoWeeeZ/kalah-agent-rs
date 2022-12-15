use std::time::Duration;

use crate::kalah::ValuationFn;
use crate::Board;

use super::search::minimax_search;
use crate::agent::Agent;

pub struct MinimaxAgent {
    board: Board,
    thinking_dur: Duration,
    alpha_beta_prune: bool,

    valuation_fn: ValuationFn,
}

impl MinimaxAgent {
    #[allow(dead_code)]
    pub fn new(h: u8, s: u16, thinking_dur: Duration, alpha_beta_prune: bool, valuation_fn: ValuationFn) -> Self {
        MinimaxAgent {
            board: Board::new(h, s),
            thinking_dur,
            alpha_beta_prune,
            valuation_fn,
        }
    }
}

impl Agent for MinimaxAgent {
    fn inform_move(&mut self, move_: crate::Move) {
        self.board.apply_move(move_);
    }

    fn get_move(&mut self) -> crate::Move {
        minimax_search(&self.board, self.valuation_fn, self.thinking_dur, self.alpha_beta_prune)
    }
}
