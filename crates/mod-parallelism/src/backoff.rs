pub struct Backoff {
    delay: u32,
}

impl Backoff {
    const INIT_LIMIT: u32 = 0x000000FF;
    const SPIN_LIMIT: u32 = 0x00FFFFFF;

    /// 새로운 `Backoff`를 생성합니다.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        let mut delay = rand::random();
        delay = delay % Self::INIT_LIMIT;
        Self { delay }
    }

    /// 현재 스레드를 Spin loop를 돌며 대기후 재시도 합니다.
    #[inline]
    pub fn spin_wait(&mut self) {
        for _ in 0..self.delay {
            std::hint::spin_loop();
        }

        self.delay = (self.delay << 1).min(Self::SPIN_LIMIT);
    }
}
