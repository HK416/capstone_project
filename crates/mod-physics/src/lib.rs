pub mod rigid_body;

mod bounds;
pub use bounds::*;

mod sphere;
pub use sphere::*;

mod capsule;
pub use capsule::*;

mod ray;
pub use ray::*;

mod collision;
pub use collision::*;


pub trait Collision {
    /// ## 충돌체크시 고려사항
    /// 
    /// ### 캡슐이 UFO형태인 경우 정상 동작 여부
    /// 
    /// | | Capsule | YCapsule |
    /// |---|:---:|:---:|
    /// | Sphere | O | O |
    /// | Capsule | X | X |
    /// | YCapsule | X | X |
    /// | BoundingBox | X | X |
    /// 
    /// - O: 정상동작  
    /// - X: 정상동작을 보장하지 않음
    fn check_collision(&self, other: &dyn Collision) -> bool;

    fn check_sphere_collision(&self, sphere: &Sphere) -> bool;
    fn check_boundingbox_collision(&self, boundingbox: &BoundingBox) -> bool;
    fn check_capsule_collision(&self, capsule: &Capsule) -> bool;
    fn check_ycapsule_collision(&self, ycapsule: &YCapsule) -> bool;
}


impl Collision for Sphere {
    fn check_collision(&self, other: &dyn Collision) -> bool {
        other.check_sphere_collision(self)
    }

    fn check_sphere_collision(&self, sphere: &Sphere) -> bool {
        check_sphere_sphere_collision(self, sphere)
    }

    fn check_boundingbox_collision(&self, boundingbox: &BoundingBox) -> bool {
        check_boundingbox_sphere_collision(boundingbox, self)
    }

    fn check_capsule_collision(&self, capsule: &Capsule) -> bool {
        check_sphere_capsule_collision(self, capsule)
    }

    fn check_ycapsule_collision(&self, ycapsule: &YCapsule) -> bool {
        check_sphere_ycapsule_collision(self, ycapsule)
    }
}

impl Collision for BoundingBox {
    fn check_collision(&self, other: &dyn Collision) -> bool {
        other.check_boundingbox_collision(self)
    }

    fn check_sphere_collision(&self, sphere: &Sphere) -> bool {
        check_boundingbox_sphere_collision(self, sphere)
    }

    fn check_boundingbox_collision(&self, boundingbox: &BoundingBox) -> bool {
        check_boundingbox_boundingbox_collision(self, boundingbox)
    }

    fn check_capsule_collision(&self, capsule: &Capsule) -> bool {
        check_boundingbox_capsule_collision(self, capsule)
    }

    fn check_ycapsule_collision(&self, ycapsule: &YCapsule) -> bool {
        check_boundingbox_ycapsule_collision(self, ycapsule)
    }
}

impl Collision for Capsule {
    fn check_collision(&self, other: &dyn Collision) -> bool {
        other.check_capsule_collision(self)
    }

    fn check_sphere_collision(&self, sphere: &Sphere) -> bool {
        check_sphere_capsule_collision(sphere, self)
    }

    fn check_boundingbox_collision(&self, boundingbox: &BoundingBox) -> bool {
        check_boundingbox_capsule_collision(boundingbox, self)
    }

    fn check_capsule_collision(&self, capsule: &Capsule) -> bool {
        check_capsule_capsule_collision(self, capsule)
    }

    fn check_ycapsule_collision(&self, ycapsule: &YCapsule) -> bool {
        check_capsule_ycapsule_collision(self, ycapsule)
    }
}

impl Collision for YCapsule {
    fn check_collision(&self, other: &dyn Collision) -> bool {
        other.check_ycapsule_collision(self)
    }

    fn check_sphere_collision(&self, sphere: &Sphere) -> bool {
        check_sphere_ycapsule_collision(sphere, self)
    }
    
    fn check_boundingbox_collision(&self, boundingbox: &BoundingBox) -> bool {
        check_boundingbox_ycapsule_collision(boundingbox, self)
    }

    fn check_capsule_collision(&self, capsule: &Capsule) -> bool {
        check_capsule_ycapsule_collision(capsule, self)
    }

    fn check_ycapsule_collision(&self, ycapsule: &YCapsule) -> bool {
        check_ycapsule_ycapsule_collision(self, ycapsule)
    }
}



/// Ray와 다른 객체가 충돌하는지 검사, 충돌한다면 가장 가까운 충돌 지점까지의 거리를 반환한다.
pub trait RayIntersect {
    fn ray_intersect(&self, ray: &Ray) -> Option<f32>;
}


