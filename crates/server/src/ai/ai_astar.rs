use glam::Vec3A;
use std::collections::{BinaryHeap, HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub struct Node3D {
    pub pos: Vec3A,
    pub g: f32, // cost from start
    pub h: f32, // heuristic to goal
    pub f: f32, // total cost
    pub parent: Option<Vec3A>,
}

impl Eq for Node3D {}

impl PartialOrd for Node3D {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // BinaryHeap is max-heap, so reverse order for min-heap
        other.f.partial_cmp(&self.f)
    }
}

impl Ord for Node3D {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

/// 3D A* 경로탐색
/// - start: 시작 좌표
/// - goal: 목표 좌표
/// - step: 한 번에 이동할 거리(격자 크기)
/// - is_walkable: 해당 위치가 이동 가능한지 판정하는 함수
///
/// 반환: 경로(waypoints) 리스트, 실패시 None
pub fn astar_pathfind_vec3a<F>(
    start: Vec3A,
    goal: Vec3A,
    step: f32,
    mut is_walkable: F,
) -> Option<Vec<Vec3A>>
where
    F: FnMut(Vec3A) -> bool,
{
    let mut open = BinaryHeap::new();
    let mut closed: HashSet<[i32; 3]> = HashSet::new();
    let mut nodes: HashMap<[i32; 3], Node3D> = HashMap::new();

    let start_idx = to_idx(start, step);
    let goal_idx = to_idx(goal, step);

    let h = (goal - start).length();
    let start_node = Node3D { pos: start, g: 0.0, h, f: h, parent: None };
    open.push(start_node.clone());
    nodes.insert(start_idx, start_node);

    let directions = neighbor_directions();

    while let Some(current) = open.pop() {
        let curr_idx = to_idx(current.pos, step);
        if curr_idx == goal_idx || (current.pos - goal).length() < step {
            // 경로 복원
            let mut path = vec![current.pos];
            let mut p = current.parent;
            while let Some(prev) = p {
                path.push(prev);
                let prev_idx = to_idx(prev, step);
                p = nodes.get(&prev_idx).and_then(|n| n.parent);
            }
            path.reverse();
            return Some(path);
        }
        closed.insert(curr_idx);
        for dir in &directions {
            let next_pos = current.pos + *dir * step;
            let next_idx = to_idx(next_pos, step);
            if closed.contains(&next_idx) { continue; }
            if !is_walkable(next_pos) { continue; }
            let g = current.g + step;
            let h = (goal - next_pos).length();
            let f = g + h;
            let next_node = Node3D {
                pos: next_pos,
                g,
                h,
                f,
                parent: Some(current.pos),
            };
            if let Some(existing) = nodes.get(&next_idx) {
                if g < existing.g {
                    nodes.insert(next_idx, next_node.clone());
                    open.push(next_node);
                }
            } else {
                nodes.insert(next_idx, next_node.clone());
                open.push(next_node);
            }
        }
    }
    None
}

fn to_idx(pos: Vec3A, step: f32) -> [i32; 3] {
    [
        (pos.x / step).round() as i32,
        (pos.y / step).round() as i32,
        (pos.z / step).round() as i32,
    ]
}

fn neighbor_directions() -> Vec<Vec3A> {
    let mut dirs = vec![];
    for &dx in &[-1.0, 0.0, 1.0] {
        for &dy in &[-1.0, 0.0, 1.0] {
            for &dz in &[-1.0, 0.0, 1.0] {
                if dx == 0.0 && dy == 0.0 && dz == 0.0 { continue; }
                dirs.push(Vec3A::new(dx, dy, dz));
            }
        }
    }
    dirs
}
