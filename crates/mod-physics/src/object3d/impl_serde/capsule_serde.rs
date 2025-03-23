use super::*;


#[derive(serde::Serialize, serde::Deserialize)]
struct CapsuleHelper {
    center: Vec3,
    height: f32,
    radius: f32,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct OCapsuleHelper {
    center: Vec3,
    direction: Vec3,
    height: f32,
    radius: f32,
}


impl serde::Serialize for Capsule {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let helper = CapsuleHelper {
            center: Vec3::from_glam(self.center),
            height: self.height,
            radius: self.radius,
        };
        helper.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Capsule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = CapsuleHelper::deserialize(deserializer)?;
        Ok(Capsule::new(
            helper.center.to_glam(),
            helper.height,
            helper.radius,
        ))
    }
}


impl serde::Serialize for OrientedCapsule {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let helper = OCapsuleHelper {
            center: Vec3::from_glam(self.center),
            direction: Vec3::from_glam(self.direction()),
            height: self.height,
            radius: self.radius,
        };
        helper.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for OrientedCapsule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = OCapsuleHelper::deserialize(deserializer)?;
        Ok(OrientedCapsule::new(
            helper.center.to_glam(),
            helper.direction.to_glam(),
            helper.height,
            helper.radius,
        ).unwrap())
    }
}