impl RayIntersect for Sphere {
    fn ray_intersect(&self, ray: &Ray) -> Option<f32> {
        let center = glam::Vec3A::from(self.center);
        let origin = glam::Vec3A::from(ray.origin);
        let origin_to_center = center - origin;
        
        let radius_sq = self.radius.powi(2);
        let oc_len_sq = origin_to_center.length_squared();

        // 광선이 구 안에서 시작하는 경우
        if oc_len_sq <= radius_sq {
            return Some(0.0);
        }

        let direction = glam::Vec3A::from(ray.direction());

        let proj = origin_to_center.dot(direction);

        // 광선이 구의 바깥에서 시작하고 구의 바깥으로 향하는 경우
        if proj <= 0.0 {
            return None;
        }

        let foot_height_sq = oc_len_sq - proj.powi(2);
        
        if foot_height_sq <= radius_sq {
            Some(proj - (radius_sq - foot_height_sq).sqrt())
        }
        else {
            None
        }
    }
}

impl RayIntersect for Capsule {
    fn ray_intersect(&self, ray: &Ray) -> Option<f32> {
        use mod_math::Line;
        
        let seg = self.get_seg();
        let seg_len = self.height - 2.0 * self.radius;
        
        let sphere1 = Sphere {
            center: seg.start,
            radius: self.radius,
        };

        // 기둥이 없는 경우 구와의 충돌체크
        if seg_len == 0.0 {
            return ray.intersect(&sphere1);
        }

        let radius_sq = self.radius.powi(2);

        // 시작점이 캡슐 안에 있는 경우
        if seg.distance_to_point_sq(&ray.origin.into()) <= radius_sq {
            return Some(0.0);
        }

        // 기둥부분 충돌체크
        let ray_origin = glam::Vec3A::from(ray.origin);
        let ray_direction = glam::Vec3A::from(ray.direction());
        
        let cylinder_direction = glam::Vec3A::from(self.direction());

        // 기둥의 아래부분 중심
        let center = glam::Vec3A::from(seg.start);

        let ray_line = Line::build(ray_origin, ray_direction).unwrap();
        let capsule_line = Line::build(center, cylinder_direction).unwrap();

        let (nearest_dist_sq, h) = ray_line.distance_sq_and_foot_from_other(&capsule_line);
        // 캡슐의 중심선과 ray직선 사이의 최소거리가 radius보다 크면 충돌하지 않음
        if nearest_dist_sq > radius_sq {
            return None;
        }

        let h_to_origin = ray_origin - h;
        let h_to_origin_len = -h_to_origin.dot(ray_direction);
        // h to origin * direction이 양수이면 충돌하지 않음(ray의 시작점이 기둥 바깥이고 바깥 방향으로 향할때)
        if h_to_origin_len < 0.0 {
            return None;
        }

        let cos = ray_direction.dot(cylinder_direction);
        let cos_sq = cos.powi(2);
        let sin_sq = 1.0 - cos_sq;

        let h_to_intersect_proj_sq = radius_sq - nearest_dist_sq;
        let h_to_intersect_sq = h_to_intersect_proj_sq / sin_sq;

        let h_to_intersect = h_to_intersect_sq.sqrt();
        
        let intersect = h - ray_direction * h_to_intersect;

        let center_to_intersect = intersect - center;
        let center_to_intersect_proj = center_to_intersect.dot(cylinder_direction);

        // 교점이 기둥의 아래쪽에 존재하면 아래 구와 충돌체크
        if center_to_intersect_proj < 0.0 {
            return ray.intersect(&sphere1);
        }

        // 교점이 기둥의 위쪽에 존재하면 위 구와 충돌체크
        if center_to_intersect_proj > seg_len {
            let sphere2 = Sphere {
                center: seg.end,
                radius: self.radius,
            };

            return ray.intersect(&sphere2);
        }

        // 교점이 기둥 범위 안에 있다면 기둥과 충돌하는 거리 리턴
        let distance = h_to_origin_len - h_to_intersect;
        Some(distance)
   }
}

impl RayIntersect for YCapsule {
    fn ray_intersect(&self, ray: &Ray) -> Option<f32> {
        let capsule = Capsule::build(
            self.center, 
            glam::Vec3::Y, 
            self.height, 
            self.radius
        ).unwrap();

        capsule.ray_intersect(ray)
    }
}

