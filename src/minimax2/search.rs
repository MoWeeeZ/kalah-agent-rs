use std::sync::{Arc, Mutex};
use std::time::Instant;

use rand::{thread_rng, Rng};

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

    #[allow(dead_code)]
    fn current_nps(&self) -> f64 {
        self.total_nodes_visited as f64 / self.start_t.elapsed().as_secs_f64()
    }

    fn minimax(
        &mut self,
        board: &Board,
        remaining_depth: u32, /* , alpha: Valuation, beta: Valuation */
    ) -> Valuation {
        // use std::cmp::Ordering::{Equal, Greater, Less};

        self.total_nodes_visited += 1;

        if remaining_depth == 0 || !board.has_legal_move() {
            // return board.valuation();
            return (self.valuation_fn)(board);
        }

        let legal_moves = board.legal_moves(Player::White);

        // immediate win for Black
        let mut best_value = Valuation::TerminalBlackWin { plies: 0 };
        /* let mut alpha = alpha; */

        for next_move in legal_moves {
            let mut next_board = board.clone();
            let their_next_move = !next_board.apply_move(next_move);

            // run minimax on child; if neccessary, shuffle and negate value, alpha, and beta from their perspective to ours
            let value = {
                if their_next_move {
                    next_board.flip_board();
                }

                /* let (their_alpha, their_beta) = if their_next_move {
                    (-beta, -alpha)
                } else {
                    (alpha, beta)
                }; */

                // if next move is by opponent: decrease remaining_depth
                // if it's a bonus move, don't decrease to keep the game tree balanced
                let next_remaining_depth = if their_next_move {
                    remaining_depth - 1
                } else {
                    remaining_depth
                };

                let their_value = self.minimax(&next_board, next_remaining_depth /* , their_alpha, their_beta */);

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
            .increase_depth(); */

            if value > best_value {
                best_value = value;
            }

            /* // value either has higher value or both are terminal and value is shorter sequence
            if value > alpha {
                alpha = value;
            }

            if value >= beta {
                // beta cutoff, return early
                return best_value;
            } */
        }

        best_value
    }

    pub fn start_search(self, board: Board) {
        use Valuation::{TerminalBlackWin, TerminalWhiteWin};

        let mut me = self;

        me.start_t = std::time::Instant::now();

        let legal_moves = board.legal_moves(Player::White);

        let mut current_best_value = Valuation::TerminalBlackWin { plies: 0 };

        for max_depth in 0.. {
            let mut best_value = TerminalBlackWin { plies: 0 };
            let mut best_move = legal_moves[0];

            let mut value;
            /* let mut alpha = TerminalBlackWin { plies: 0 };
            let beta = TerminalWhiteWin { plies: 0 }; */

            for current_move in legal_moves.iter() {
                if !me.search_state.lock().unwrap().search_active {
                    // since max_depth search never completed: max_depth - 1
                    if LOG_STATS {
                        println!("--------------------------------------------");
                        println!("* Minimax worker exited after max_depth {}", max_depth - 1);
                        println!("* Best move had value {:?}", current_best_value);
                        println!("* NPS: {:.2e}", me.current_nps());
                        println!("--------------------------------------------\n");
                    }
                    return;
                }

                let mut next_board = board.clone();
                let their_move = !next_board.apply_move(*current_move);

                if their_move {
                    next_board.flip_board();
                }

                value = {
                    /* let (their_alpha, their_beta) = match their_move {
                        true => (-beta, -alpha),
                        false => (alpha, beta),
                    }; */

                    let their_value = me.minimax(&next_board, max_depth /* , their_alpha, their_beta */);

                    match their_move {
                        true => -their_value,
                        false => their_value,
                    }
                };

                // replace if value is either better or the same and wins a coin flip
                // (to make decision non-deterministic in that case)
                if value > best_value || value == best_value && thread_rng().gen::<bool>() {
                    best_value = value;
                    best_move = *current_move;

                    if let Valuation::TerminalWhiteWin { plies: _plies } = best_value {
                        if LOG_STATS {
                            println!("--------------------------------------------");
                            println!("* Found certain win in {} plies", _plies);
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
                }

                /* if value >= alpha {
                    alpha = value;
                } */

                // no beta cutoff since beta is always -inf
                /* if value >= beta {
                    // beta cutoff
                    break;
                } */
            }

            // *current_best_move.lock().unwrap() = best_move;
            me.search_state.lock().unwrap().current_best_move = best_move;
            current_best_value = best_value;

            /* println!(
                "Depth {}: found best move with value {}\talpha: {}\t{}",
                max_depth, best_value, alpha, beta
            ); */

            if let TerminalBlackWin { plies: _plies } = current_best_value {
                // all moves are certain losses, pick the one with the most plies and exit
                if LOG_STATS {
                    println!("--------------------------------------------");
                    println!("* Found certain loss in {} plies", _plies);
                    println!("--------------------------------------------");
                    println!();
                }
                me.search_state.lock().unwrap().search_active = false;
                return;
            }
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

    /* // subtract the setup time and a buffer time from thinking_dur
    /* let remaining_thinking_dur =
        thinking_dur - (std::time::Instant::now() - start_t) - BUFFER_TIME;

    std::thread::sleep(remaining_thinking_dur); */

    // buffer to end of timer we want to keep
    const BUFFER_TIME: Duration = std::time::Duration::from_millis(50);
    // time to sleep between checking if the search is done
    const SLEEP_TIME: Duration = std::time::Duration::from_millis(100);
    let stop_time = start_t + (thinking_dur - BUFFER_TIME);

    // wait for either the timer to (almost) expire or the worker thread to stop
    loop {
        let now = std::time::Instant::now();

        if stop_time < now || !search_active.load(Ordering::Acquire) {
            // timer expired or search stopped by worker thread
            break;
        }

        let remaining_time = stop_time - now;

        // sleep for the minimum of SLEEP_TIME and remaining_time
        std::thread::sleep(std::cmp::min(SLEEP_TIME, remaining_time));
    }

    let best_move = *current_best_move.lock().unwrap();

    search_active.store(false, Ordering::Release);
    // detach worker thread; it will exit soonish
    drop(t_handle);

    best_move */
}
