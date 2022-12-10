use rand::{thread_rng, RngCore};

use super::node::Node;
use crate::{Board, Move, Player};

fn value_heuristic(board: &Board) -> f32 {
    board.our_store as f32 - board.their_store as f32
}

fn count_nodes(node: &Node) -> u64 {
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
}
