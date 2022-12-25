use crate::agent::{Agent, AgentState};
use crate::{Board, Move, Player};

/// agent that always picks the first available move
/// useful for performance tests since, unlike RandomAgent, it's deterministic
pub struct FirstMoveAgent {
    state: AgentState,

    board: Board,
}

impl FirstMoveAgent {
    #[allow(dead_code)]
    pub fn new(h: u8, s: u16) -> Self {
        FirstMoveAgent {
            state: AgentState::Waiting,
            board: Board::new(h, s),
        }
    }
}

impl Agent for FirstMoveAgent {
    fn update_board(&mut self, board: &Board) {
        self.board = board.clone();
    }

    fn get_current_best_move(&mut self) -> Move {
        assert_eq!(self.state, AgentState::Go);

        *self.board.legal_moves(Player::White).first().unwrap()
    }

    fn get_state(&self) -> AgentState {
        self.state
    }

    fn go(&mut self) {
        self.state = AgentState::Go;
    }

    fn stop(&mut self) {
        self.state = AgentState::Waiting;
    }

    fn ponder(&mut self) {
        self.state = AgentState::Ponder;
    }
}
