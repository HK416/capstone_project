use std::collections::VecDeque;

use crate::{
    collision::Collider,  
    object3d::BoundingBox,
};


/// Deserialize로 불러올때 bounding_box를 생성하려면 `build_bounding_box`를 한번 호출해야 합니다.  
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ColliderTree {
    pub collider: Collider,
    pub left: Option<Box<ColliderTree>>,
    pub right: Option<Box<ColliderTree>>,
    #[serde(skip)]
    pub bounding_box: BoundingBox,
}

impl ColliderTree {
    /// Deserialize로 불러온 후 한번 호출하여 collider의 bounding_box를 생성합니다.  
    pub fn build_bounding_box(&mut self) {
        let mut stack = VecDeque::with_capacity(100);
        stack.push_back(self);

        while let Some(node) = stack.pop_back() {
            node.bounding_box = node.collider.create_bounding_box();

            if let Some(right) = &mut node.right {
                stack.push_back(right.as_mut());
            }
            if let Some(left) = &mut node.left {
                stack.push_back(left.as_mut());
            }
        }
    }
}

/// DFS iterator
pub struct ColliderTreeIterator<'a> {
    stack: VecDeque<&'a ColliderTree>,
}

impl<'a> ColliderTreeIterator<'a> {
    pub fn new(root: &'a ColliderTree) -> Self {
        let mut stack = VecDeque::new();
        stack.push_back(root);
        Self { stack }
    }
}

impl<'a> Iterator for ColliderTreeIterator<'a> {
    type Item = (&'a Collider, &'a BoundingBox);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.stack.pop_back() {
            if let Some(right) = &node.right {
                self.stack.push_back(right);
            }
            if let Some(left) = &node.left {
                self.stack.push_back(left);
            }
            return Some((&node.collider, &node.bounding_box));
        }
        None
    }
}