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
    search_state: Arc<Mutex<MinimaxSearchState>>,

    valuation_fn: ValuationFn,

    total_nodes_visited: u64,

    start_t: Instant,
}

impl MinimaxWorker {
    pub fn new(valuation_fn: ValuationFn, search_state: SharedMinimaxSearchState) -> Self {
        MinimaxWorker {
            search_state,
            valuation_fn,
            total_nodes_visited: 0,
            start_t: Instant::now(),
        }
    }

    fn current_nps(&self) -> f64 {
        self.total_nodes_visited as f64 / self.start_t.elapsed().as_secs_f64()
    }

    fn minimax(&mut self, board: Board, remaining_depth: u32, alpha: Valuation, beta: Valuation) -> (Move, Valuation) {
        if !self.search_state.lock().unwrap().search_active {
            // search has been ended, search results don't matter anymore, exit thread asap
            return (Move::new(127, Player::White), Valuation::NonTerminal { value: 0 });
        }

        self.total_nodes_visited += 1;

        if remaining_depth == 0 || !board.has_legal_move() {
            return (Move::new(127, Player::White), (self.valuation_fn)(&board));
        }

        let mut best_move = Move::new(127, Player::White);
        let mut best_value = Valuation::TerminalBlackWin { plies: 0 };
        let mut alpha = alpha;

        for move_ in board.legal_moves(Player::White) {
            let mut board_after_move = board.clone();
            let their_turn = !board_after_move.apply_move(move_);

            let value = if their_turn {
                // opponent move: flip board, alpha, beta to their perspective and flip returned value to ours
                board_after_move.flip_board();
                -self.minimax(board_after_move, remaining_depth - 1, -beta, -alpha).1
            } else {
                // bonus move: don't decrease depth
                self.minimax(board_after_move, remaining_depth, alpha, beta).1
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

        me.start_t = std::time::Instant::now();

        let mut current_best_value = Valuation::TerminalBlackWin { plies: 0 };

        let alpha = TerminalBlackWin { plies: 0 };
        let beta = TerminalWhiteWin { plies: 0 };

        let max_depth = 6;
        // {
        for max_depth in 6.. {
            let board = board.clone();
            let (best_move, best_value) = me.minimax(board, max_depth, alpha, beta);

            if !me.search_state.lock().unwrap().search_active {
                if LOG_STATS {
                    println!("--------------------------------------------");
                    println!("* Minimax worker exited after max_depth {}", max_depth - 1);
                    println!("* Best move had value {:?}", current_best_value);
                    println!("* NPS: {:.2e} ({:?})", me.current_nps(), me.start_t.elapsed());
                    println!("--------------------------------------------\n");
                }
                return;
            }

            if let Valuation::TerminalWhiteWin { plies } = best_value {
                if LOG_STATS {
                    println!("--------------------------------------------");
                    println!("* Found certain win in {} plies", plies);
                    println!("--------------------------------------------\n");
                }
                {
                    let mut search_state = me.search_state.lock().unwrap();
                    search_state.current_best_move = best_move;
                    search_state.search_active = false;
                }
                return;
            }

            if let TerminalBlackWin { plies } = best_value {
                // all moves are certain losses, pick the one with the most plies and exit
                if LOG_STATS {
                    println!("--------------------------------------------");
                    println!("* Found certain loss in {} plies", plies);
                    println!("--------------------------------------------");
                    println!();
                }
                {
                    let mut search_state = me.search_state.lock().unwrap();
                    search_state.current_best_move = best_move;
                    search_state.search_active = false;
                }
                return;
            }

            me.search_state.lock().unwrap().current_best_move = best_move;
            current_best_value = best_value;
        }

        me.search_state.lock().unwrap().search_active = false;

        if LOG_STATS {
            println!("--------------------------------------------");
            println!("* Minimax worker exited after search depth {}", max_depth);
            println!(
                "* Best move {} had value {:?}",
                me.search_state.lock().unwrap().current_best_move,
                current_best_value
            );
            println!("* NPS: {:.2e} ({:?})", me.current_nps(), me.start_t.elapsed());
            println!("--------------------------------------------\n");
        }
    }
}

/*====================================================================================================================*/

pub fn minimax_search(board: &Board, valuation_fn: ValuationFn, search_state: SharedMinimaxSearchState) {
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
                let worker: MinimaxWorker = MinimaxWorker::new(valuation_fn, search_state);
                worker.start_search(board);
            }
        });
    }

    // detach worker thread; will get shut down automatically when search_active gets set to false
    drop(t_handle);
}
