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

    pub fn direction(&self) -> glam::Vec3 {
        self.direction
    }

    pub fn intersect<T: RayIntersect>(&self, object: &T) -> Option<f32> {
        object.ray_intersect(self)
    }
}


/// Ray와 다른 객체가 충돌하는지 검사, 충돌한다면 가장 가까운 충돌 지점까지의 거리를 반환한다.
pub trait RayIntersect {
    fn ray_intersect(&self, ray: &Ray) -> Option<f32>;
}
