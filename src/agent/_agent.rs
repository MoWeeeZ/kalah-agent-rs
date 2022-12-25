use crate::{Board, Move};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AgentState {
    Waiting, // agent doing nothing, waiting for go or ponder
    Go,      // agent calculating next move
    Ponder,  // agent pondering next move while waiting for opponent
}

pub trait Agent {
    fn update_board(&mut self, board: &Board);
    fn get_current_best_move(&mut self) -> Move;

    fn get_state(&self) -> AgentState;
    fn go(&mut self);
    fn stop(&mut self);
    fn ponder(&mut self);
}