impl RayIntersect for BoundingBox {
    fn ray_intersect(&self, ray: &Ray) -> Option<f32> {
        // 1. Ray를 BoundingBox의 로컬 공간으로 변환
        let (local_ray_origin, local_ray_direction) = match self.rotation() {
            Some(rotation) => {
                let inv_rotation = rotation.transpose();    // 회전행렬의 전치행렬은 역행렬과 같다.
                let local_origin = inv_rotation * (ray.origin - self.center);
                let local_dir = inv_rotation * ray.direction();
                (local_origin, local_dir)
            }
            None => (ray.origin - self.center, ray.direction()),
        };
        
        // 2. Ray와 BoundingBox 충돌 검사
        let extents = self.extents();
        let mut tmin = f32::NEG_INFINITY;
        let mut tmax = f32::INFINITY;

        for i in 0..3 {
            // Ray가 Box의 면에 평행한 경우
            if local_ray_direction[i] == 0.0 {
                // Ray의 시작점이 Box 밖에 있는 경우
                if local_ray_origin[i] < -extents[i] || extents[i] < local_ray_origin[i] {
                    return None;
                } 
                // Ray의 시작점이 Box 안에 있는 경우
                continue;
            } else {
                let t1 = (-extents[i] - local_ray_origin[i]) / local_ray_direction[i];
                let t2 = (extents[i] - local_ray_origin[i]) / local_ray_direction[i];

                let (t1, t2) = if t1 > t2 { (t2, t1) } else { (t1, t2) };

                tmin = tmin.max(t1);
                tmax = tmax.min(t2);

                if tmin > tmax || tmax < 0.0 {
                    return None;
                }
            }
        }
        
        // 3. Ray의 충돌 거리 반환
        // Ray 시작점이 Box 안에 있는 경우 거리는 0
        if tmin < 0.0 && 0.0 <= tmax {
            Some(0.0)
        } else if tmin >= 0.0 {
            Some(tmin)
        } else {
            None
        }
    }
}



use mod_math::Segment;


fn check_boundingbox_boundingbox_collision(a: &BoundingBox, b: &BoundingBox) -> bool {
    if a.rotation().is_some() || b.rotation().is_some() {
        a.obb_collision(b)
    } else {
        a.aabb_collision(b)
    }
}

fn check_boundingbox_sphere_collision(boundingbox: &BoundingBox, sphere: &Sphere) -> bool {
    // Sphere를 BoundingBox의 로컬 공간으로 변환
    let local_sphere_center = match boundingbox.rotation() {
        Some(rotation) => {
            let inv_rotation = rotation.transpose();    // 회전행렬의 전치행렬은 역행렬과 같다.
            let local_origin = inv_rotation * (sphere.center - boundingbox.center);
            local_origin
        }
        None => sphere.center - boundingbox.center,
    };

    check_aabb_sphere_collision(
        &boundingbox.extents(), 
        &local_sphere_center, 
        sphere.radius
    )
}

/// 원점에 위치하는 회전이 없는 BoundingBox와 Sphere의 충돌 체크
fn check_aabb_sphere_collision(aabb_extents: &glam::Vec3, center: &glam::Vec3, radius: f32) -> bool {
    let mut distance_sq = 0.0;

    for i in 0..3 {
        // 각 축에 대해 Box에서 벗어난 거리 측정
        let dist = center[i].abs() - aabb_extents[i];
        if dist > 0.0 {
            // dist <= 0.0인 경우는 거리 0으로 처리
            distance_sq += dist.powi(2);
        }
    }

    distance_sq <= radius.powi(2)
}

