//! 인게임 점령 데이터와 관련된 코드를 관리합니다.

use crate::components::{BigEndian, Team, TryFromBigEndian};

/// 최대 점령점수. capture_score가 이 값에 도달하면 게임이 종료됩니다.
pub const MAX_CAPTURE_SCORE: f32 = 60.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CapturePoint {
    /// 점령도
    pub capture_progress: f32,
    /// 팀별 점령 점수
    pub capture_score: [f32; 2],
    /// 점령중인 팀
    pub capture_team: Option<Team>,
}

impl BigEndian for CapturePoint {
    fn byte_size() -> usize {
        f32::byte_size() + <[f32; 2]>::byte_size() + u8::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.capture_progress.to_big_endian_bytes());
        bytes.extend_from_slice(&self.capture_score[0].to_big_endian_bytes());
        bytes.extend_from_slice(&self.capture_score[1].to_big_endian_bytes());

        // `None`일 경우 0x2를 설정합니다.
        bytes.extend_from_slice(
            &self
                .capture_team
                .map(|team| team as u8)
                .unwrap_or(0x2)
                .to_big_endian_bytes(),
        );
        bytes
    }
}

impl Default for CapturePoint {
    fn default() -> Self {
        Self {
            capture_progress: 0.0,
            capture_score: [0.0, 0.0],
            capture_team: None,
        }
    }
}

impl TryFromBigEndian for CapturePoint {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(CapturePoint)
        );

        let mut offset = 0;
        let mut size = f32::byte_size();
        let mut data = &bytes[offset..offset + size];
        let capture_progress = f32::from_big_endian_bytes(data);

        offset = offset + size;
        size = <[f32; 2]>::byte_size();
        data = &bytes[offset..offset + size];
        let capture_score = <[f32; 2]>::from_big_endian_bytes(data);

        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let val = u8::from_big_endian_bytes(data);
        let capture_team = if val < 0x2 {
            unsafe { Some(Team::new(val).unwrap_unchecked()) }
        } else {
            None
        };

        Some(Self {
            capture_progress,
            capture_score,
            capture_team,
        })
    }
}
