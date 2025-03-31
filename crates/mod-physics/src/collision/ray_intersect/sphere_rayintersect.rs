use crate::object3d::Sphere;
use super::{Ray, RayIntersect, RayIntersectInfo};


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