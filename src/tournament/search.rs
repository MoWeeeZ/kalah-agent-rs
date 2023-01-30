use std::sync::{Arc, Mutex};

use crate::kalah::valuation::Valuation;
use crate::{Board, Move, Player};

const VALUATION_FN: fn(&Board) -> Valuation = crate::kalah::valuation::store_diff_valuation;

/*====================================================================================================================*/

pub type SharedMinimaxSearchState = Arc<Mutex<MinimaxSearchState>>;

pub struct MinimaxSearchState {
    pub search_active: bool,

    pub current_best_move: Move,
}

pub fn new_shared_minimax_search_state(search_active: bool, fallback_move: Move) -> SharedMinimaxSearchState {
    Arc::new(Mutex::new(MinimaxSearchState {
        search_active,
        current_best_move: fallback_move,
    }))
}

/*====================================================================================================================*/

struct MinimaxWorker {
    search_state: Arc<Mutex<MinimaxSearchState>>,
}

impl MinimaxWorker {
    pub fn new(search_state: SharedMinimaxSearchState) -> Self {
        MinimaxWorker { search_state }
    }

    fn minimax(&mut self, board: &Board, remaining_depth: u32, alpha: Valuation, beta: Valuation) -> (Move, Valuation) {
        if !self.search_state.lock().unwrap().search_active {
            // search has been ended, search results don't matter anymore, exit thread asap
            return (Move::new(127, Player::White), Valuation::NonTerminal { value: 0 });
        }

        if remaining_depth == 0 || !board.has_legal_move() {
            return (Move::new(127, Player::White), VALUATION_FN(board));
        }

        let mut best_move = Move::new(127, Player::White);
        let mut best_value = Valuation::TerminalBlackWin { plies: 0 };
        let mut alpha = alpha;

        let mut board_after_move = board.clone();

        for house in 0..board.h() {
            let move_ = Move::new(house, Player::White);

            if !board.is_legal_move(move_) {
                continue;
            }

            // let mut board_after_move = board.clone();
            board_after_move.clone_from(board);
            let their_turn = !board_after_move.apply_move(move_);

            let value = if their_turn {
                // opponent move: flip board, alpha, beta to their perspective and flip returned value to ours
                board_after_move.flip_board();
                -self.minimax(&board_after_move, remaining_depth - 1, -beta, -alpha).1
            } else {
                // bonus move: don't decrease depth
                self.minimax(&board_after_move, remaining_depth, alpha, beta).1
            }
            .increase_plies();

            if value >= best_value {
                best_move = move_;
                best_value = value;
            }

            if value > beta {
                // beta cutoff, return early
                break;
            }

            // value either has higher value or both are terminal and value is shorter sequence
            if best_value > alpha {
                alpha = best_value;
            }
        }

        (best_move, best_value)
    }

    pub fn start_search(self, board: Board) {
        use Valuation::{TerminalBlackWin, TerminalWhiteWin};

        let mut me = self;

        let alpha = TerminalBlackWin { plies: 0 };
        let beta = TerminalWhiteWin { plies: 0 };

        for max_depth in 6.. {
            let board = board.clone();
            let (best_move, best_value) = me.minimax(&board, max_depth, alpha, beta);

            if !me.search_state.lock().unwrap().search_active {
                return;
            }

            if let Valuation::TerminalWhiteWin { plies: _ } = best_value {
                {
                    let mut search_state = me.search_state.lock().unwrap();
                    search_state.current_best_move = best_move;
                    search_state.search_active = false;
                }
                return;
            }

            if let TerminalBlackWin { plies: _ } = best_value {
                // all moves are certain losses, pick the one with the most plies and exit
                {
                    let mut search_state = me.search_state.lock().unwrap();
                    search_state.current_best_move = best_move;
                    search_state.search_active = false;
                }
                return;
            }

            me.search_state.lock().unwrap().current_best_move = best_move;
        }

        me.search_state.lock().unwrap().search_active = false;
    }
}

/*====================================================================================================================*/

pub fn minimax_search(board: &Board, search_state: SharedMinimaxSearchState) {
    assert!(
        board.has_legal_move(),
        "Called minimax_search on board with no legal moves"
    );

    let t_handle;

    {
        // let worker_board = board.clone();

        t_handle = std::thread::spawn({
            let board = board.clone();
            move || {
                let worker: MinimaxWorker = MinimaxWorker::new(search_state);
                worker.start_search(board);
            }
        });
    }

    // detach worker thread; will get shut down automatically when search_active gets set to false
    drop(t_handle);
}
