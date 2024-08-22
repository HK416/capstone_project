use std::time::Instant;

/// `GameTimer`가 저장하는 샘플 경과 시간의 갯수입니다.
pub const NUM_SAMPLES: usize = 50;



/// 특정 시각의 경과 시간을 측정하는 자료형입니다.
/// 
/// ※ 샘플의 평균 경과 시간을 반환합니다.
/// 
#[derive(Debug, Clone, Copy)]
pub struct GameTimer {
    base_timepoint: Instant, 
    prev_timepoint: Instant,
    curr_timepoint: Instant,

    frame_samples: [f32; NUM_SAMPLES],
    cnt_frame_samples: usize,

    frame_rate: u32,
    elapsed_time_sec: f32,

    frame_per_seconds: u32,
    fps_elapsed_time_sec: f32,
}

impl GameTimer {
    /// `GameTimer`를 시작합니다.
    #[inline(always)]
    pub fn start() -> Self {
        Self::default()
    }

    /// `GameTimer`를 초기화 합니다.
    #[inline(always)]
    pub fn reset(&mut self) {
        *self = Self::start();
    }

    /// 경과한 시간과 프레임 레이트를 측정합니다.
    pub fn tick<'a>(&'a mut self) {
        self.curr_timepoint = Instant::now();
        let elapsed_time_sec = self.curr_timepoint
            .saturating_duration_since(self.prev_timepoint)
            .as_secs_f32();

        self.prev_timepoint = self.curr_timepoint;

        // 샘플 경과 시간을 추가합니다.
        if (self.elapsed_time_sec - elapsed_time_sec).abs() < 1.0 {
            self.frame_samples.copy_within(0..NUM_SAMPLES - 1, 1);
            self.frame_samples[0] = elapsed_time_sec;
            self.cnt_frame_samples = (self.cnt_frame_samples + 1).min(NUM_SAMPLES);
        }

        // 프레임 레이트를 측정합니다.
        self.frame_per_seconds += 1;
        self.fps_elapsed_time_sec += elapsed_time_sec;
        if self.fps_elapsed_time_sec > 1.0 {
            self.frame_rate = self.frame_per_seconds;
            self.frame_per_seconds = 0;
            self.fps_elapsed_time_sec -= 1.0;
        }

        // 현재 경과 시간을 측정합니다.
        self.elapsed_time_sec = 0.0;
        if self.cnt_frame_samples > 0 {
            self.elapsed_time_sec = self.frame_samples
                .iter()
                .take(self.cnt_frame_samples)
                .sum();
            self.elapsed_time_sec /= self.cnt_frame_samples as f32;
        }
    }

    /// 현재 프레임 레이트를 반환합니다.
    #[must_use]
    #[inline(always)]
    pub fn frame_rate(&self) -> u32 {
        self.frame_rate
    }

    /// 현재 경과 시간을 반환합니다.
    #[must_use]
    #[inline(always)]
    pub fn elapsed_time_sec(&self) -> f32 {
        self.elapsed_time_sec
    }

    /// 총 경과 시간을 반환합니다.
    #[must_use]
    #[inline(always)]
    pub fn total_time_sec(&self) -> f32 {
        self.curr_timepoint
            .saturating_duration_since(self.base_timepoint)
            .as_secs_f32()
    }
}

impl Default for GameTimer {
    #[inline]
    fn default() -> Self {
        let timepoint = Instant::now();
        Self { 
            base_timepoint: timepoint, 
            prev_timepoint: timepoint, 
            curr_timepoint: timepoint, 
            frame_samples: [0.0; NUM_SAMPLES], 
            cnt_frame_samples: 0, 
            frame_rate: 0,
            elapsed_time_sec: 0.0,
            frame_per_seconds: 0, 
            fps_elapsed_time_sec: 0.0,
        }
    }
}
