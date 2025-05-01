use crate::{
    components::{BigEndian, DamageLog, Epoch, TryFromBigEndian},
    protocol::{Packet, PacketType, RawPacket},
};

/// 서버에서 클라이언트로 보내는 데미지 로그 정보 패킷.
///
/// # Note
/// 이 패킷은 항상 1024Byte 보다 작아야 합니다.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdpDamageLogPacket {
    pub logs: Vec<DamageLog>,
}

impl UdpDamageLogPacket {
    /// 새로운 데미지 로그 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 데미지 로그가 패킷에 담을 수 있는 수를 초과할 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(logs: Vec<DamageLog>) -> Self {
        assert!(
            logs.len() <= Self::capacity(),
            "damage logs must be at most {}",
            Self::capacity()
        );

        Self { logs }
    }

    /// 새로운 데미지 로그 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 데미지 로그가 패킷에 담을 수 있는 수를 초과할 경우 [`panic!`]을 호출합니다.
    ///
    pub fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = DamageLog>,
        I::IntoIter: ExactSizeIterator,
    {
        Self::new(iter.into_iter().collect())
    }

    /// 패킷에 담을 수 있는 로그의 수를 반환합니다.
    pub fn capacity() -> usize {
        (1024 - (Epoch::byte_size() + u16::byte_size())) / DamageLog::byte_size()
    }
}

impl Packet for UdpDamageLogPacket {
    fn packet_type() -> PacketType {
        PacketType::UdpDamageLog
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = u16::byte_size() + DamageLog::byte_size() * self.logs.len();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&(self.logs.len() as u16).to_big_endian_bytes());
        for log in self.logs.iter() {
            data.extend_from_slice(&log.to_big_endian_bytes());
        }

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(DamageLog)
            );
        }

        RawPacket::new(Self::packet_type(), &data)
    }

    fn try_from_raw(raw: RawPacket) -> Option<Self> {
        // 패킷 종류가 일치하는지 확인합니다.
        if raw.packet_type() != Self::packet_type() {
            log::warn!(
                "invalid packet type. (RAW:{:?}, TARGET:{:?})",
                raw.packet_type(),
                Self::packet_type()
            );
            return None;
        }

        // 로그의 수를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = u16::byte_size();
        let mut data = &bytes[offset..offset + size];
        let mut num_logs = u16::from_big_endian_bytes(data);

        // 로그 데이터를 가져옵니다.
        let mut logs = Vec::with_capacity(num_logs as usize);
        while num_logs > 0 {
            offset = offset + size;
            size = DamageLog::byte_size();
            data = &bytes[offset..offset + size];
            logs.push(DamageLog::try_from_big_endian_bytes(data)?);
            num_logs -= 1;
        }

        Some(Self { logs })
    }
}

#[cfg(test)]
mod tests {
    use crate::components::UserId;

    use super::*;

    #[test]
    fn test_udp_datamge_log_packet() {
        let origin = UdpDamageLogPacket::new(vec![
            DamageLog {
                user_id: UserId::new(123456),
                damage: 1010,
            },
            DamageLog {
                user_id: UserId::new(1),
                damage: 52,
            },
        ]);
        let raw_packet = origin.as_raw();
        let other = UdpDamageLogPacket::try_from_raw(raw_packet).unwrap();

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