/// 캡슐이 UFO형태인 경우는 고려하지 않는다.  
/// 
/// 원점에 위치하는 회전이 없는 BoundingBox와 Capsule의 충돌 체크  
fn check_aabb_capsule_collision(aabb_extents: &glam::Vec3, capsule: &Capsule) -> bool {
    let seg = capsule.get_seg();
    let Segment { start, end } = seg;

    // 두 점 중 하나라도 AABB 안에 있으면 충돌
    if -aabb_extents.x <= start.x && start.x <= aabb_extents.x && 
        -aabb_extents.y <= start.y && start.y <= aabb_extents.y && 
        -aabb_extents.z <= start.z && start.z <= aabb_extents.z || 
        -aabb_extents.x <= end.x && end.x <= aabb_extents.x && 
        -aabb_extents.y <= end.y && end.y <= aabb_extents.y && 
        -aabb_extents.z <= end.z && end.z <= aabb_extents.z {
        return true;
    }

    let clamped_start_x = start.x.max(-aabb_extents.x).min(aabb_extents.x);
    let clamped_end_x = end.x.max(-aabb_extents.x).min(aabb_extents.x);
    let clamped_start_y = start.y.max(-aabb_extents.y).min(aabb_extents.y);
    let clamped_end_y = end.y.max(-aabb_extents.y).min(aabb_extents.y);
    let clamped_start_z = start.z.max(-aabb_extents.z).min(aabb_extents.z);
    let clamped_end_z = end.z.max(-aabb_extents.z).min(aabb_extents.z);

    let clamped_start = [
        glam::Vec3::new(-aabb_extents.x, clamped_start_y, clamped_start_z),
        glam::Vec3::new(aabb_extents.x, clamped_start_y, clamped_start_z),
        glam::Vec3::new(clamped_start_x, -aabb_extents.y, clamped_start_z),
        glam::Vec3::new(clamped_start_x, aabb_extents.y, clamped_start_z),
        glam::Vec3::new(clamped_start_x, clamped_start_y, -aabb_extents.z),
        glam::Vec3::new(clamped_start_x, clamped_start_y, aabb_extents.z),
    ];
    let clamped_end = [
        glam::Vec3::new(-aabb_extents.x, clamped_end_y, clamped_end_z),
        glam::Vec3::new(aabb_extents.x, clamped_end_y, clamped_end_z),
        glam::Vec3::new(clamped_end_x, -aabb_extents.y, clamped_end_z),
        glam::Vec3::new(clamped_end_x, aabb_extents.y, clamped_end_z),
        glam::Vec3::new(clamped_end_x, clamped_end_y, -aabb_extents.z),
        glam::Vec3::new(clamped_end_x, clamped_end_y, aabb_extents.z),
    ];
    for i in 0..6 {
        let clamped_seg = Segment {
            start: clamped_start[i],
            end: clamped_end[i],
        };
        if seg.distance_to_other(&clamped_seg) <= capsule.radius {
            return true;
        }
    }

    false
}

/// 캡슐이 UFO형태인 경우에는 제대로 동작하지 않는다.  
/// 
/// 원점에 위치하는 회전이 없는 BoundingBox와 YCapsule의 충돌 체크
fn check_aabb_ycapsule_collision(aabb_extents: &glam::Vec3, ycapsule: &YCapsule) -> bool {
    // 1. 캡슐의 y범위가 AABB의 y범위에서 완전히 벗어나는지 체크
    let bot_y = ycapsule.center.y;
    let top_y = ycapsule.center.y + ycapsule.height;
    if top_y < -aabb_extents.y || aabb_extents.y < bot_y {
        return false;
    }

    // 2. 캡슐의 기둥부분이 AABB의 y범위에 걸치는지 체크
    let bot_y = ycapsule.center.y + ycapsule.radius;
    let top_y = ycapsule.center.y + ycapsule.height - ycapsule.radius;
    if -aabb_extents.y <= top_y && bot_y <= aabb_extents.y {
        // 걸린다면 xz평면에서의 사각형과 원의 충돌체크
        let dist_x = (ycapsule.center.x.abs() - aabb_extents.x).max(0.0);
        let dist_z = (ycapsule.center.z.abs() - aabb_extents.z).max(0.0);
        return (dist_x.powi(2) + dist_z.powi(2)) <= ycapsule.radius.powi(2);
    }

    // 3. 캡슐의 위/아래 구 부분이 AABB의 y범위에 걸치는지 체크
    let sphere_center_y = if top_y <= -aabb_extents.y { top_y } else { bot_y };
    check_aabb_sphere_collision(
        &aabb_extents,
        &glam::Vec3::new(ycapsule.center.x, sphere_center_y, ycapsule.center.z),
        ycapsule.radius
    )
}

/// 캡슐이 UFO형태인 경우에는 제대로 동작하지 않는다.
fn check_boundingbox_capsule_collision(boundingbox: &BoundingBox, capsule: &Capsule) -> bool {
    // 1. Capsule을 BoundingBox의 로컬 공간으로 변환
    let (local_center, local_direction) = match boundingbox.rotation() {
        Some(rotation) => {
            let inv_rotation = rotation.transpose();    // 회전행렬의 전치행렬은 역행렬과 같다.
            let local_center = inv_rotation * (capsule.center - boundingbox.center);
            let local_direction = inv_rotation * capsule.direction();

            (local_center, local_direction)
        }

        None => {
            let local_center = capsule.center - boundingbox.center;
            let local_direction = capsule.direction();

            (local_center, local_direction)
        }
    };

    let capsule = Capsule::build(
        local_center,
        local_direction,
        capsule.height,
        capsule.radius,
    ).unwrap();

    // 2. AABB와 Capsule 충돌 검사
    check_aabb_capsule_collision(&boundingbox.extents(), &capsule)
}

