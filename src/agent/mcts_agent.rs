use crate::mcts::Search;
use crate::Agent;
use crate::Move;

pub struct MctsAgent {
    search: Search,
}

impl MctsAgent {}

impl Agent for MctsAgent {
    fn inform_move(&mut self, move_: Move) {
        self.search.inform_move(move_);
    }

    fn get_move(&mut self) -> Move {
        self.search.current_best_move()
    }
}
