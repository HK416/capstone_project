use crate::components::{BigEndian, DamageLog, Epoch, TryFromBigEndian};

use super::{Packet, PacketType, RawPacket};

/// 서버에서 클라이언트로 보내는 데미지 로그 정보 패킷.
///
/// # Note
/// 이 패킷은 항상 1024Byte 보다 작아야 합니다.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdpDamageLogPacket {
    pub epoch: Epoch,
    pub num_logs: u16,
    pub logs: Vec<DamageLog>,
}

impl UdpDamageLogPacket {
    /// 새로운 데미지 로그 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 데미지 로그가 패킷에 담을 수 있는 수를 초과할 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(epoch: Epoch, logs: Vec<DamageLog>) -> Self {
        assert!(
            logs.len() <= Self::capacity(),
            "damage logs must be at most {}",
            Self::capacity()
        );

        Self {
            epoch,
            num_logs: logs.len() as u16,
            logs,
        }
    }

    /// 패킷에 담을 수 있는 로그의 수를 반환합니다.
    pub fn capacity() -> usize {
        (1024 - (Epoch::byte_size() + u16::byte_size())) / DamageLog::byte_size()
    }
}

impl Default for UdpDamageLogPacket {
    fn default() -> Self {
        Self {
            epoch: Epoch::new(0),
            num_logs: 0,
            logs: Vec::default(),
        }
    }
}

impl Packet for UdpDamageLogPacket {
    fn packet_type() -> super::PacketType {
        PacketType::UdpDamageLog
    }

    fn as_raw(&self) -> super::RawPacket {
        let data_size =
            Epoch::byte_size() + u16::byte_size() + DamageLog::byte_size() * self.num_logs as usize;

        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.epoch.to_big_endian_bytes());
        data.extend_from_slice(&self.num_logs.to_big_endian_bytes());
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

        // 서버의 시대를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = Epoch::byte_size();
        let mut data = &bytes[offset..offset + size];
        let epoch = Epoch::from_big_endian_bytes(data);

        // 로그의 수를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let num_logs = u16::from_big_endian_bytes(data);

        // 로그 데이터를 가져옵니다.
        let mut count = num_logs as usize;
        let mut logs = Vec::with_capacity(count);
        while count > 0 {
            offset = offset + size;
            size = DamageLog::byte_size();
            data = &bytes[offset..offset + size];
            logs.push(DamageLog::try_from_big_endian_bytes(data)?);
            count -= 1;
        }

        Some(Self {
            epoch,
            num_logs,
            logs,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::components::{HealthPoint, UserId};

    use super::*;

    #[test]
    fn validation_test_packet() {
        let origin = UdpDamageLogPacket::new(
            Epoch::new(0),
            vec![
                DamageLog {
                    user_id: UserId::new(123456),
                    damage: HealthPoint(1010),
                },
                DamageLog {
                    user_id: UserId::new(1),
                    damage: HealthPoint(52),
                },
            ],
        );
        let raw_packet = origin.as_raw();
        let other = UdpDamageLogPacket::try_from_raw(raw_packet).unwrap();

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
