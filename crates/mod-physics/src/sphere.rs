use super::{Ray, RayIntersect};
use mod_math::Line;


pub struct Sphere {
    pub center: glam::Vec3,
    pub radius: f32,
}

impl Sphere {
    pub fn check_point_collision(&self, point: &glam::Vec3A) -> bool {
        let center = glam::Vec3A::from(self.center);
        (point - center).length_squared() <= self.radius.powi(2)
    }

    pub fn check_line_collision(&self, line: &Line) -> bool {
        let dist = line.distance_to_point_sq(&self.center.into());
        dist <= self.radius.powi(2)
    }

    pub fn check_sphere_collision(&self, sphere: &Sphere) -> bool {
        let center1 = glam::Vec3A::from(self.center);
        let center2 = glam::Vec3A::from(sphere.center);
        (center1 - center2).length_squared() <= (self.radius + sphere.radius).powi(2)
    }
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