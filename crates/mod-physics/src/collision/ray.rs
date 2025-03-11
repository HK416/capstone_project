use crate::object3d::{BoundingBox, Capsule, Sphere};


pub struct Ray {
    pub origin: glam::Vec3,
    direction: glam::Vec3,
}

impl Ray {
    pub fn build(origin: glam::Vec3A, direction: glam::Vec3A) -> Result<Self, &'static str> {
        match direction.try_normalize() {
            Some(direction) => Ok(Self { 
                origin: origin.into(), 
                direction: direction.into()
            }),
            None => Err("Direction cannot be zero vector")
        }
    }

    pub fn set_direction(&mut self, direction: glam::Vec3A) -> Result<(), &'static str> {
        match direction.try_normalize() {
            Some(direction) => {
                self.direction = direction.into();
                Ok(())
            },
            None => Err("Direction cannot be zero vector")
        }
    }

    /// 정규화된 방향 벡터를 반환한다.
    pub fn direction(&self) -> glam::Vec3 {
        self.direction
    }

    pub fn intersect<T: RayIntersect>(&self, object: &T) -> Option<RayIntersectInfo> {
        object.ray_intersect(self)
    }
}


pub struct RayIntersectInfo {
    pub distance: f32,
    /// 충돌한 지점(표면)의 법선 벡터
    pub normal: glam::Vec3A,
}

/// Ray와 다른 객체가 충돌하는지 검사, 충돌한다면 가장 가까운 충돌 지점까지의 거리를 반환한다.
pub trait RayIntersect {
    fn ray_intersect(&self, ray: &Ray) -> Option<RayIntersectInfo>;
}

impl RayIntersect for Sphere {
    fn ray_intersect(&self, ray: &Ray) -> Option<RayIntersectInfo> {
        let center = glam::Vec3A::from(self.center);
        let origin = glam::Vec3A::from(ray.origin);
        let origin_to_center = center - origin;
        
        let radius_sq = self.radius.powi(2);
        let oc_len_sq = origin_to_center.length_squared();

        // 광선이 구 안에서 시작하는 경우
        if oc_len_sq <= radius_sq {
            return Some(RayIntersectInfo {
                distance: 0.0,
                normal: glam::Vec3A::ZERO,
            });
        }

        let direction = glam::Vec3A::from(ray.direction());

        let proj = direction.dot(origin_to_center);

        // 광선이 구의 바깥에서 시작하고 구의 바깥으로 향하는 경우
        if proj <= 0.0 {
            return None;
        }

        let foot_height_sq = oc_len_sq - proj.powi(2);
        let depth = radius_sq - foot_height_sq; // 충돌점과 수선의발 사이 거리
        if depth < 0.0 {
            return None;
        }

        let distance = proj - depth.sqrt();
        let collision_point = origin + direction * distance;
        let normal = (collision_point - center).normalize();

        Some(RayIntersectInfo {
            distance,
            normal,
        })
    }
}

impl RayIntersect for Capsule {
    fn ray_intersect(&self, ray: &Ray) -> Option<RayIntersectInfo> {
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
            return Some(RayIntersectInfo {
                distance: 0.0,
                normal: glam::Vec3A::ZERO,
            });
        }

        // 기둥부분 충돌체크
        let ray_origin = glam::Vec3A::from(ray.origin);
        let ray_direction = glam::Vec3A::from(ray.direction());
        
        let cylinder_direction = match self.direction() {
            Some(direction) => glam::Vec3A::from(direction),
            None => glam::Vec3A::Y,
        };

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

        // 교점이 기둥 범위 안에 있다면 기둥과 충돌체크
        let normal = intersect - (center + cylinder_direction * center_to_intersect_proj);
        Some(RayIntersectInfo { 
            distance: h_to_origin_len - h_to_intersect,
            normal: normal.normalize(),
        })
    }
}

impl RayIntersect for BoundingBox {
    fn ray_intersect(&self, ray: &Ray) -> Option<RayIntersectInfo> {
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
        let mut normal = glam::Vec3::ZERO;

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

                let mut normal_sign = 1.0;
                let (t1, t2) = if t1 > t2 {
                    (t2, t1)
                } else {
                    normal_sign = -1.0;
                    (t1, t2)
                };

                if t1 > tmin {
                    normal = glam::Vec3::ZERO;
                    normal[i] = normal_sign;
                    tmin = t1;
                }
                tmax = tmax.min(t2);

                if tmin > tmax || tmax < 0.0 {
                    return None;
                }
            }
        }
        
        // 3. Ray의 충돌 거리 반환
        // Ray 시작점이 Box 안에 있는 경우 거리는 0
        if tmin < 0.0 && 0.0 <= tmax {
            Some(RayIntersectInfo {
                distance: 0.0,
                normal: glam::Vec3A::ZERO,
            })
        } else if tmin >= 0.0 {
            let normal = glam::Vec3A::from(normal);
            let normal = match self.rotation() {
                Some(rotation) => rotation * normal,
                None => normal,
            };
            Some(RayIntersectInfo {
                distance: tmin,
                normal,
            })
        } else {
            None
        }
    }
}
