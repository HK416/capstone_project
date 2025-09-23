use crate::{
    collision::{Collider, StaticCollision},
    object3d::BoundingBox,
};


/// Deserialize로 불러올때 bounding_box를 생성하려면 `build_bounding_box`를 한번 호출해야 합니다.  
#[derive(Debug, Clone)]
pub struct ColliderTree {
    root: Box<Node>,
}

impl ColliderTree {
    pub fn load_from_json(json_buf: &[u8]) -> Result<Self, serde_json::Error> {
        let raw_colliders: RawColliderTree = serde_json::from_slice(json_buf)?;

        let mut root = None;

        let mut stack = Vec::with_capacity(16);
        stack.push((Dimension::X, &mut root, raw_colliders));

        while let Some((dim, node, raw_node)) = stack.pop() {
            let bound = raw_node.collider.create_bounding_box();
            let children_bound = BoundingBox::new(
                glam::Vec3::ZERO,
                glam::Vec3::ZERO
            );
            let new_node = Node {
                collider: raw_node.collider.clone(),
                bound,
                children_bound,
                dim: dim.clone(),
                left: None,
                right: None,
            };
            *node = Some(Box::new(new_node));
            
            let Node { left, right, .. } = &mut **node.as_mut().unwrap();
            if let Some(raw_right) = raw_node.right {
                stack.push((dim.next(), right, *raw_right));
            }
            if let Some(raw_left) = raw_node.left {
                stack.push((dim.next(), left, *raw_left));
            }
        }

        let mut root = root.unwrap();
        root.build_children_bound();

        Ok(Self { root })
    }

    /// `bound`와 AABB 충돌하는 모든 Collider를 반환합니다.
    pub fn search_aabb_collision(&self, bound: BoundingBox) -> Vec<&Collider> {
        let bound_min = bound.center - bound.extents();
        let bound_max = bound.center + bound.extents();

        let mut result = Vec::with_capacity(16);        
        let mut stack = Vec::with_capacity(16);
        stack.push(&*self.root);

        while let Some(node) = stack.pop() {
            if node.bound.check_static_collision(&bound) {
                result.push(&node.collider);
            }

            let axis = node.dim.clone() as usize;

            if let Some(left) = &node.left {
                let left_bound = left.children_bound;
                let left_max = left_bound.center[axis] + left_bound.extents()[axis];
                if bound_min[axis] <= left_max {
                    stack.push(left);
                }
            }
            if let Some(right) = &node.right {
                let right_bound = right.children_bound;
                let right_min = right_bound.center[axis] - right_bound.extents()[axis];
                if bound_max[axis] >= right_min {
                    stack.push(right);
                }
            }
        }

        result
    }

    /// `point`가 포함된 모든 Collider를 반환합니다.
    pub fn search_point_collision(&self, point: &glam::Vec3) -> Vec<&Collider> {
        let mut result = Vec::with_capacity(16);
        
        let mut stack = Vec::with_capacity(16);
        stack.push(&*self.root);

        while let Some(node) = stack.pop() {
            if !node.children_bound.check_point_collision(&point) {
                continue;
            }

            if node.bound.check_point_collision(&point) {
                result.push(&node.collider);
            }

            if let Some(left) = &node.left {
                stack.push(left);
            }
            if let Some(right) = &node.right {
                stack.push(right);
            }
        }

        result
    }

    pub fn count_colliders(&self) -> usize {
        let mut count = 0;
        let mut stack = Vec::with_capacity(16);
        stack.push(&*self.root);

        while let Some(node) = stack.pop() {
            count += 1;

            if let Some(left) = &node.left {
                stack.push(left);
            }
            if let Some(right) = &node.right {
                stack.push(right);
            }
        }

        count
    }
}

#[derive(Debug, Clone)]
struct Node {
    collider: Collider,
    bound: BoundingBox,
    /// 자식 노드를 전부 감싸는 경계 박스
    children_bound: BoundingBox,
    /// 기준 차원
    dim: Dimension,
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
}

impl Node {
    fn build_children_bound(&mut self) {
        if self.left.is_none() && self.right.is_none() {
            self.children_bound = self.bound.clone();
        }

        let mut min = self.bound.center - self.bound.extents();
        let mut max = self.bound.center + self.bound.extents();
        if let Some(left) = &mut self.left {
            if left.children_bound.extents() == glam::Vec3::ZERO {
                left.build_children_bound();
            }
            let left_bound = left.children_bound;
            let left_min = left_bound.center - left_bound.extents();
            let left_max = left_bound.center + left_bound.extents();
            min = min.min(left_min);
            max = max.max(left_max);
        }
        if let Some(right) = &mut self.right {
            if right.children_bound.extents() == glam::Vec3::ZERO {
                right.build_children_bound();
            }
            let right_bound = right.children_bound;
            let right_min = right_bound.center - right_bound.extents();
            let right_max = right_bound.center + right_bound.extents();
            min = min.min(right_min);
            max = max.max(right_max);
        }

        self.children_bound = BoundingBox::from_start_end(min, max);
    }
}

#[derive(Debug, Clone)]
enum Dimension {
    X,
    Y,
    Z,
}

impl Dimension {
    fn next(&self) -> Self {
        match self {
            Dimension::X => Dimension::Y,
            Dimension::Y => Dimension::Z,
            Dimension::Z => Dimension::X,
        }
    }
}

#[derive(serde::Deserialize)]
struct RawColliderTree {
    collider: Collider,
    left: Option<Box<RawColliderTree>>,
    right: Option<Box<RawColliderTree>>,
}
