use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rand::{thread_rng, Rng};

use crate::board::Valuation;
use crate::{Board, Move, Player};

struct MinimaxWorker {
    total_nodes_visited: u64,
    alpha_beta_prune: bool,

    start_t: Instant,
}

impl MinimaxWorker {
    pub fn new(alpha_beta_prune: bool) -> Self {
        MinimaxWorker {
            total_nodes_visited: 0,
            alpha_beta_prune,
            start_t: Instant::now(),
        }
    }

    fn current_nps(&self) -> f64 {
        let stop_t = std::time::Instant::now();
        self.total_nodes_visited as f64 / (stop_t - self.start_t).as_secs_f64()
    }

    fn minimax(&mut self, board: &Board, remaining_depth: u32, alpha: Valuation, beta: Valuation) -> Valuation {
        self.total_nodes_visited += 1;

        if remaining_depth == 0 || !board.has_legal_move() {
            return board.value_heuristic();
        }

        let legal_moves = board.legal_moves(Player::White);

        // immediate win for Black
        let mut best_value = Valuation::TerminalBlackWin { plies: 0 };
        let mut alpha = alpha;

        for legal_move in legal_moves {
            let mut next_board = board.clone();
            let their_next_move = !next_board.apply_move(legal_move);

            if their_next_move {
                next_board.flip_board();
            }

            // run minimax on child; if neccessary, shuffle and negate value, alpha, and beta from their perspective to ours
            let value = {
                let (their_alpha, their_beta) = match their_next_move {
                    true => (-beta, -alpha),
                    false => (alpha, beta),
                };

                let their_value = self.minimax(&next_board, remaining_depth - 1, their_alpha, their_beta);

                match their_next_move {
                    true => -their_value.advance_step(),
                    false => their_value.advance_step(),
                }
            };

            if value > best_value {
                best_value = value;
            }

            if self.alpha_beta_prune {
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

    pub fn start_search(self, board: Board, current_best_move: Arc<Mutex<Move>>, search_active: Arc<AtomicBool>) {
        use Valuation::{TerminalBlackWin, TerminalWhiteWin};

        let mut me = self;

        me.start_t = std::time::Instant::now();

        let legal_moves = board.legal_moves(Player::White);

        let mut current_best_value = TerminalBlackWin { plies: 0 };

        for max_depth in 0.. {
            let mut best_value = TerminalBlackWin { plies: 0 };
            let mut best_move = legal_moves[0];

            let mut value;
            let mut alpha = TerminalBlackWin { plies: 0 };
            let beta = TerminalWhiteWin { plies: 0 };

            for current_move in legal_moves.iter() {
                if !search_active.load(Ordering::Acquire) {
                    // since max_depth search never completed: max_depth - 1
                    println!("--------------------------------------------");
                    println!("* Minimax worker exited after max_depth {}", max_depth - 1);
                    println!("* Best move had value {:?}", current_best_value);
                    println!("* NPS: {:.2e}", me.current_nps());
                    println!("* alpha-beta pruning: {}", me.alpha_beta_prune);
                    println!("--------------------------------------------\n");
                    return;
                }

                let mut next_board = board.clone();
                let their_move = !next_board.apply_move(*current_move);

                if their_move {
                    next_board.flip_board();
                }

                value = {
                    let (their_alpha, their_beta) = match their_move {
                        true => (-beta, -alpha),
                        false => (alpha, beta),
                    };

                    let their_value = me.minimax(&next_board, max_depth, their_alpha, their_beta);

                    match their_move {
                        true => -their_value,
                        false => their_value,
                    }
                };

                // replace if value is either better or the same and wins a coin flip
                // (to make decision non-deterministic in that case)
                if value > best_value || value == best_value && thread_rng().gen::<f64>() > 0.5 {
                    best_value = value;
                    best_move = *current_move;

                    if let Valuation::TerminalWhiteWin { plies } = best_value {
                        println!("--------------------------------------------");
                        println!("* Found certain win in {} plies", plies);
                        println!("--------------------------------------------\n");
                        *current_best_move.lock().unwrap() = best_move;
                        search_active.store(false, Ordering::Release);
                        return;
                    }
                }

                if me.alpha_beta_prune {
                    if value >= alpha {
                        alpha = value;
                    }

                    if value >= beta {
                        // beta cutoff
                        break;
                    }
                }
            }

            *current_best_move.lock().unwrap() = best_move;
            current_best_value = best_value;

            /* println!(
                "Depth {}: found best move with value {}\talpha: {}\t{}",
                max_depth, best_value, alpha, beta
            ); */

            if let TerminalBlackWin { plies } = current_best_value {
                println!("--------------------------------------------");
                println!("* Found certain loss in {} plies", plies);
                println!("--------------------------------------------");
                println!();
                // don't exit early if we find a certain loss: our opponent might not've :)

                search_active.store(false, Ordering::Release);
                return;
            }
        }
    }
}

/*====================================================================================================================*/

pub fn minimax_search(board: &Board, thinking_dur: Duration, alpha_beta_prune: bool) -> Move {
    assert!(
        board.has_legal_move(),
        "Called minimax_search on board with no legal moves"
    );

    let start_t = std::time::Instant::now();

    let search_active = Arc::new(AtomicBool::new(true));
    let current_best_move = Arc::new(Mutex::new(Move::new(127, Player::White)));

    let t_handle;

    {
        let worker_board = board.clone();
        let worker_current_best_move = Arc::clone(&current_best_move);
        let worker_search_active = Arc::clone(&search_active);

        t_handle = std::thread::spawn(move || {
            let worker = MinimaxWorker::new(alpha_beta_prune);
            worker.start_search(worker_board, worker_current_best_move, worker_search_active);
        });
    }

    // subtract the setup time and a buffer time from thinking_dur
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

    best_move
}
