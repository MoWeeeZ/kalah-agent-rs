use std::sync::Arc;

use crate::kalah::ValuationFn;
use crate::{Board, Move, Player};

use super::search::{minimax_search, new_shared_minimax_search_state, SharedMinimaxSearchState};
use crate::agent::{Agent, AgentState};

pub struct MinimaxAgent<const ALPHA_BETA_PRUNE: bool> {
    state: AgentState,

    board: Board,

    search_state: Option<SharedMinimaxSearchState>,

    valuation_fn: ValuationFn,
}

impl<const ALPHA_BETA_PRUNE: bool> MinimaxAgent<ALPHA_BETA_PRUNE> {
    #[allow(dead_code)]
    pub fn new(h: u8, s: u16, valuation_fn: ValuationFn) -> Self {
        MinimaxAgent {
            state: AgentState::Waiting,
            board: Board::new(h, s),
            search_state: None,
            valuation_fn,
        }
    }
}

impl<const ALPHA_BETA_PRUNE: bool> Agent for MinimaxAgent<ALPHA_BETA_PRUNE> {
    fn set_board(&mut self, board: &Board) {
        self.board = board.clone();
    }

    fn get_current_best_move(&mut self) -> Move {
        assert_eq!(self.state, AgentState::Go);

        self.search_state.as_ref().unwrap().lock().unwrap().current_best_move
    }

    fn get_state(&self) -> crate::agent::AgentState {
        self.state
    }

    fn go(&mut self) {
        // use first legal move as a fallback in case we don't complete a single search iteration, which really should
        // not happen
        let fallback_move = *self.board.legal_moves(Player::White).first().unwrap();
        let search_state = new_shared_minimax_search_state(true, fallback_move);

        minimax_search::<ALPHA_BETA_PRUNE>(&self.board, self.valuation_fn, Arc::clone(&search_state));

        self.state = AgentState::Go;
        self.search_state = Some(search_state);
    }

    fn stop(&mut self) {
        self.state = AgentState::Waiting;

        // set search_active to false, then drop reference
        self.search_state.as_ref().unwrap().lock().unwrap().search_active = false;
        self.search_state = None;
    }

    fn ponder(&mut self) {
        // self.state = AgentState::Ponder;
        todo!()
    }
}
