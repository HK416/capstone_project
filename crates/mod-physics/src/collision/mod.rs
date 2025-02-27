use super::{
    bounds::BoundingBox,
    capsule::Capsule,
    sphere::Sphere,
};


pub enum Collider {
    Box(BoundingBox),
    Capsule(Capsule),
    Sphere(Sphere),
}

impl Collider {
    pub fn check_collision(&self, other: &Self) -> bool {
        todo!()
    }

    pub fn check_collision_details(&self, other: &Self) -> Option<CollisionDetails> {
        todo!()
    }
}

pub fn check_collision(a: &impl ConvexHull, b: &impl ConvexHull) -> bool {
    a.gjk(b)
}


pub struct CollisionDetails {
    pub normal: glam::Vec3,
    pub penetration: f32,
    pub contact_point: glam::Vec3,
}


struct Face {
    vertices: [usize; 3],
    normal: glam::Vec3A,
}

/// 바닥면을 이루는 세 점과 나머지 한 점 순으로 저장하고,  
/// 바닥면의 법선벡터는 항상 원점 반대방향이어야 한다. (CCW)  
struct Simplex {
    vertices: [glam::Vec3A; 4],
}

impl Simplex {
    /// Simplex 안쪽에 원점이 있는지 확인하고,  
    /// 그렇지 않다면 원점과 가장 가까운 면을 구한다.  
    /// (바닥면은 검사하지 않는다.)  
    fn get_nearest_if_not_contains_origin(&self) -> Option<Face> {
        // CCW
        let faces = [
            [3, 1, 0],
            [3, 2, 1],
            [3, 0, 2],
        ];
        let v = [
            self.vertices[0] - self.vertices[3],
            self.vertices[1] - self.vertices[3],
            self.vertices[2] - self.vertices[3],
        ];
        let n = [
            v[1].cross(v[0]).normalize(),
            v[2].cross(v[1]).normalize(),
            v[0].cross(v[2]).normalize(),
        ];
        let dist = [
            -n[0].dot(self.vertices[3]),
            -n[1].dot(self.vertices[3]),
            -n[2].dot(self.vertices[3]),
        ];

        let mut min_distance = f32::MAX;
        let mut nearest_face = None;

        for i in 0..3 {
            if 0.0 <= dist[i] && dist[i] < min_distance {
                min_distance = dist[i];
                nearest_face = Some(Face {
                    vertices: faces[i],
                    normal: n[i],
                });
            }
        }

        nearest_face
    }
}


pub trait ConvexHull {
    /// 도형에 속하는 점 중 direction 방향으로 가장 먼 점을 반환한다.  
    /// 
    /// direction은 단위 벡터여야 한다.  
    fn get_furthest_point(&self, direction: &glam::Vec3A) -> glam::Vec3A;

    /// 두 도형의 Minkowski 차의 Support Point를 구한다.  
    fn get_support(&self, other: &impl ConvexHull, direction: &glam::Vec3A) -> glam::Vec3A {
        self.get_furthest_point(direction) - other.get_furthest_point(&-direction)
    }

    /// 충돌거리가 0인경우(접하는 경우)에도 true를 반환한다.  
    fn gjk(&self, other: &impl ConvexHull) -> bool {
        let mut simplex = Simplex {
            vertices: [glam::Vec3A::ZERO; 4],
        };

        // 1. 임의의 방향에 대한 support point를 구한다.
        let mut direction = glam::Vec3A::X;
        simplex.vertices[0] = self.get_support(other, &direction);

        // 2. -support방향에 대한 support point를 구한다.
        direction = match simplex.vertices[0].try_normalize() {
            Some(dir) => -dir,
            None => return true,    // 원점과 접함 == 두 도형이 접함
        };
        simplex.vertices[1] = self.get_support(other, &direction);
        if simplex.vertices[1].dot(direction) < 0.0 {
            return false;
        }

        // 3. 두 support point를 잇는 직선에서 원점을 향하는 방향에 대한 support point를 구한다.
        let cross = simplex.vertices[0].cross(simplex.vertices[1]);
        // 원점과 두 support point가 한 직선 상에 있는 경우
        if cross == glam::Vec3A::ZERO {
            // 원점은 무조건 두 support point 사이에 있으므로(2번째 단계에서의 조건문에 의해) 충돌
            return true;
        }
        direction = cross.cross(simplex.vertices[1] - simplex.vertices[0]);
        direction = direction.normalize();
        simplex.vertices[2] = self.get_support(other, &direction);
        if simplex.vertices[2].dot(direction) < 0.0 {
            return false;
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
            return false;
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
                return false;
            }
        }

        true
    }
}

impl ConvexHull for BoundingBox {
    fn get_furthest_point(&self, direction: &glam::Vec3A) -> glam::Vec3A {
        let vertices = self.get_vertices();
        let mut max = direction.dot(vertices[0]);
        let mut index = 0;
        for i in 1..vertices.len() {
            let dot = direction.dot(vertices[i]);
            if dot > max {
                max = dot;
                index = i;
            }
        }
        vertices[index]
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
