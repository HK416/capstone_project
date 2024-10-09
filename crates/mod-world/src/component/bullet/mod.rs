//! 총알의 데이터
//! 
//! 1. 종류 (발사 플레이어 캐릭터 모델)
//! - 캐릭터 모델마다 발사하는 총알의 크기나 모양이 다르다.
//! 
//! 2. 위치
//! 
//! 3. 방향
//! 
//! 4. 속력 (방향 없음)
//! - 총알은 직선으로 날아가는 등속도 운동을 한다고 가정한다.
//! 
//! 5. 사거리
//! - 총알은 최대 사거리까지 이동한 후 사라진다.
//! 
//! 6. 발사한 플레이어의 식별자
//! 



/// 총알의 종류와 Attacking 상태에서 지연 시간을 나타냅니다.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum BulletKind {
    ArisOriginal, 
}

impl BulletKind {
    /// 총알의 식별 번호로부터 총알의 종류를 생성합니다.
    #[inline]
    #[must_use]
    pub fn from_id(id: u32) -> Self {
        match id {
            0 => BulletKind::ArisOriginal, 
            _ => panic!("out of range!")
        }
    }

    /// 총알의 식별 번호를 반환합니다.
    #[inline]
    #[must_use]
    pub fn into_id(self) -> u32 {
        match self {
            BulletKind::ArisOriginal => 0,
        }
    }


    /// 총알의 지연 시간을 반환합니다.
    #[inline]
    #[must_use]
    pub fn delay_time_sec(self) -> f32 {
        match self {
            BulletKind::ArisOriginal => 1.0,
        }
    }
}



/// 총알의 발사 지연 시간 타이머입니다.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct DelayTimer(pub f32);



/// 클라이언트에서 사용하는 총알의 데이터입니다.
#[derive(Debug, Clone, Copy)]
pub struct Bullet {
    /// 총알의 종류입니다.
    pub kind: BulletKind,

    /// 총알의 위치입니다.
    pub translation: gmm::Vector, 

    /// 총알이 날아가는 방향입니다.
    pub direction: gmm::Vector, 

    /// 총알의 속력입니다.
    pub speed: f32, 

    /// 총알의 최대 사거리입니다.
    pub range: f32, 
}



mod pool;

pub use self::pool::*;
