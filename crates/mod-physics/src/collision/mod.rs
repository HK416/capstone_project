mod convex_hull;
mod collider;
mod static_collision;
mod dynamic_collision;
mod ray_intersect;

pub use convex_hull::ConvexHull;
pub use collider::Collider;
pub use static_collision::{StaticCollision, StaticCollisionDetails};
pub use dynamic_collision::{DynamicCollision, DynamicCollisionDetails};
pub use ray_intersect::{Ray, RayIntersect, RayIntersectInfo};

use std::collections::VecDeque;


#[derive(Debug, Clone, serde::Deserialize)]
pub struct ColliderTree {
    pub collider: Collider,
    pub left: Option<Box<ColliderTree>>,
    pub right: Option<Box<ColliderTree>>,
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
    type Item = &'a Collider;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.stack.pop_back() {
            if let Some(right) = &node.right {
                self.stack.push_back(right);
            }
            if let Some(left) = &node.left {
                self.stack.push_back(left);
            }
            return Some(&node.collider);
        }
        None
    }
}