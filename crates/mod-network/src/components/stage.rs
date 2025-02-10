use super::{BigEndian, HealthPoint, ObjectId, TryFromBigEndian};

/// 게임 월드의 플레이어가 입은 데미지 정보입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DamageLog {
    pub object_id: ObjectId,
    pub damage: HealthPoint,
}

impl BigEndian for DamageLog {
    fn byte_size() -> usize {
        ObjectId::byte_size() + HealthPoint::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.object_id.to_big_endian_bytes());
        bytes.extend_from_slice(&self.damage.to_big_endian_bytes());
        bytes
    }
}

impl Default for DamageLog {
    fn default() -> Self {
        Self {
            object_id: ObjectId::NULL,
            damage: HealthPoint(0),
        }
    }
}

impl TryFromBigEndian for DamageLog {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(DamageLog)
        );

        // 오브젝트 식별자를 가져옵니다.
        let mut offset = 0;
        let mut size = ObjectId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let object_id = ObjectId::try_from_big_endian_bytes(data)?;

        // 데미지를 가져옵니다.
        offset = offset + size;
        size = HealthPoint::byte_size();
        data = &bytes[offset..offset + size];
        let damage = HealthPoint::try_from_big_endian_bytes(data)?;

        Some(Self { object_id, damage })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_test_player() {
        let origin = DamageLog {
            object_id: ObjectId::new(3141592),
            damage: HealthPoint(2700),
        };
        let bytes = origin.to_big_endian_bytes();
        let other = DamageLog::from_big_endian_bytes(&bytes);

        // 바이트 배열 크기가 같은지 확인
        assert_eq!(DamageLog::byte_size(), bytes.len());
        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
