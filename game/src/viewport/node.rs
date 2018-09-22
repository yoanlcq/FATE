use std::cell::RefCell;
use super::{Split, LeafViewport, ViewportNodeID};

#[derive(Debug, Clone, PartialEq)]
pub struct ViewportNode {
    pub parent: Option<ViewportNodeID>,
    pub value: ViewportNodeValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewportNodeValue {
    Leaf(RefCell<LeafViewport>),
    Split {
        split: Split,
        children: (ViewportNodeID, ViewportNodeID),
    },
}

impl ViewportNode {
    pub fn new_root(leaf: LeafViewport) -> Self {
        Self {
            parent: None,
            value: ViewportNodeValue::Leaf(leaf.into()),
        }
    }
}
impl ViewportNodeValue {
    pub fn unwrap_leaf(&self) -> &RefCell<LeafViewport> {
        match *self {
            ViewportNodeValue::Leaf(ref leaf) => leaf,
            _ => panic!("This viewport node is not a leaf"),
        }
    }
}
