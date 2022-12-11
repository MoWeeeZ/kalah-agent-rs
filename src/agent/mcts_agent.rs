use crate::mcts::Search;
use crate::{Agent, Board, Move};

pub struct MctsAgent {
    search: Search,
}

impl MctsAgent {
    #[allow(dead_code)]
    pub fn new(h: u8, s: u16, num_threads: u64) -> Self {
        let board = Board::new(h, s);
        let mut search = Search::new(board);
        search.start_threads(num_threads);

        MctsAgent { search }
    }
}

impl Agent for MctsAgent {
    fn inform_move(&mut self, move_: Move) {
        self.search.inform_move(move_);
    }

    fn get_move(&mut self) -> Move {
        self.search.current_best_move()
    }
}
