use std::{
    sync::atomic::{AtomicU32, Ordering as MemOrdering},
    time::{SystemTime, UNIX_EPOCH},
};

use mod_parallelism::backoff::Backoff;

// 식별자 비트 (64bit)
// +------------------+------------------+-------------------------------------+  
// | 초 단위 하위 24비트  |나노초 단위 하위 16비트|            카운터 24비트               |  
// +------------------+------------------+-------------------------------------+  
//
/// 64비트 식별자를 생성하는 생성기입니다.
/// 생성기의 카운트는 최대 24bit(16,777,216) 값을 가집니다.
#[derive(Debug)]
pub struct IdentifierGenerator {
    counter: AtomicU32,
}

impl IdentifierGenerator {
    pub const MAX_COUNT: u32 = 0xFFFFFF;

    /// 새로운 식별자 생성기를 생성합니다.
    pub const fn new() -> Self {
        Self {
            counter: AtomicU32::new(0),
        }
    }

    /// 주어진 정수로 초기화된 식별자 생성기를 생성합니다.
    pub const fn with_count(n: u32) -> Self {
        Self {
            counter: AtomicU32::new(n),
        }
    }

    /// 64비트 식별자를 생성합니다.
    pub fn generate(&self) -> u64 {
        let now = SystemTime::now();
        let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();

        // 현재 카운트를 가져옵니다.
        #[allow(unused_assignments)]
        let mut count = 0;
        let mut backoff = Backoff::new();
        loop {
            count = self.counter.load(MemOrdering::Relaxed);
            if self.counter.compare_exchange(
                count, 
                (count + 1) % Self::MAX_COUNT, 
                MemOrdering::Release, 
                MemOrdering::Relaxed
            ).is_ok() {
                break;
            }
            backoff.wait();
        }

        // 초 단위 시간과 나노초 단위 시간의 하위 16비트를 가져옵니다.
        let secs_part = duration.as_secs() & 0xFFFFFF;
        let nanos_part = (duration.subsec_nanos() as u64) & 0xFFFF;

        (secs_part << 40) | (nanos_part << 24) | (count as u64)
    }

    /// 식별자 생성기의 카운트를 증가시킵니다.
    pub fn increase_count(&self) {
        self.counter.fetch_add(1, MemOrdering::AcqRel);
    }

    /// 식별자 생성기의 카운터를 가져옵니다.
    pub fn get_count(&self) -> u32 {
        self.counter.load(MemOrdering::Relaxed)
    }
}

impl Default for IdentifierGenerator {
    fn default() -> Self {
        Self {
            counter: AtomicU32::new(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::IdentifierGenerator;

    #[test]
    fn check_identifier_generation() {
        let generator = IdentifierGenerator::new();
        let id_0 = generator.generate();
        let id_1 = generator.generate();
        assert_ne!(id_0, id_1);
    }
}
