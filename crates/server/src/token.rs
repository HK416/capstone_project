use std::{
    net::SocketAddr,
    sync::{
        OnceLock,
        atomic::{AtomicU32, Ordering as MemOrdering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use mod_network::components::{LoginToken, UserId};
use mod_parallelism::collections::SkipMap;
use tokio::time::{Duration, Instant};

/// 사용자 계정의 로그인 토큰입니다.
static TOKENS: OnceLock<SkipMap<(UserId, SocketAddr), (LoginToken, Instant)>> = OnceLock::new();

/// 사용자 계정과 로그인 토큰 집합을 가져옵니다.
fn get_tokens() -> &'static SkipMap<(UserId, SocketAddr), (LoginToken, Instant)> {
    TOKENS.get_or_init(|| SkipMap::default())
}

/// 로그인된 사용자의 로그인 토큰을 관리합니다.  
/// 실제 데이터는 전역 변수에 저장되며 `UserTokenMap`는 전역 변수에 접근할 수 있는 인터페이스를 제공합니다.
pub struct UserTokenMap;

impl UserTokenMap {
    /// 사용자의 로그인 토큰을 할당받습니다.
    pub fn alloc(key: (UserId, SocketAddr)) -> LoginToken {
        let token = generate_token();
        let time_point = Instant::now();
        get_tokens().insert(key, (token, time_point));
        token
    }

    /// 사용자의 로그인 토큰을 확인합니다.
    pub fn is_valid(key: &(UserId, SocketAddr), token: LoginToken) -> bool {
        get_tokens().get(key).is_some_and(|item| {
            // 로그인 토큰이 일치하는지, 토큰 발행 시간이 30분을 넘기지 않았는지 확인합니다.
            item.0 == token && item.1.elapsed() < Duration::from_secs(1800)
        })
    }

    /// 등록된 사용자의 로그인 토큰을 제거합니다.
    pub fn remove(key: &(UserId, SocketAddr)) {
        get_tokens().remove(key);
    }
}

/// 무작위의 로그인 토큰을 발행합니다.
fn generate_token() -> LoginToken {
    /// 난수를 생성하기 위한 카운터입니다.
    /// 해당 함수를 호출할 때 마다 1씩 증가합니다.
    static COUNTER: AtomicU32 = AtomicU32::new(0);

    let now = SystemTime::now();
    let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();

    let counter_bit = COUNTER.fetch_add(1, MemOrdering::AcqRel) as u64 & 0xFFFFFF;
    let time_bit = duration.subsec_micros() as u64 & 0xFFFFFF;
    let rand_bit = rand::random::<u64>() & 0xFFFF;

    LoginToken::new((rand_bit << 48) | (time_bit << 24) | counter_bit)
}
