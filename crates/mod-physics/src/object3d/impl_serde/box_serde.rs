use super::*;


#[derive(serde::Serialize, serde::Deserialize)]
struct BoundingBoxHelper {
    center: Vec3,
    size: Vec3,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ObbHelper {
    center: Vec3,
    size: Vec3,
    rotation: Quat,
}


impl serde::Serialize for BoundingBox {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let helper = BoundingBoxHelper {
            center: Vec3::from_glam(self.center),
            size: Vec3::from_glam(self.extents()),
        };
        helper.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for BoundingBox {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = BoundingBoxHelper::deserialize(deserializer)?;
        Ok(BoundingBox::new(
            helper.center.to_glam(),
            helper.size.to_glam(),
        ))
    }
}


impl serde::Serialize for OrientedBoundingBox {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let quat = glam::Quat::from_mat3(&self.rotation());
        let helper = ObbHelper {
            center: Vec3::from_glam(self.center),
            size: Vec3::from_glam(self.extents()),
            rotation: Quat::from_glam(quat),
        };
        helper.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for OrientedBoundingBox {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = ObbHelper::deserialize(deserializer)?;
        let rotation = helper.rotation.to_glam();
        let rotation = glam::Mat3::from_quat(rotation);
        Ok(OrientedBoundingBox::new(
            helper.center.to_glam(),
            helper.size.to_glam(),
            rotation,
        ))
    }
}