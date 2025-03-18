use crate::object3d::{BoundingBox, OrientedBoundingBox};
use super::{RayIntersect, RayIntersectInfo, Ray};


impl RayIntersect for BoundingBox {
    fn ray_intersect(&self, ray: &Ray) -> Option<RayIntersectInfo> {
        let ray_direction = ray.direction();
        let local_ray_origin = ray.origin - self.center;
        
        let extents = self.extents();
        let mut tmin = f32::NEG_INFINITY;
        let mut tmax = f32::INFINITY;
        let mut normal = glam::Vec3::ZERO;

        for i in 0..3 {
            // Ray가 Box의 면에 평행한 경우
            if ray_direction[i] == 0.0 {
                // Ray의 시작점이 Box 밖에 있는 경우
                if local_ray_origin[i] < -extents[i] || extents[i] < local_ray_origin[i] {
                    return None;
                } 
                // Ray의 시작점이 Box 안에 있는 경우
                continue;
            } else {
                let t1 = (-extents[i] - local_ray_origin[i]) / ray_direction[i];
                let t2 = (extents[i] - local_ray_origin[i]) / ray_direction[i];

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
            Some(RayIntersectInfo {
                distance: tmin,
                normal,
            })
        } else {
            None
        }
    }
}


impl RayIntersect for OrientedBoundingBox {
    fn ray_intersect(&self, ray: &Ray) -> Option<RayIntersectInfo> {
        let rotation = self.rotation();
        let inv_rotation = rotation.transpose();    // 회전행렬의 전치행렬은 역행렬과 같다.
        let local_ray_origin = inv_rotation * (ray.origin - self.center);
        let local_ray_direction = inv_rotation * ray.direction();

        let ray = Ray::build(
            local_ray_origin.into(), 
            local_ray_direction.into(),
        ).unwrap();
        let aabb = BoundingBox::new(glam::Vec3::ZERO, self.extents());
        
        let info = aabb.ray_intersect(&ray)?;
        Some(RayIntersectInfo {
            distance: info.distance,
            normal: rotation * info.normal,
        })
    }
}
