use std::sync::{Arc, Mutex};
use std::time::Instant;

// use rand::{thread_rng, Rng};

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

    fn minimax(&mut self, board: &Board, remaining_depth: u32, alpha: Valuation, beta: Valuation) -> (Move, Valuation) {
        self.total_nodes_visited += 1;

        if remaining_depth == 0 || !board.has_legal_move() {
            return (Move::new(127, Player::White), (self.valuation_fn)(board));
        }

        let legal_moves = board.legal_moves(Player::White);

        // immediate win for Black
        let mut best_move = Move::new(0, Player::White);
        let mut best_value = Valuation::TerminalBlackWin { plies: 0 };
        let mut alpha = alpha;

        for move_ in legal_moves {
            let mut next_board = board.clone();
            let their_next_move = !next_board.apply_move(move_);

            // run minimax on child; if neccessary, shuffle and negate value, alpha, and beta from their perspective to ours
            let value = {
                if their_next_move {
                    next_board.flip_board();
                }

                let (their_alpha, their_beta) = if their_next_move {
                    (-beta, -alpha)
                } else {
                    (alpha, beta)
                };

                // if next move is by opponent: decrease remaining_depth
                // if it's a bonus move, don't decrease to keep the game tree balanced
                let next_remaining_depth = if their_next_move {
                    remaining_depth - 1
                } else {
                    remaining_depth
                };

                let (_, their_value) = self.minimax(&next_board, next_remaining_depth, their_alpha, their_beta);

                if their_next_move {
                    -their_value.increase_plies()
                } else {
                    their_value.increase_plies()
                }
            };

            /* let value = if their_next_move {
                next_board.flip_board();
                -self.minimax(&next_board, remaining_depth - 1, -beta, -alpha)
            } else {
                self.minimax(&next_board, remaining_depth, alpha, beta)
            }
            .increase_plies(); */

            if value > best_value
            /* || value == best_value && thread_rng().gen::<bool>() */
            {
                best_move = move_;
                best_value = value;
            }

            if value >= beta {
                // beta cutoff, return early
                break;
            }

            // value either has higher value or both are terminal and value is shorter sequence
            if value > alpha {
                alpha = value;
            }
        }

        (best_move, best_value)
    }

    pub fn start_search(self, board: Board) {
        use Valuation::{TerminalBlackWin, TerminalWhiteWin};

        let mut me = self;

        me.start_t = std::time::Instant::now();

        let mut current_best_value /* = Valuation::TerminalBlackWin { plies: 0 } */;

        let alpha = TerminalBlackWin { plies: 0 };
        let beta = TerminalWhiteWin { plies: 0 };

        for max_depth in 1.. {
            let (best_move, best_value) = me.minimax(&board, max_depth, alpha, beta);

            // *current_best_move.lock().unwrap() = best_move;
            me.search_state.lock().unwrap().current_best_move = best_move;
            current_best_value = best_value;

            if let Valuation::TerminalWhiteWin { plies } = best_value {
                if LOG_STATS {
                    println!("--------------------------------------------");
                    println!("* Found certain win in {} plies", plies);
                    println!("--------------------------------------------\n");
                }
                // *current_best_move.lock().unwrap() = best_move;
                // search_active.store(false, Ordering::Release);
                {
                    let mut search_state = me.search_state.lock().unwrap();
                    search_state.current_best_move = best_move;
                    search_state.search_active = false;
                }
                return;
            }

            if let TerminalBlackWin { plies } = current_best_value {
                // all moves are certain losses, pick the one with the most plies and exit
                if LOG_STATS {
                    println!("--------------------------------------------");
                    println!("* Found certain loss in {} plies", plies);
                    println!("--------------------------------------------");
                    println!();
                }
                me.search_state.lock().unwrap().search_active = false;
                return;
            }

            if !me.search_state.lock().unwrap().search_active {
                if LOG_STATS {
                    println!("--------------------------------------------");
                    println!("* Minimax worker exited after max_depth {}", max_depth - 1);
                    println!("* Best move had value {:?}", current_best_value);
                    println!("* NPS: {:.2e}", me.current_nps());
                    println!("--------------------------------------------\n");
                }
                return;
            }
        }

        me.search_state.lock().unwrap().search_active = false;
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
