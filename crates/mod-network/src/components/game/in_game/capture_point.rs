use crate::components::{BigEndian, Team, TryFromBigEndian};


#[derive(Debug, Clone, PartialEq)]
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
        f32::byte_size() * 3 + size_of::<Team>()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.capture_progress.to_big_endian_bytes());
        bytes.extend_from_slice(&self.capture_score[0].to_big_endian_bytes());
        bytes.extend_from_slice(&self.capture_score[1].to_big_endian_bytes());
        if let Some(team) = self.capture_team {
            bytes.extend_from_slice(&team.to_big_endian_bytes());
        }
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
        assert!(
            bytes.len() == Self::byte_size() || bytes.len() == Self::byte_size() - size_of::<Team>(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(DamageLog)
        );

        let mut offset = 0;
        let mut size = f32::byte_size();
        let mut data = &bytes[offset..offset + size];
        let capture_progress = f32::from_big_endian_bytes(data);

        offset = offset + size;
        size = f32::byte_size();
        data = &bytes[offset..offset + size];
        let capture_score_0 = f32::from_big_endian_bytes(data);

        offset = offset + size;
        size = f32::byte_size();
        data = &bytes[offset..offset + size];
        let capture_score_1 = f32::from_big_endian_bytes(data);

        if bytes.len() > offset + size {
            size = Team::byte_size();
            data = &bytes[offset..offset + size];
            let capture_team = Team::from_big_endian_bytes(data);
            Some(Self {
                capture_progress,
                capture_score: [capture_score_0, capture_score_1],
                capture_team: Some(capture_team),
            })
        } else {
            Some(Self {
                capture_progress,
                capture_score: [capture_score_0, capture_score_1],
                capture_team: None,
            })
        }
    }
}