/// 캡슐이 UFO형태인 경우에는 제대로 동작하지 않는다.
fn check_boundingbox_ycapsule_collision(boundingbox: &BoundingBox, ycapsule: &YCapsule) -> bool {
    match boundingbox.rotation() {
        Some(rotation) => {
            let inv_rotation = rotation.transpose();    // 회전행렬의 전치행렬은 역행렬과 같다.
            let local_center = inv_rotation * (ycapsule.center - boundingbox.center);
            let local_direction = inv_rotation * glam::Vec3::Y;

            let capsule = Capsule::build(
                local_center,
                local_direction,
                ycapsule.height, 
                ycapsule.radius
            ).unwrap();

            check_aabb_capsule_collision(&boundingbox.extents(), &capsule)
        }

        None => {
            let capsule = YCapsule {
                center: ycapsule.center - boundingbox.center,
                height: ycapsule.height,
                radius: ycapsule.radius,
            };
            
            check_aabb_ycapsule_collision(&boundingbox.extents(), &capsule)
        }
    }
}

fn check_sphere_sphere_collision(a: &Sphere, b: &Sphere) -> bool {
    let center1 = glam::Vec3A::from(a.center);
    let center2 = glam::Vec3A::from(b.center);
    (center1 - center2).length_squared() <= (a.radius + b.radius).powi(2)
}

/// 캡슐이 UFO형태인 경우에도 제대로 동작한다.
/// 
/// Capsule의 크기를 sphere의 radius만큼 확장하고 sphere의 중심점과 충돌하는지 체크
fn check_sphere_capsule_collision(sphere: &Sphere, capsule: &Capsule) -> bool {
    let capsule = capsule.inflated(sphere.radius);

    capsule.check_point_collision(&sphere.center.into())
}

/// 캡슐이 UFO형태인 경우에도 제대로 동작한다.  
/// 
/// Capsule의 크기를 sphere의 radius만큼 확장하고 sphere의 중심점과 충돌하는지 체크
fn check_sphere_ycapsule_collision(sphere: &Sphere, ycapsule: &YCapsule) -> bool {
    let capsule = ycapsule.inflated(sphere.radius);

    capsule.check_point_collision(&sphere.center)
}

/// 캡슐이 UFO형태인 경우에는 제대로 동작하지 않는다.
/// 
/// 두 선분 사이의 거리를 구하는 것과 같은가?
/// 캡슐은 한 선분에서 radius거리 이내의 모든 점의 집합  
/// 이는 self를 sphere.radius만큼 확장한 캡슐과 나머지 캡슐의 양쪽 구의 중심을 이은 선분이 충돌하는지 체크하는것과 같다.
/// 따라서 두 선분의 최소 거리가 self.radius + sphere.radius보다 작거나 같으면 충돌한다.
fn check_capsule_capsule_collision(a: &Capsule, b: &Capsule) -> bool {
    // 두 캡슐위의 점 사이 거리가 두 캡슐의 높이 합보다 크면 충돌하지 않음
    let c_to_c = glam::Vec3A::from(b.center - a.center);
    if c_to_c.length_squared() > (a.height + b.height).powi(2) {
        return false;
    }

    // 테스트 필요
    let distance = Segment::distance_between_segments(&a.get_seg(), &b.get_seg());

    distance <= a.radius + b.radius
}

/// 캡슐이 UFO형태인 경우에는 제대로 동작하지 않는다.
/// 
/// YCapsule을 Capsule로 변환하여 check_capsule_capsule_collision 수행
fn check_capsule_ycapsule_collision(capsule: &Capsule, ycapsule: &YCapsule) -> bool {
    let ycapsule = Capsule::build(
        ycapsule.center, 
        glam::Vec3::Y, 
        ycapsule.height, 
        ycapsule.radius
    ).unwrap();

    check_capsule_capsule_collision(capsule, &ycapsule)
}

/// other캡슐이 UFO형태인 경우에는 제대로 동작하지 않는다.
/// 
/// Y축에 정렬된 두 캡슐의 충돌 체크
fn check_ycapsule_ycapsule_collision(a: &YCapsule, b: &YCapsule) -> bool {
    let a = a.inflated(b.radius);

    match a.get_y_range_at(b.center.x, b.center.z) {
        Some((bot_y, top_y)) => {
            let Segment { start, end } = b.get_seg();

            if end.y < bot_y || top_y < start.y {
                return false;
            }
            else {
                return true;
            }
        },
        None => false,
    }
}
