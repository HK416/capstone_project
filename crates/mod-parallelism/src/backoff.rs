/// `Backoff` 값의 초기 최대 값 입니다.
const BACKOFF_INIT_LIMIT: u32 = 0x000000FF;

/// `Backoff` 값의 최대 값 입니다.
const BACKOFF_MAX_LIMIT: u32 = 0x00FFFFFF;

/// ## Backoff
/// 지정된 횟수만큼 스핀 루프를 돌며 대기합니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Backoff(u32);

impl Backoff {
    pub fn new() -> Self {
        Self::default()
    }

    /// 지정된 횟수만큼 스핀 루프를 돌며 대기합니다.
    /// 이후 지정된 횟수를 증가시킵니다.
    pub fn wait(&mut self) {
        for _ in 0..self.0 {
            core::hint::spin_loop();
        }

        self.0 = (self.0 << 1).min(BACKOFF_MAX_LIMIT);
    }
}

impl Default for Backoff {
    fn default() -> Self {
        Self(rand::random::<u32>() % BACKOFF_INIT_LIMIT)
    }
}
