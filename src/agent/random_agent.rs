use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::agent::{Agent, AgentState};
use crate::{Board, Move, Player};

pub struct RandomAgent {
    state: AgentState,

    board: Board,
}

impl RandomAgent {
    #[allow(dead_code)]
    pub fn new(h: u8, s: u16) -> Self {
        RandomAgent {
            state: AgentState::Waiting,
            board: Board::new(h, s),
        }
    }
}

impl Agent for RandomAgent {
    fn update_board(&mut self, board: &Board) {
        self.board = board.clone();
    }

    fn get_current_best_move(&mut self) -> Move {
        assert_eq!(self.state, AgentState::Go);

        self.state = AgentState::Waiting;

        *self.board.legal_moves(Player::White).choose(&mut thread_rng()).unwrap()
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
