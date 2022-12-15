use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use crate::kalah::valuation::{Valuation, ValuationFn};
use crate::mcts::node::{Edge, Node, SharedNode};
use crate::util::math::{sample_index_weighted, softmax};
use crate::{Board, Move};

// inverse temperature for move sampling in mcts selection phase
const SELECTION_BETA: f32 = 1.0;
// mixing factor for uniform probability distribution in mcts selection phase
// const EPSILON: f32 = 0.25;

/*====================================================================================================================*/

// represents the global search object, which is held by the agent and controls the individual search threads,
// including starting and stopping them, getting the current best move, and advancing the game tree on moves made
pub struct Search {
    threads: Vec<thread::JoinHandle<()>>,

    search_active: Arc<AtomicBool>,

    // root node behind a Arc<RwLock<Node>>; this allows all threads to simultaneously traverse the game tree
    // only nodes that are currently being altered will be locked, as they all are behind RwLocks
    root_node: SharedNode,

    valuation_fn: ValuationFn,
}

impl Search {
    pub fn new(board: Board, valuation_fn: ValuationFn) -> Self {
        Search {
            threads: Vec::new(),
            search_active: Arc::new(AtomicBool::new(false)),
            root_node: Node::new_shared(board, 0),
            valuation_fn,
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
            let valuation_fn = self.valuation_fn;

            self.threads.push(thread::spawn(move || {
                let search_thread = SearchWorker::new(thread_id, root_node, search_active, valuation_fn);

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
        let mut node = match self.root_node.read().unwrap().get_child_node(move_) {
            Some(node) => node,
            None => {
                let mut board = self.root_node.read().unwrap().board().clone();
                let depth = self.root_node.read().unwrap().depth() + 1;
                board.apply_move(move_);
                Node::new_shared(board, depth)
            }
        };

        // self.root_node = node;

        // swap new root and old root
        std::mem::swap(&mut self.root_node, &mut node);

        // drop old root on new thread so node deallocation won't block the main thread
        let gc_thread = std::thread::spawn(move || {
            // drop node to trigger deallocation (or not, then the droppping worker thread will have to take care of it)
            drop(node);
        });
        // detach gc_thread
        drop(gc_thread);
    }

    pub fn current_best_move(&self) -> Move {
        let (moves, probabilities) = self.root_node.read().unwrap().get_current_policy();

        let idx = sample_index_weighted(&probabilities);

        moves[idx]
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

    valuation_fn: ValuationFn,
}

impl SearchWorker {
    pub fn new(
        thread_id: u64,
        root_node: SharedNode,
        search_active: Arc<AtomicBool>,
        valuation_fn: ValuationFn,
    ) -> Self {
        SearchWorker {
            thread_id,
            root_node,
            search_active,
            valuation_fn,
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
            self.mcts_iteration(self.root_node.clone());
        }

        println!("Search thread {} shutting down.", self.thread_id);
    }

    //
    fn mcts_iteration(&self, node: SharedNode) -> Valuation {
        // select next move
        let next_edge: Edge;

        // region in which node is read-locked
        {
            let node = node.read().unwrap();
            // each move with their respective value
            let (edges, values) = node.get_current_edge_values();
            // each move with their unnormalised probability

            let probabilities = softmax(&values, SELECTION_BETA);

            // let uni_prob = 1.0 / probabilities.len() as f32;

            // mix probability from edge value with uniform probability
            /* for p in probabilities.iter_mut() {
                *p = (1.0 - EPSILON) * (*p) + EPSILON * uni_prob;
            } */

            // sample from edges using probabilities as weights
            let idx = sample_index_weighted(&probabilities);
            next_edge = edges[idx].clone();
        }

        // backpropagated v from terminal node
        let v = match next_edge.child_node() {
            // we're an inner node => keep moving down the tree
            Some(next_node) => self.mcts_iteration(next_node),
            // child_node of edge does not exist => we've hit a leaf node => expand
            None => self.expand_node(Arc::clone(&node), next_edge.get_move()),
        };

        // lock node and update all its edges
        node.write().unwrap().update_with_value(v);

        v
    }

    fn expand_node(&self, parent_node: SharedNode, next_move: Move) -> Valuation {
        // write-lock parent_node for entire function
        let mut parent_node = parent_node.write().unwrap();

        let edge = parent_node.get_edge(next_move);
        if edge.child_node().is_some() {
            // race condition: 2 threads both selected the same node for expansion and called expand_node
            // this is the slower thread: the node is already expanded, so simply return its q_value

            return edge.q_value();
        }

        let mut next_board = parent_node.board().clone();
        let depth = parent_node.depth();

        next_board.apply_move(next_move);

        // let v = next_board.valuation();
        let v = (self.valuation_fn)(&next_board);

        let node = Node::new_shared(next_board, depth);

        parent_node.get_edge_mut(next_move).set_child_node(node);

        v
    }
}
