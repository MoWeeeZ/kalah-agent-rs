use std::sync::atomic::AtomicU64;
use std::sync::{Arc, RwLock, Weak};

use crate::kalah::valuation::Valuation;
use crate::util::math::softmax;
use crate::{Board, Move, Player};

pub type SharedNode = Arc<RwLock<Node>>;
pub type WeakSharedNode = Weak<RwLock<Node>>;

static mut NODES_ALLOCATED: AtomicU64 = AtomicU64::new(0);
static mut NODES_DEALLOCATED: AtomicU64 = AtomicU64::new(0);

// inverse temperature for next move selection
const POLICY_BETA: f32 = 1.0;

/*====================================================================================================================*/

pub struct Node {
    board: Board,

    edges: Vec<Edge>,

    depth: u64,
}

#[allow(dead_code)]
impl Node {
    pub fn new_shared(board: Board, depth: u64) -> SharedNode {
        unsafe {
            NODES_ALLOCATED.fetch_add(1, std::sync::atomic::Ordering::Release);
        }

        let mut node = Arc::new(RwLock::new(Node {
            board,
            // edge_list: EdgeList::new(),
            edges: Vec::new(),
            depth,
        }));

        Node::init_edges(&mut node);

        node
    }

    fn init_edges(node: &mut SharedNode) {
        let legal_moves = node.read().unwrap().board.legal_moves(Player::White);
        let mut edge_list = Vec::with_capacity(legal_moves.len());

        for legal_move in legal_moves.into_iter() {
            edge_list.push(Edge::new(Arc::downgrade(node), legal_move));
        }

        edge_list.shrink_to_fit();

        node.write().unwrap().edges = edge_list;
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn depth(&self) -> u64 {
        self.depth
    }

    pub fn get_edge(&self, move_: Move) -> &Edge {
        self.edges.iter().find(|edge| edge.from_move == move_).unwrap()
    }

    pub fn get_edge_mut(&mut self, move_: Move) -> &mut Edge {
        self.edges.iter_mut().find(|edge| edge.from_move == move_).unwrap()
    }

    pub fn get_current_edge_values(&self) -> (Vec<&Edge>, Vec<f32>) {
        todo!()
    }

    pub fn update_with_value(&mut self, _v: Valuation) {
        todo!();
    }

    pub fn get_current_policy(&self) -> (Vec<Move>, Vec<f32>) {
        let edge_len = self.edges.len();

        assert!(edge_len != 0, "No move available");

        let terminal_edges: Vec<&Edge> = self.edges.iter().filter(|edge| edge.is_terminal()).collect();
        let num_terminals = terminal_edges.len();

        if !terminal_edges.is_empty() {
            use Valuation::{TerminalDraw, TerminalWhiteWin};

            let best_terminal = terminal_edges
                .into_iter()
                .max_by(|&edge1, &edge2| edge1.q_value().cmp(&edge2.q_value()))
                .unwrap();

            let best_valuation = self.edges.iter().map(|edge| edge.q_value()).max().unwrap().as_f32();

            // if either:
            //  - the best (terminal and overall) move is a win for White (us)
            //  - we have not choice (since all edges are terminal)
            //  - all non-terminal edges are negative and we have a forced draw
            // return the best terminal move
            if matches!(best_terminal.q_value(), TerminalWhiteWin { .. })
                || num_terminals == edge_len
                || (best_valuation <= 0.0 && matches!(best_terminal.q_value(), TerminalDraw { .. }))
            {
                return (vec![best_terminal.get_move()], vec![1.0]);
            }
        }

        // terminal nodes handled: only consider non-terminals from now on

        let mut moves = Vec::with_capacity(edge_len - num_terminals);
        let mut visit_counts = Vec::with_capacity(edge_len - num_terminals);

        for edge in self.edges.iter().filter(|edge| !edge.is_terminal()) {
            moves.push(edge.get_move());
            // f32 can represent integers up to ~16,000,000 exact; should be enough precision
            visit_counts.push(edge.visit_count as f32);
        }

        let probabilities = softmax(&visit_counts, POLICY_BETA);

        (moves, probabilities)
    }

    pub fn get_child_node(&self, for_move: Move) -> Option<SharedNode> {
        self.edges
            .iter()
            .find(|&edge| edge.get_move() == for_move)
            .and_then(|edge| edge.child_node())
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        unsafe {
            NODES_DEALLOCATED.fetch_add(1, std::sync::atomic::Ordering::Release);
        }
    }
}

/*====================================================================================================================*/

#[allow(dead_code)]
#[derive(Clone)]
pub struct Edge {
    from_move: Move,
    parent_node: Weak<RwLock<Node>>,
    // child_node has to the option to be None, so they only get allocated then they actually get visited
    // important in particular when the tree becomes deeper so we don't allocate a lot of nodes that never get hit
    child_node: Option<SharedNode>,

    w_value: Valuation,
    visit_count: u64,
}

impl Edge {
    pub fn new(parent_node: WeakSharedNode, from_move: Move) -> Self {
        Edge {
            parent_node,
            from_move,
            child_node: None,
            w_value: Valuation::NonTerminal { value: 0.0 },
            visit_count: 0,
        }
    }

    pub fn get_move(&self) -> Move {
        self.from_move
    }

    pub fn child_node(&self) -> Option<SharedNode> {
        self.child_node.as_ref().map(Arc::clone)
    }

    pub fn set_child_node(&mut self, node: SharedNode) {
        assert!(self.child_node.is_none());

        self.child_node = Some(node);
    }

    pub fn q_value(&self) -> Valuation {
        self.w_value / self.visit_count as f32
    }

    pub fn is_terminal(&self) -> bool {
        self.w_value.is_terminal()
    }
}
