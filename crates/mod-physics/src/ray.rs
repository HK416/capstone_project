use super::RayIntersect;


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

    pub fn intersect<T: RayIntersect>(&self, object: &T) -> Option<f32> {
        object.ray_intersect(self)
    }
}
