use std::sync::atomic::AtomicU64;
use std::sync::{Arc, RwLock, Weak};

use crate::{Board, Move, Player};

pub type SharedNode = Arc<RwLock<Node>>;
pub type WeakSharedNode = Weak<RwLock<Node>>;

static mut NODE_COUNT: AtomicU64 = AtomicU64::new(0);

/*====================================================================================================================*/

pub struct Node {
    board: Board,

    edge_list: EdgeList,
}

#[allow(dead_code)]
impl Node {
    pub fn new_shared(board: Board) -> SharedNode {
        unsafe {
            NODE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Release);
        }

        let mut node = Arc::new(RwLock::new(Node {
            board,
            edge_list: EdgeList::new(),
        }));

        Node::init_edges(&mut node);

        node
    }

    fn init_edges(node: &mut SharedNode) {
        let legal_moves = node.read().unwrap().board.legal_moves(Player::White);
        let mut edge_list = EdgeList::with_capacity(legal_moves.len());

        for legal_move in legal_moves.into_iter() {
            edge_list.push(Edge::new(Arc::downgrade(node), legal_move));
        }
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn is_leaf(&self) -> bool {
        self.edge_list.is_empty()
    }

    pub fn get_current_edge_values(&self) -> Vec<(&Edge, f64)> {
        todo!()
    }

    pub fn update_with_value(&mut self, _v: f64) {
        todo!();
    }

    pub fn get_current_policy(&self) -> Vec<(Move, f64)> {
        todo!()
    }

    pub fn get_child_node(&self, move_: Move) -> Option<SharedNode> {
        self.edge_list
            .iter()
            .find(|&edge| edge.get_move() == move_)
            .map(|edge| edge.child_node())
            .map(|node| node.unwrap())
    }
}

/*====================================================================================================================*/

#[allow(dead_code)]
#[derive(Clone)]
pub struct Edge {
    parent_node: Weak<RwLock<Node>>,
    move_: Move,
    // child_node has to the option to be None, so they only get allocated then they actually get visited
    // important in particular when the tree becomes deeper so we don't allocate a lot of nodes that never get hit
    child_node: Option<SharedNode>,

    total_value: f64,
    visit_count: u64,
}

impl Edge {
    pub fn new(parent_node: WeakSharedNode, move_: Move) -> Self {
        Edge {
            parent_node,
            move_,
            child_node: None,
            total_value: 0.0,
            visit_count: 0,
        }
    }

    pub fn get_move(&self) -> Move {
        self.move_
    }

    pub fn child_node(&self) -> Option<SharedNode> {
        self.child_node.as_ref().map(Arc::clone)
    }
}

/*====================================================================================================================*/

// wrapper for Vec<Edge> that makes sure the Vec only grows its buffer by 1 every time
struct EdgeList {
    edges: Vec<Edge>,
}

#[allow(dead_code)]
impl EdgeList {
    pub fn new() -> Self {
        EdgeList { edges: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        EdgeList {
            edges: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, edge: Edge) {
        self.edges.reserve_exact(1);
        assert!(
            self.edges.capacity() == self.edges.len() + 1,
            "edges vec has len {}, but capacity {}",
            self.edges.len(),
            self.edges.capacity()
        );
        self.edges.push(edge);
    }

    pub fn len(&self) -> usize {
        self.edges.len()
    }

    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Edge> {
        self.edges.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator + '_ {
        self.edges.iter_mut()
    }
}

impl<I: std::slice::SliceIndex<[Edge]>> std::ops::Index<I> for EdgeList {
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.edges[index]
    }
}

impl<I: std::slice::SliceIndex<[Edge]>> std::ops::IndexMut<I> for EdgeList {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.edges[index]
    }
}
