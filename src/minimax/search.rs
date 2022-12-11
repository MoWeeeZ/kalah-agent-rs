use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rand::{thread_rng, Rng};

use crate::{Board, Move, Player};

fn value_heuristic(board: &Board) -> f32 {
    board.our_store as f32 - board.their_store as f32
}

/* fn count_nodes(node: &Node) -> u64 {
    if node.has_children() {
        node.child_iter().map(count_nodes).sum()
    } else {
        1
    }
}

fn reset_and_extend_tree(node: &mut Node) {
    node.value = f32::NEG_INFINITY;

    if !node.has_children() {
        // leaf node

        let board = node.board().clone();
        let valid_moves = board.legal_moves(Player::White);
        let depth = node.depth();

        for valid_move in valid_moves {
            let mut next_board = board.clone();
            let flip = !next_board.apply_move(valid_move);

            if flip {
                next_board.flip_board();
            }

            let next_node = Box::new(Node::new(next_board, valid_move, depth + 1));
            node.append_child(next_node);
        }
    } else {
        // inner node
        for child_node in node.child_iter_mut() {
            reset_and_extend_tree(child_node);
        }
    }
}

fn minimax_tree_value(node: &mut Node) {
    if !node.has_children() {
        node.value = value_heuristic(node.board());

        return;
    }

    let flipped = node.board().flipped();

    let mut value = node.value;

    for child_node in node.child_iter_mut() {
        minimax_tree_value(child_node);

        let child_value = if flipped == child_node.board().flipped() {
            child_node.value
        } else {
            -child_node.value
        };

        if child_value > value {
            value = child_value;
        }
    }

    node.value = value;
}

pub fn minimax_search(board: &Board, max_depth: u64) -> Move {
    assert!(
        board.has_legal_move(Player::White),
        "Called minimax_search on board with no legal moves"
    );
    let start_time = std::time::Instant::now();

    let mut best_move = Move::new(127, Player::White);

    let mut root_node = Node::new(board.clone(), Move::new(127, Player::White), 0);

    for _depth in 0..max_depth {
        reset_and_extend_tree(&mut root_node);
    }

    minimax_tree_value(&mut root_node);

    let mut best_value = f32::NEG_INFINITY;

    for child_node in root_node.child_iter() {
        let value = match child_node.colour() {
            Player::White => child_node.value,
            Player::Black => -child_node.value,
        };

        if value > best_value || (value == best_value && thread_rng().next_u64() % 2 == 0) {
            best_value = value;
            best_move = child_node.pre_move();
        }
    }

    let end_time = std::time::Instant::now();

    let dur = end_time - start_time;

    let node_count = count_nodes(&root_node);

    println!("Ran minimax to depth {}", max_depth);
    println!("Total nodes considered: {}", node_count);
    println!("NPS: {:.2e}", node_count as f64 / dur.as_secs_f64());
    println!("Best move have value {}\n", best_value);

    best_move
} */

fn minimax(board: &Board, remaining_depth: u32, alpha: f32, beta: f32) -> f32 {
    if remaining_depth == 0 {
        return value_heuristic(board);
    }

    let legal_moves = board.legal_moves(Player::White);

    if legal_moves.is_empty() {
        // we have no move left -> this is a terminal node
        // meaning the player with more seeds in their store wins the game
        // thus if we have more seeds in the store (i.e. value > 0) this node is a guaranteed win
        let mut board = board.clone();
        board.finish_game();

        let value = match value_heuristic(&board) {
            val if val > 0.0 => f32::INFINITY,
            val if val < 0.0 => f32::NEG_INFINITY,
            val if val == 0.0 => 0.0,
            val => panic!("Value has invalid value {}", val),
        };

        return value;
    }

    let mut best_value = f32::NEG_INFINITY;
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

            let their_value = minimax(&next_board, remaining_depth - 1, their_alpha, their_beta);

            match their_next_move {
                true => -their_value,
                false => their_value,
            }
        };

        if value > best_value {
            best_value = value;
        }

        if value >= alpha {
            alpha = value;
        }

        if value >= beta {
            // beta cutoff, return early
            return best_value;
        }
    }

    best_value
}

fn minimax_worker(board: Board, current_best_move: Arc<Mutex<Move>>, search_active: Arc<AtomicBool>) {
    let legal_moves = board.legal_moves(Player::White);

    let mut current_best_value = f32::NAN;

    for max_depth in 0.. {
        let mut best_value = f32::NEG_INFINITY;
        let mut best_move = Move::new(127, Player::White);

        let mut value;
        let mut alpha = f32::NEG_INFINITY;
        let beta = f32::INFINITY;

        for current_move in legal_moves.iter() {
            if !search_active.load(Ordering::Acquire) {
                // since max_depth search never completed: max_depth - 1
                println!("Minimax worker exited after max_depth {}", max_depth - 1);
                println!("Best move had value {}", current_best_value);
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

                let their_value = minimax(&next_board, max_depth, their_alpha, their_beta);

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

                if best_value == f32::INFINITY {
                    println!("Found certain win");
                    *current_best_move.lock().unwrap() = best_move;
                    search_active.store(false, Ordering::Release);
                    return;
                }
            }

            if value >= alpha {
                alpha = value;
            }

            if value >= beta {
                // beta cutoff
                break;
            }
        }

        *current_best_move.lock().unwrap() = best_move;
        current_best_value = best_value;

        /* println!(
            "Depth {}: found best move with value {}\talpha: {}\t{}",
            max_depth, best_value, alpha, beta
        ); */

        if current_best_value == f32::NEG_INFINITY {
            println!("Found certain loss");
            // don't exit early if we find a certain loss: our opponent might not've :)

            /* search_active.store(false, Ordering::Release);
            return; */
        }
    }
}

pub fn minimax_search(board: &Board, thinking_dur: Duration) -> Move {
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
            minimax_worker(worker_board, worker_current_best_move, worker_search_active);
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
