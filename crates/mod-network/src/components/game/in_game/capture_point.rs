//! 인게임 점령 데이터와 관련된 코드를 관리합니다.

use std::cmp;

use crate::components::{BigEndian, Team, TryFromBigEndian};

/// 최대 점령도
pub const MAX_CAPTURE_PROGRESS_VAL: i16 = 15_000;
/// 밀리초 당 증가하는 점령도의 양
pub const CAPTURE_PROGRESS_PER_MS: u16 = 1;
/// 최대 점령점수. capture_score가 이 값에 도달하면 게임이 종료됩니다.
pub const MAX_CAPTURE_SCORE: u16 = 50_000;
/// 밀리초 당 증가하는 점령 점수의 양
pub const CAPTURE_SCORE_PER_MS: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CapturePoint {
    /// 점령도. 레드팀의 경우 음수, 블루 팀의 경우 양수입니다.
    progress: i16,
    /// 팀별 점령 점수
    score: [u16; 2],
    /// 점령중인 팀
    team: Option<Team>,
}

impl CapturePoint {
    /// 새로운 `CapturePoint`를 생성합니다.
    pub const fn new(progress: i16, score: [u16; 2], team: Option<Team>) -> Self {
        Self {
            progress,
            score,
            team,
        }
    }

    /// 블루 팀의 점령도를 0..=1 사이의 값으로 나타냅니다.
    pub fn blue_progress(&self) -> f32 {
        let progress = self.progress.clamp(0, MAX_CAPTURE_PROGRESS_VAL);
        progress.abs() as f32 / MAX_CAPTURE_PROGRESS_VAL as f32
    }

    /// 블루 팀의 점령 점수를 0..=1 사이의 값으로 나타냅니다.
    pub fn blue_score(&self) -> f32 {
        self.score[Team::Blue as usize] as f32 / MAX_CAPTURE_SCORE as f32
    }

    /// 레드 팀의 점령도를 0..=1 사이의 값으로 나타냅니다.
    pub fn red_progress(&self) -> f32 {
        let progress = self.progress.clamp(-MAX_CAPTURE_PROGRESS_VAL, 0);
        progress.abs() as f32 / MAX_CAPTURE_PROGRESS_VAL as f32
    }

    /// 레드 팀의 점령 점수를 0..=1 사이의 값으로 나타냅니다.
    pub fn red_score(&self) -> f32 {
        self.score[Team::Red as usize] as f32 / MAX_CAPTURE_SCORE as f32
    }

    /// 현재 점령중인 팀을 설정합니다.
    pub fn set_capture_team(&mut self, team: Option<Team>) {
        self.team = team;
    }

    /// 높은 점수를 가진 팀을 반환합니다. 두 팀의 점령 점수가 같은 경우 `None`을 반환합니다.
    pub fn max_score_team(&self) -> Option<Team> {
        let blue_score = self.score[Team::Blue as usize];
        let red_score = self.score[Team::Red as usize];
        match blue_score.cmp(&red_score) {
            cmp::Ordering::Less => Some(Team::Red),
            cmp::Ordering::Equal => None,
            cmp::Ordering::Greater => Some(Team::Blue),
        }
    }

    /// 주어진 시간 만큼 현재 점령 중인 팀의 점령도와 점령 점수를 갱신합니다.
    pub fn update(&mut self, elapsed_time_ms: u16, offset: u16) {
        if let Some(team) = self.team {
            match team {
                Team::Blue => {
                    // 블루 팀의 점령도가 가득 찬 경우 점령 점수를 갱신합니다.
                    if self.progress >= MAX_CAPTURE_PROGRESS_VAL {
                        let val = CAPTURE_SCORE_PER_MS * elapsed_time_ms * offset;
                        let score = &mut self.score[Team::Blue as usize];
                        *score = score.saturating_add(val).min(MAX_CAPTURE_SCORE);
                    }
                    // 블루 팀의 점령도가 가득 차지 않은 경우 점령도를 갱신합니다.
                    else {
                        let val = CAPTURE_PROGRESS_PER_MS * elapsed_time_ms * offset;
                        self.progress = self
                            .progress
                            .saturating_add_unsigned(val)
                            .min(MAX_CAPTURE_PROGRESS_VAL);
                    }
                }
                Team::Red => {
                    // 레드 팀의 점령도가 가득 찬 경우 점령 점수를 갱신합니다.
                    if self.progress <= -MAX_CAPTURE_PROGRESS_VAL {
                        let val = CAPTURE_SCORE_PER_MS * elapsed_time_ms * offset;
                        let score = &mut self.score[Team::Red as usize];
                        *score = score.saturating_add(val).min(MAX_CAPTURE_SCORE);
                    }
                    // 레드 팀의 점령도가 가득 차지 않은 경우 점령도를 갱신합니다.
                    else {
                        let val = CAPTURE_PROGRESS_PER_MS * elapsed_time_ms * offset;
                        self.progress = self
                            .progress
                            .saturating_sub_unsigned(val)
                            .max(-MAX_CAPTURE_PROGRESS_VAL);
                    }
                }
            }
        }
    }
}

impl BigEndian for CapturePoint {
    fn byte_size() -> usize {
        i16::byte_size() + <[i16; 2]>::byte_size() + u8::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.progress.to_big_endian_bytes());
        bytes.extend_from_slice(&self.score.to_big_endian_bytes());
        // `None`일 경우 0x2를 설정합니다.
        let team = self.team.map(|team| team as u8).unwrap_or(0x2);
        bytes.extend_from_slice(&team.to_big_endian_bytes());

        // 바이트 배열 유효성을 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(CapturePoint)
            );
        }

        bytes
    }
}

impl Default for CapturePoint {
    fn default() -> Self {
        Self {
            progress: 0,
            score: [0, 0],
            team: None,
        }
    }
}

impl TryFromBigEndian for CapturePoint {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 크기가 다른지 확인한다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(CapturePoint)
            );
        }

        let mut offset = 0;
        let mut size = i16::byte_size();
        let mut data = &bytes[offset..offset + size];
        let progress = i16::from_big_endian_bytes(data);

        offset = offset + size;
        size = <[u16; 2]>::byte_size();
        data = &bytes[offset..offset + size];
        let score = <[u16; 2]>::from_big_endian_bytes(data);

        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let team = Team::new(u8::from_big_endian_bytes(data));

        Some(Self {
            progress,
            score,
            team,
        })
    }
}
