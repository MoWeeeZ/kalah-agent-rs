use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::kalah::valuation::{Valuation, ValuationFn};
use crate::{Board, Move, Player};

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

struct MinimaxWorker<const ALPHA_BETA_PRUNE: bool> {
    search_state: Arc<Mutex<MinimaxSearchState>>,

    valuation_fn: ValuationFn,

    total_nodes_visited: u64,

    start_t: Instant,
}

impl<const ALPHA_BETA_PRUNE: bool> MinimaxWorker<ALPHA_BETA_PRUNE> {
    pub fn new(valuation_fn: ValuationFn, search_state: SharedMinimaxSearchState) -> Self {
        MinimaxWorker {
            search_state,
            valuation_fn,
            total_nodes_visited: 0,
            start_t: Instant::now(),
        }
    }

    #[cfg(debug_assertions)]
    fn current_nps(&self) -> f64 {
        self.total_nodes_visited as f64 / self.start_t.elapsed().as_secs_f64()
    }

    fn minimax(&mut self, board: &Board, remaining_depth: u32, alpha: Valuation, beta: Valuation) -> Valuation {
        use std::cmp::Ordering::{Equal, Greater, Less};

        self.total_nodes_visited += 1;

        if remaining_depth == 0 || !board.has_legal_move() {
            // return board.valuation();
            return (self.valuation_fn)(board);
        }

        let legal_moves = board.legal_moves(Player::White);

        // immediate win for Black
        let mut best_value = Valuation::TerminalBlackWin { plies: 0 };
        let mut alpha = alpha;

        let mut child_states: Vec<(Board, bool)> = legal_moves
            .iter()
            .map(|legal_move| {
                let mut next_board = board.clone();
                let their_next_move = !next_board.apply_move(*legal_move);

                if their_next_move {
                    next_board.flip_board();
                }

                (next_board, their_next_move)
            })
            .collect();

        // move ordering: search bonus moves first
        child_states.sort_by(|&(_, tnm1), &(_, tnm2)| match (tnm1, tnm2) {
            (true, false) => Less,
            (false, true) => Greater,
            _ => Equal,
        });

        for (next_board, their_next_move) in child_states {
            /* let mut next_board = board.clone();
            let their_next_move = !next_board.apply_move(legal_move);

            if their_next_move {
                next_board.flip_board();
            } */

            // run minimax on child; if neccessary, shuffle and negate value, alpha, and beta from their perspective to ours
            let value = {
                let (their_alpha, their_beta) = match their_next_move {
                    true => (-beta, -alpha),
                    false => (alpha, beta),
                };

                // if next move is by opponent: decrease remaining_depth
                // if it's a bonus move, don't decrease to keep the game tree balanced
                let next_remaining_depth = if their_next_move {
                    remaining_depth - 1
                } else {
                    remaining_depth
                };

                let their_value = self.minimax(&next_board, next_remaining_depth, their_alpha, their_beta);

                match their_next_move {
                    true => -their_value.increase_depth(),
                    false => their_value.increase_depth(),
                }
            };

            if value > best_value {
                best_value = value;
            }

            if ALPHA_BETA_PRUNE {
                // value either has higher value or both are terminal and value is shorter sequence
                if value > alpha {
                    alpha = value;
                }

                if value >= beta {
                    // beta cutoff, return early
                    return best_value;
                }
            }
        }

        best_value
    }

