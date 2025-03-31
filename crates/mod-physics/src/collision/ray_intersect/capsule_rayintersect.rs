use crate::object3d::{Capsule, OrientedCapsule, Sphere};
use super::{Ray, RayIntersect, RayIntersectInfo};


impl RayIntersect for Capsule {
    fn ray_intersect(&self, ray: &Ray) -> Option<RayIntersectInfo> {
        let capsule = OrientedCapsule::new(
            self.center,
            glam::Vec3::Y,
            self.height,
            self.radius,
        ).unwrap();

        ray.intersect(&capsule)
    }
}

impl RayIntersect for OrientedCapsule {
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

        // 교점이 기둥 범위 안에 있다면 기둥과 충돌체크
        let normal = intersect - (center + cylinder_direction * center_to_intersect_proj);
        Some(RayIntersectInfo { 
            distance: h_to_origin_len - h_to_intersect,
            normal: normal.normalize(),
        })
    }
}
