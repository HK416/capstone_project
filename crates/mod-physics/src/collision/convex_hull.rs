use std::collections::BinaryHeap;
use crate::{
    bounds::BoundingBox,
    capsule::Capsule,
    sphere::Sphere,
    CollisionDetails,
};


pub trait ConvexHull {
    /// 도형에 속하는 점 중 direction 방향으로 가장 먼 점을 반환한다.  
    fn get_furthest_point(&self, direction: &glam::Vec3A) -> glam::Vec3A;

    /// 두 도형의 Minkowski 차의 Support Point를 구한다.  
    fn get_support(&self, other: &impl ConvexHull, direction: &glam::Vec3A) -> glam::Vec3A {
        self.get_furthest_point(direction) - other.get_furthest_point(&-direction)
    }

    /// 충돌거리가 0인경우(접하는 경우)에도 Some을 반환한다.  
    fn gjk(&self, other: &impl ConvexHull) -> Option<Simplex> {
        let mut simplex = Simplex {
            vertices: [glam::Vec3A::ZERO; 4],
            count: 0,
        };

        // 1. 임의의 방향에 대한 support point를 구한다.
        let mut direction = glam::Vec3A::X;
        simplex.vertices[0] = self.get_support(other, &direction);
        
        // 2. -support방향에 대한 support point를 구한다.
        direction = match simplex.vertices[0].try_normalize() {
            Some(dir) => -dir,
            // support point가 원점인 경우 = 두 도형이 접함
            None => {
                simplex.count = 1;
                return Some(simplex);
            }
        };
        simplex.vertices[1] = self.get_support(other, &direction);
        if simplex.vertices[1].dot(direction) < 0.0 {
            return None;
        }

        // 3. 두 support point를 잇는 직선에서 원점을 향하는 방향에 대한 support point를 구한다.
        let cross = simplex.vertices[0].cross(simplex.vertices[1]);
        let v = simplex.vertices[1] - simplex.vertices[0];
        if cross == glam::Vec3A::ZERO {     // 원점과 두 support point가 한 직선 상에 있는 경우
            direction = glam::Vec3A::Y.cross(v);
            if direction == glam::Vec3A::ZERO {
                direction = glam::Vec3A::Z.cross(v);
            }
            direction = direction.normalize();
            simplex.vertices[2] = self.get_support(other, &direction);
            if simplex.vertices[2] == simplex.vertices[0] || simplex.vertices[2] == simplex.vertices[1] {
                direction = -direction;
                simplex.vertices[2] = self.get_support(other, &direction);
            }
        }
        else {
            direction = cross.cross(v);
            direction = direction.normalize();
            simplex.vertices[2] = self.get_support(other, &direction);
        }
        if simplex.vertices[2].dot(direction) < 0.0 {
            return None;
        }

        // 4. 세 support point가 만드는 평면에서 원점을 향하는 방향에 대한 support point를 구한다.
        let normal = (simplex.vertices[1] - simplex.vertices[0]).cross(simplex.vertices[2] - simplex.vertices[0]);
        if normal.dot(simplex.vertices[0]) < 0.0 {
            direction = normal;
            // CCW로 정렬
            (simplex.vertices[1], simplex.vertices[2]) = (simplex.vertices[2], simplex.vertices[1]);
        } else {
            direction = -normal;
        }
        direction = direction.normalize();
        simplex.vertices[3] = self.get_support(other, &direction);
        if simplex.vertices[3].dot(direction) < 0.0 {
            return None;
        }

        // 5. Simplex가 원점을 포함할 때까지 반복한다.
        while let Some(face) = simplex.get_nearest_if_not_contains_origin() {
            simplex.vertices = [
                // CCW로 정렬
                simplex.vertices[face.vertices[0]], 
                simplex.vertices[face.vertices[2]], 
                simplex.vertices[face.vertices[1]], 
                self.get_support(other, &face.normal)
            ];
            if simplex.vertices[3].dot(face.normal) < 0.0 {
                return None;
            }
        }

        simplex.count = 4;
        Some(simplex)
    }

