use std::marker::PhantomData;

use crate::{Board, Move, Player};

pub struct Node {
    board: Board,
    pre_move: Move,

    sibling: Option<Box<Node>>,
    children: Option<Box<Node>>,

    depth: u64,

    pub value: f32,
}

impl Node {
    pub fn new(board: Board, pre_move: Move, depth: u64) -> Self {
        Node {
            board,
            pre_move,
            sibling: None,
            children: None,
            depth,
            value: f32::NEG_INFINITY,
        }
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn pre_move(&self) -> Move {
        self.pre_move
    }

    pub fn depth(&self) -> u64 {
        self.depth
    }

    pub fn colour(&self) -> Player {
        if self.board.flipped() {
            Player::Black
        } else {
            Player::White
        }
    }

    // lot less efficient than pointer base solution, but should be fine since we should never have more than ~ 20 children and that should still be cachable
    pub fn append_child(&mut self, child_node: Box<Node>) {
        match self.child_iter_mut().last() {
            Some(last_child) => last_child.sibling = Some(child_node),
            None => self.children = Some(child_node),
        }
    }

    pub fn has_children(&self) -> bool {
        self.children.is_some()
    }

    pub fn child_iter(&self) -> NodeChildIter {
        NodeChildIter {
            next_node: self.children.as_deref(),
        }
    }

    pub fn child_iter_mut(&mut self) -> NodeChildIterMut {
        NodeChildIterMut {
            next_node: self.children.as_deref_mut().map(|children| children as *mut Node),
            _boo: PhantomData,
        }
    }
}

pub struct NodeChildIter<'a> {
    next_node: Option<&'a Node>,
}

impl<'a> Iterator for NodeChildIter<'a> {
    type Item = &'a Node;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_node?;

        let ret_node = self.next_node.unwrap();

        self.next_node = ret_node.sibling.as_deref();

        Some(ret_node)
    }
}

pub struct NodeChildIterMut<'a> {
    next_node: Option<*mut Node>,
    _boo: PhantomData<&'a mut Node>,
}

impl<'a> Iterator for NodeChildIterMut<'a> {
    type Item = &'a mut Node;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_node?;

        unsafe {
            let ret_node = &mut *self.next_node.unwrap();

            self.next_node = ret_node.sibling.as_deref_mut().map(|next_node| next_node as *mut Node);

            Some(ret_node)
        }
    }
}
