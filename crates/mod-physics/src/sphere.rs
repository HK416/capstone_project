use super::{Ray, RayIntersect};
use mod_math::Line;


pub struct Sphere {
    pub center: gmm::Float3,
    pub radius: f32,
}

impl Sphere {
    pub fn check_point_collision(&self, point: &gmm::Float3) -> bool {
        let p = gmm::Vector::from(*point - self.center);
        p.vec3_len_sq() <= self.radius.powi(2)
    }

    pub fn check_line_collision(&self, line: &Line) -> bool {
        let dist = line.distance_to_point_sq(&gmm::Vector::from(self.center));
        dist <= self.radius.powi(2)
    }

    pub fn check_sphere_collision(&self, sphere: &Sphere) -> bool {
        let p = gmm::Vector::from(self.center - sphere.center);
        p.vec3_len_sq() <= (self.radius + sphere.radius).powi(2)
    }
}


impl RayIntersect for Sphere {
    fn ray_intersect(&self, ray: &Ray) -> Option<f32> {
        let center = gmm::Vector::from(self.center);
        let origin = gmm::Vector::from(ray.origin);
        let origin_to_center = center - origin;
        
        let radius_sq = self.radius.powi(2);
        let oc_len_sq = origin_to_center.vec3_len_sq();

        // 광선이 구 안에서 시작하는 경우
        if oc_len_sq <= radius_sq {
            return Some(0.0);
        }

        let direction = gmm::Vector::from(ray.direction());

        let proj = origin_to_center.vec3_dot(direction);
        let proj_scalar = Into::<gmm::Float3>::into(proj).x;

        // 광선이 구의 바깥에서 시작하고 구의 바깥으로 향하는 경우
        if proj_scalar <= 0.0 {
            return None;
        }

        let foot_height_sq = oc_len_sq - proj_scalar.powi(2);
        
        if foot_height_sq <= radius_sq {
            Some(proj_scalar - (radius_sq - foot_height_sq).sqrt())
        }
        else {
            None
        }
    }
}