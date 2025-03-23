use crate::object3d::{
    BoundingBox, OrientedBoundingBox, VertexBox, 
    Capsule, OrientedCapsule, 
    Sphere
};


impl From<&OrientedBoundingBox> for BoundingBox {
    fn from(obb: &OrientedBoundingBox) -> Self {
        let vbox = VertexBox::from(obb);
        BoundingBox::from(&vbox)
    }
}

impl From<&VertexBox> for BoundingBox {
    fn from(vertexbox: &VertexBox) -> Self {
        let mut min = glam::Vec3A::splat(f32::INFINITY);
        let mut max = glam::Vec3A::splat(f32::NEG_INFINITY);
        for vertex in vertexbox.get_vertices().iter() {
            min = min.min(*vertex);
            max = max.max(*vertex);
        }
        
        let center = (min + max) / 2.0;
        let extents = (max - min) / 2.0;

        BoundingBox::new(
            center.into(), 
            extents.into(), 
        )
    }
}

impl From<&Capsule> for BoundingBox {
    fn from(capsule: &Capsule) -> Self {
        let mut center = capsule.center;
        center.y += capsule.height / 2.0;

        let extents = glam::Vec3::new(capsule.radius, capsule.height / 2.0, capsule.radius);

        BoundingBox::new(center, extents)
    }
}

impl From<&OrientedCapsule> for BoundingBox {
    fn from(ocapsule: &OrientedCapsule) -> Self {
        let mut center = ocapsule.center;
        center += ocapsule.direction() * (ocapsule.height / 2.0);
        
        let seg = ocapsule.direction() * (ocapsule.height - 2.0 * ocapsule.radius);
        let ex = seg.x.abs() / 2.0;
        let ey = seg.y.abs() / 2.0;
        let ez = seg.z.abs() / 2.0;
        let extents = glam::Vec3::new(ex, ey, ez) + ocapsule.radius;

        BoundingBox::new(center, extents)
    }
}

impl From<&Sphere> for BoundingBox {
    fn from(sphere: &Sphere) -> Self {
        let center = sphere.center;
        let extents = glam::Vec3::splat(sphere.radius);

        BoundingBox::new(center, extents)
    }
}