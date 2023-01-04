use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::kalah::valuation::{Valuation, ValuationFn};
use crate::{Board, Move, Player};

const LOG_STATS: bool = false;

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
    max_depth: u32,

    search_state: Arc<Mutex<MinimaxSearchState>>,

    valuation_fn: ValuationFn,

    total_nodes_visited: u64,

    start_t: Instant,
}

impl MinimaxWorker {
    pub fn new(max_depth: u32, valuation_fn: ValuationFn, search_state: SharedMinimaxSearchState) -> Self {
        MinimaxWorker {
            max_depth,
            search_state,
            valuation_fn,
            total_nodes_visited: 0,
            start_t: Instant::now(),
        }
    }

    fn current_nps(&self) -> f64 {
        self.total_nodes_visited as f64 / self.start_t.elapsed().as_secs_f64()
    }

    fn minimise(&mut self, board: Board, remaining_depth: u32, alpha: Valuation, beta: Valuation) -> (Move, Valuation) {
        use Player::Black;

        if !self.search_state.lock().unwrap().search_active {
            // search has been ended, search results don't matter anymore, exit thread asap
            panic!("Could not complete minimax search to level 6");
        }

        if remaining_depth == 0 || !board.has_legal_move() {
            return (Move::new(127, Black), (self.valuation_fn)(&board));
        }

        self.total_nodes_visited += 1;

        let mut best_value = Valuation::TerminalWhiteWin { plies: 0 };
        let mut best_move = Move::new(127, Black);

        let mut beta = beta;

        for move_ in board.legal_moves(Black) {
            let mut board_after_move = board.clone();
            let their_turn = !board_after_move.apply_move(move_);

            let (_, value) = if their_turn {
                self.maximise(board_after_move, remaining_depth - 1, alpha, beta)
            } else {
                self.minimise(board_after_move, remaining_depth, alpha, beta)
            };
            let value = value.increase_plies();

            if value < best_value {
                best_value = value;
                best_move = move_;
            }
            if best_value <= alpha {
                break;
            }
            if best_value < beta {
                beta = best_value;
            }
        }

        (best_move, best_value)
    }

    fn maximise(&mut self, board: Board, remaining_depth: u32, alpha: Valuation, beta: Valuation) -> (Move, Valuation) {
        use Player::White;

        if !self.search_state.lock().unwrap().search_active {
            // search has been ended, search results don't matter anymore, exit thread asap
            panic!("Could not complete minimax search to level 6");
        }

        if remaining_depth == 0 || !board.has_legal_move() {
            return (Move::new(127, White), (self.valuation_fn)(&board));
        }

        self.total_nodes_visited += 1;

        let mut best_value = Valuation::TerminalBlackWin { plies: 0 };
        let mut best_move = Move::new(127, White);

        let mut alpha = alpha;

        for move_ in board.legal_moves(White) {
            let mut board_after_move = board.clone();
            let their_turn = !board_after_move.apply_move(move_);

            let (_, value) = if their_turn {
                self.minimise(board_after_move, remaining_depth - 1, alpha, beta)
            } else {
                self.maximise(board_after_move, remaining_depth, alpha, beta)
            };
            let value = value.increase_plies();

            if value > best_value {
                best_move = move_;
                best_value = value;
            }
            if best_value >= beta {
                break;
            }
            if best_value > alpha {
                alpha = best_value;
            }
        }

        (best_move, best_value)
    }

    pub fn minimax_search(self, board: Board) {
        use Valuation::{TerminalBlackWin, TerminalWhiteWin};

        let mut me = self;

        me.start_t = std::time::Instant::now();

        let alpha = TerminalBlackWin { plies: 0 };
        let beta = TerminalWhiteWin { plies: 0 };

        let (best_move, best_value) = me.maximise(board, me.max_depth, alpha, beta);

        me.search_state.lock().unwrap().current_best_move = best_move;
        me.search_state.lock().unwrap().search_active = false;

        if LOG_STATS {
            println!("--------------------------------------------");
            println!("* Minimax reference worker exited after max_depth {}", me.max_depth);
            println!("* Best move {} had value {:?}", best_move, best_value);
            println!("* NPS: {:.2e} ({:?})", me.current_nps(), me.start_t.elapsed());
            println!("--------------------------------------------\n");
        }
    }
}

/*====================================================================================================================*/

pub fn start_search(board: &Board, depth: u32, valuation_fn: ValuationFn, search_state: SharedMinimaxSearchState) {
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
                let worker: MinimaxWorker = MinimaxWorker::new(depth, valuation_fn, search_state);
                worker.minimax_search(board);
            }
        });
    }

    // detach worker thread; will get shut down automatically when search_active gets set to false
    drop(t_handle);
}
