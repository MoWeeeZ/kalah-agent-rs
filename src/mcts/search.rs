use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use rand::seq::SliceRandom;

use crate::mcts::node::{Edge, Node, SharedNode};
use crate::{Board, Move};

// inverse temperature for move sampling in mcts selection phase
const SELECTION_BETA: f64 = 1.0;
// mixing factor for uniform probability distribution in mcts selection phase
const EPSILON: f64 = 0.25;

// inverse temperature for next move selection
const POLICY_BETA: f64 = 1.0;

/*====================================================================================================================*/

// represents the global search object, which is held by the agent and controls the individual search threads,
// including starting and stopping them, getting the current best move, and advancing the game tree on moves made
pub struct Search {
    threads: Vec<thread::JoinHandle<()>>,

    search_active: Arc<AtomicBool>,

    // root node behind a Arc<RwLock<Node>>; this allows all threads to simultaneously traverse the game tree
    // only nodes that are currently being altered will be locked, as they all are behind RwLocks
    root_node: SharedNode,
}

impl Search {
    pub fn new(board: Board) -> Self {
        Search {
            threads: Vec::new(),
            search_active: Arc::new(AtomicBool::new(false)),
            root_node: Node::new_shared(board),
        }
    }

    pub fn start_threads(&mut self, thread_count: u64) {
        assert!(thread_count > 0, "Can't start zero threads.");
        assert!(
            self.threads.is_empty(),
            "Trying to start thread with threads already running"
        );

        self.search_active.store(true, Ordering::Release);

        for thread_id in 0..thread_count {
            let root_node = Arc::clone(&self.root_node);
            let search_active = Arc::clone(&self.search_active);

            self.threads.push(thread::spawn(move || {
                let search_thread = SearchWorker::new(thread_id, root_node, search_active);

                search_thread.run();
            }));
        }
    }

    pub fn stop_threads(&mut self) {
        self.search_active.store(false, Ordering::Release);

        while !self.threads.is_empty() {
            let last = self.threads.pop().unwrap();

            last.join().expect("Search thread panicked!");
        }
    }

    pub fn inform_move(&mut self, move_: Move) {
        let node = match self.root_node.read().unwrap().get_child_node(move_) {
            Some(node) => node,
            None => {
                let mut board = self.root_node.read().unwrap().board().clone();
                board.apply_move(move_);
                Node::new_shared(board)
            }
        };

        self.root_node = node;

        // std::mem::swap(&mut self.root_node, &mut node);

        // // drop old root node to make all nodes deallocate after all search threads have no reference to them anymore
        // drop(node);
    }

    pub fn current_best_move(&self) -> Move {
        let policy = self.root_node.read().unwrap().get_current_policy();

        // each move with their unnormalised probability
        let probabilities: Vec<(Move, f64)> = policy
            .into_iter()
            .map(|(move_, value)| (move_, (POLICY_BETA * value).exp()))
            .collect();

        let softmax_sum: f64 = probabilities.iter().map(|&(_, p)| p).sum();

        let next_move = probabilities
            .choose_weighted(&mut rand::thread_rng(), |&(_, p)| p / softmax_sum)
            .unwrap()
            .0;

        next_move
    }
}

impl Drop for Search {
    fn drop(&mut self) {
        self.stop_threads();
    }
}

/*====================================================================================================================*/

#[allow(dead_code)]
fn value_heuristic(board: &Board) -> f32 {
    board.our_store as f32 - board.their_store as f32
}

struct SearchWorker {
    thread_id: u64,

    root_node: SharedNode,
    search_active: Arc<AtomicBool>,
}

impl SearchWorker {
    pub fn new(thread_id: u64, root_node: SharedNode, search_active: Arc<AtomicBool>) -> Self {
        SearchWorker {
            thread_id,
            root_node,
            search_active,
        }
    }

    // entry point for search worker: this function will loop until the search is stopped
    pub fn run(self) {
        // sanity check: Search::start_threads sets search_active to true before starting threads, so this should never fire
        assert!(
            self.search_active.load(Ordering::Acquire),
            "Started a search thread, but  search is not active"
        );

        println!("Search thread {} starting up.", self.thread_id);

        while self.search_active.load(Ordering::Acquire) {
            SearchWorker::mcts_iteration(self.root_node.clone());
        }

        println!("Search thread {} shutting down.", self.thread_id);
    }

    //
    fn mcts_iteration(node: SharedNode) -> f64 {
        let next_edge;

        // region in which node is read-locked
        {
            let node = node.read().unwrap();
            // each move with their respective value
            let values = node.get_current_edge_values();
            // each move with their unnormalised probability
            let probabilities: Vec<(&Edge, f64)> = values
                .into_iter()
                .map(|(edge, value)| (edge, (SELECTION_BETA * value).exp()))
                .collect();

            let softmax_sum: f64 = probabilities.iter().map(|&(_, p)| p).sum();

            let uni_prob = 1.0 / probabilities.len() as f64;

            next_edge = probabilities
                .choose_weighted(&mut rand::thread_rng(), |&(_, p)| {
                    (1.0 - EPSILON) * (p / softmax_sum) + EPSILON * uni_prob
                })
                .unwrap()
                .0
                .clone();

            // drop node and probablilities to unlock other edges
            drop(probabilities);
        }

        // backpropagated v from terminal node
        let v = match next_edge.child_node() {
            // we're an inner node => keep moving down the tree
            Some(next_node) => SearchWorker::mcts_iteration(next_node),
            // child_node of edge does not exist => we've hit a leaf node => expand
            None => SearchWorker::expand_node(Arc::clone(&node), next_edge.get_move()),
        };

        // lock node and update all its edges
        node.write().unwrap().update_with_value(v);

        v
    }

    fn expand_node(_node: SharedNode, _move_: Move) -> f64 {
        todo!()
    }
}