    pub fn start_search(self, board: Board) {
        use Valuation::{TerminalBlackWin, TerminalWhiteWin};

        let mut me = self;

        me.start_t = std::time::Instant::now();

        // moves, their_next_moves, boards with move ordering by last iteration; initially no ordering
        let mut moves_boards_ordered: Vec<(Move, bool, Board, Valuation)> = board
            .legal_moves(Player::White)
            .into_iter()
            .map(|next_move| {
                let mut next_board = board.clone();
                let their_next_move = !next_board.apply_move(next_move);

                if their_next_move {
                    next_board.flip_board();
                }
                (
                    next_move,
                    their_next_move,
                    next_board,
                    Valuation::NonTerminal { value: 0.0 },
                )
            })
            .collect();

        let num_moves = moves_boards_ordered.len();

        let mut current_best_value;

        #[cfg(debug_assertions)]
        {
            current_best_value = Valuation::TerminalBlackWin { plies: 0 };
        }

        for max_depth in 0.. {
            // let mut best_value = TerminalBlackWin { plies: 0 };
            // let mut best_move = moves_ordered[0];

            let mut value;
            let mut alpha = TerminalBlackWin { plies: 0 };
            let beta = TerminalWhiteWin { plies: 0 };

            let mut moves_boards_vals: Vec<(Move, bool, Board, Valuation)> = Vec::with_capacity(num_moves);

            for (next_move, their_next_move, next_board, _last_valuation) in moves_boards_ordered.into_iter() {
                if !me.search_state.lock().unwrap().search_active {
                    // since max_depth search never completed: max_depth - 1
                    #[cfg(debug_assertions)]
                    {
                        println!("--------------------------------------------");
                        println!("* Minimax worker exited after max_depth {}", max_depth - 1);
                        println!("* Best move had value {:?}", current_best_value);
                        println!("* NPS: {:.2e}", me.current_nps());
                        println!("* alpha-beta pruning: {}", ALPHA_BETA_PRUNE);
                        println!("--------------------------------------------\n");
                    }
                    return;
                }

                value = {
                    let (their_alpha, their_beta) = match their_next_move {
                        true => (-beta, -alpha),
                        false => (alpha, beta),
                    };

                    let their_value = me.minimax(&next_board, max_depth, their_alpha, their_beta);

                    match their_next_move {
                        true => -their_value,
                        false => their_value,
                    }
                };

                // replace if value is either better or the same and wins a coin flip
                // (to make decision non-deterministic in that case)
                /* if value > best_value || value == best_value && thread_rng().gen::<bool>() {
                best_value = value;
                best_move = current_move; */
                /* } */

                if ALPHA_BETA_PRUNE && value >= alpha {
                    alpha = value;
                }

                moves_boards_vals.push((next_move, their_next_move, next_board, value));

                // no beta cutoff since beta is always -inf
                /* if value >= beta {
                    // beta cutoff
                    break;
                } */
            } // for current_move in moves_ordered.into_iter()

            moves_boards_vals.sort_by(|&(_, val1, _, _), &(_, val2, _, _)| val1.cmp(&val2).reverse());

            // *current_best_move.lock().unwrap() = best_move;
            me.search_state.lock().unwrap().current_best_move = moves_boards_vals.first().unwrap().0;
            current_best_value = moves_boards_vals.first().unwrap().3;

            moves_boards_ordered = moves_boards_vals;

            /* println!(
                "Depth {}: found best move with value {}\talpha: {}\t{}",
                max_depth, best_value, alpha, beta
            ); */

            // best move is a certain win for us
            if let Valuation::TerminalWhiteWin { plies: _plies } = current_best_value {
                #[cfg(debug_assertions)]
                {
                    println!("--------------------------------------------");
                    println!("* Found certain win in {} plies", _plies);
                    println!("--------------------------------------------\n");
                }
                // *current_best_move.lock().unwrap() = best_move;
                // search_active.store(false, Ordering::Release);
                me.search_state.lock().unwrap().search_active = false;
                return;
            }

            // best move is a certain loss for us
            if let TerminalBlackWin { plies: _plies } = current_best_value {
                // don't exit early if we find a certain loss: our opponent might not've :)
                #[cfg(debug_assertions)]
                {
                    println!("--------------------------------------------");
                    println!("* Found certain loss in {} plies", _plies);
                    println!("--------------------------------------------");
                    println!();
                }
                /* self.search_state.lock().unwrap().search_active = false;
                return; */
            }
        }
    }
}

/*====================================================================================================================*/

pub fn minimax_search<const ALPHA_BETA_PRUNE: bool>(
    board: &Board,
    valuation_fn: ValuationFn,
    search_state: SharedMinimaxSearchState,
) {
    assert!(
        board.has_legal_move(),
        "Called minimax_search on board with no legal moves"
    );

    let t_handle;

    {
        let worker_board = board.clone();

        t_handle = std::thread::spawn(move || {
            let worker: MinimaxWorker<ALPHA_BETA_PRUNE> = MinimaxWorker::new(valuation_fn, search_state);
            worker.start_search(worker_board);
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