    fn gjk_epa(&self, other: &impl ConvexHull) -> Option<CollisionDetails> {
        let simplex = self.gjk(other)?;
        if simplex.count <= 1 {
            return Some(CollisionDetails {
                normal: glam::Vec3A::ZERO,
                penetration: 0.0,
            });
        }

        // 1. simplex의 모든 면에 대해 원점과의 거리를 구한다.
        let mut polytope = Vec::from(simplex.vertices);
        let indices = [
            // CCW
            [0, 1, 2],
            [3, 1, 0],
            [3, 2, 1],
            [3, 0, 2],
        ];
        // 원점은 무조건 simplex 안에 있다.
        let faces = Face::vec(&polytope, &indices);
        let mut faces = faces.into_iter()
            .filter_map(|f| f)
            .collect::<BinaryHeap<_>>();

        loop {
            // 2. 최근접면의 법선벡터 방향으로 polytope를 확장한다.
            // 2-1. 최근접면을 찾고 법선벡터 방향으로 support point를 구한다.
            let nearest_face = match faces.peek() {
                Some(face) => face,
                None => {
                    println!("polytope: {:?}", polytope);
                    println!("faces: {:?}", faces);
                    panic!("No nearest face");
                }
            };
            let collision_info = CollisionDetails {
                normal: nearest_face.normal,
                penetration: nearest_face.distance,
            };
            let support = self.get_support(other, &nearest_face.normal);
            let distance = nearest_face.normal.dot(support);
            // 그런 simplex가 없다면 리턴
            if (distance - nearest_face.distance).abs() < 0.0001 {
                return Some(collision_info);
            }
            polytope.push(support);
            let idx = polytope.len() - 1;
            
            // 2-2. O to support 벡터와 방향이 같은 모든 면을 제거한다.
            let same_direction_faces;
            (same_direction_faces, faces) = faces.iter()
                .partition(|f| {
                    let d = support - polytope[f.vertices[0]];
                    f.normal.dot(d) > 0.0
                });

            // 2-3. 새로운 면을 만든다.
            let edges = same_direction_faces.iter()
                .map(|f| [
                    [f.vertices[0], f.vertices[1]],
                    [f.vertices[1], f.vertices[2]],
                    [f.vertices[2], f.vertices[0]],
                ])
                .flatten();
            let mut unique_edges = Vec::new();
            // O(n^2)
            for edge in edges {
                let rev_edge = [edge[1], edge[0]];
                let pos = unique_edges.iter().position(|e| *e == rev_edge);
                if let Some(i) = pos {
                    unique_edges.remove(i);
                } else {
                    unique_edges.push(edge);
                }
            }
            let new_face_indices = unique_edges.iter()
                .map(|edge| [edge[0], edge[1], idx])    // CCW
                .collect::<Vec<_>>();
            let new_faces = Face::vec(&polytope, &new_face_indices)
                .into_iter()
                .filter_map(|f| f)
                .collect::<Vec<_>>();
            if new_faces.is_empty() {
                return Some(collision_info);
            }

            // 3. 새로 만들어진 면들을 Heap에 추가한다.
            for face in new_faces {
                faces.push(face);
            }
        }
    }
}

impl ConvexHull for BoundingBox {
    fn get_furthest_point(&self, direction: &glam::Vec3A) -> glam::Vec3A {
        self.get_vertices().iter()
            .max_by(|&a, &b| direction.dot(*a).partial_cmp(&direction.dot(*b)).unwrap())
            .copied()
            .unwrap()
    }
}

impl ConvexHull for Capsule {
    fn get_furthest_point(&self, direction: &glam::Vec3A) -> glam::Vec3A {
        let seg = self.get_seg();
        let start = glam::Vec3A::from(seg.start);
        let end = glam::Vec3A::from(seg.end);
        let dot_start = direction.dot(start);
        let dot_end = direction.dot(end);
        if dot_start > dot_end {
            start + direction * self.radius
        } else {
            end + direction * self.radius
        }
    }
}

impl ConvexHull for Sphere {
    fn get_furthest_point(&self, direction: &glam::Vec3A) -> glam::Vec3A {
        glam::Vec3A::from(self.center) + direction * self.radius
    }
}



#[derive(Debug, Clone, Copy)]
struct Face {
    /// 평면을 이루는 세 점의 인덱스
    vertices: [usize; 3],
    /// 평면의 법선벡터
    normal: glam::Vec3A,
    /// 원점으로부터의 거리
    distance: f32,
}

impl Face {
    fn vec(vertices: &[glam::Vec3A], indices: &[[usize; 3]]) -> Vec<Option<Self>> {
        let mut faces = Vec::with_capacity(indices.len());
        for idx in indices {
            let v = [
                vertices[idx[1]] - vertices[idx[0]],
                vertices[idx[2]] - vertices[idx[0]],
            ];
            match v[0].cross(v[1]).try_normalize() {
                Some(normal) => {
                    let distance = normal.dot(vertices[idx[0]]);
                    faces.push(Some(Face {
                        vertices: *idx,
                        normal,
                        distance,
                    }));
                },
                None => {
                    faces.push(None);
                }
            };
        }
        faces
    }
}

/////////////////////// for BinaryHeap ///////////////////////
// Min Heap으로 사용하기 위해 -distance로 비교한다.

impl PartialEq for Face {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance
    }
}

impl Eq for Face {}

impl PartialOrd for Face {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (-self.distance).partial_cmp(&-other.distance)
    }
}

impl Ord for Face {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (-self.distance).partial_cmp(&-other.distance).unwrap()
    }
}

//////////////////////////////////////////////////////////////


/// 바닥면을 이루는 세 점과 나머지 한 점 순으로 저장하고,  
/// 바닥면의 법선벡터는 항상 원점 반대방향이어야 한다. (CCW)  
#[derive(Debug)]
pub struct Simplex {
    vertices: [glam::Vec3A; 4],
    /// 유효한 점의 개수
    count: usize,
}

impl Simplex {
    /// Simplex 안쪽에 원점이 있는지 확인하고,  
    /// 그렇지 않다면 원점과 가장 가까운 면을 구한다.  
    /// (바닥면은 검사하지 않는다.)  
    fn get_nearest_if_not_contains_origin(&self) -> Option<Face> {
        // CCW
        let indices = [
            [3, 1, 0],
            [3, 2, 1],
            [3, 0, 2],
        ];
        let faces = Face::vec(&self.vertices, &indices);

        let mut min_distance = f32::MAX;
        let mut nearest_face = None;

        faces.into_iter()
            .filter_map(|f| f)
            .for_each(|face| {
                let distance = -face.distance;
                if 0.0 < distance && distance < min_distance {
                    min_distance = distance;
                    nearest_face = Some(face);
                }
            });

        nearest_face
    }
}