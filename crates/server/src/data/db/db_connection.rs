use futures::executor::block_on;
use mod_network::components::{InGamePlayerResultData, Team, UserId};
use redis::{
    aio::ConnectionManager,
    AsyncCommands,
    RedisResult,
};
use super::UserInfo;


lazy_static::lazy_static!(
    static ref DB_CONNECTION: DbConnection = DbConnection::new();
);


#[derive(Clone)]
pub struct DbConnection {
    manager: ConnectionManager,
}

impl DbConnection {
    fn new() -> Self {
        log::info!("Database connection initialized.");
        
        let client = redis::Client::open("redis://127.0.0.1/")
            .expect("Failed to create Redis client");
        let manager = block_on(ConnectionManager::new(client))
            .expect("Failed to create Redis connection manager");

        DbConnection { manager }
    }

    pub fn get_connection() -> Self {
        // Return the existing database connection
        DB_CONNECTION.clone()
    }
}

impl DbConnection {
    pub async fn save(&self) -> RedisResult<()> {
        let mut conn = self.manager.clone();

        redis::cmd("BGSAVE").query_async::<()>(&mut conn).await?;
        
        Ok(())
    }

    /// 다음 uid를 가져옵니다.  
    /// 고유한 uid를 유지할 수 있도록 save를 자동으로 호출합니다.
    pub async fn get_next_uid(&self) -> RedisResult<UserId> {
        let mut conn = self.manager.clone();

        // DB에 "next_uid" 키가 없으면 1로 초기화 된다.
        let next_uid: u32 = conn.incr("next_uid", 1).await?;

        self.save().await?;

        Ok(UserId::new(next_uid))
    }

    pub async fn set_user_info(&self, uid: &UserId, user_info: &UserInfo) -> RedisResult<()> {
        let mut conn = self.manager.clone();

        let key = format!("user_info:{}", uid);

        let _: () = conn.hset(&key, "name", &user_info.name.to_string()).await?;
        let _: () = conn.hset(&key, "tier", user_info.tier as u8).await?;
        let _: () = conn.hset(&key, "profile_icon", user_info.profile_icon as u8).await?;

        Ok(())
    }

    pub async fn get_user_info(&self, uid: &UserId) -> RedisResult<Option<UserInfo>> {
        let mut conn = self.manager.clone();

        let key = format!("user_info:{}", uid);
        
        let user_info: UserInfo = conn.hgetall(&key).await?;
        if user_info.name.is_empty() {
            Ok(None)
        } else {
            Ok(Some(user_info))
        }
    }

    pub async fn save_game_result(&self, 
        play_time_ms: u32,
        winner: Option<Team>,
        result_data: &Vec<InGamePlayerResultData>
    ) -> RedisResult<()> {
        let mut conn = self.manager.clone();

        let next_match_id: u32 = conn.incr("next_match_id", 1).await?;
        let key = format!("match:{}", next_match_id);

        let _: () = conn.hset(&key, "play_time_s", play_time_ms / 1000).await?;
        let _: () = conn.hset(&key, "winner", winner.map_or(2, |w| w as u8)).await?;

        for data in result_data {
            let key = format!("{}:user_info:{}", key, data.uid);
            
            let _: () = conn.hset(&key, "character", data.character_kind as u8).await?;
            let _: () = conn.hset(&key, "kills", data.kill_count).await?;
            let _: () = conn.hset(&key, "deaths", data.retreat_count).await?;
            let _: () = conn.hset(&key, "damage_done", data.damage_dealt).await?;
            let _: () = conn.hset(&key, "damage_taken", data.damage_taken).await?;
            let _: () = conn.hset(&key, "healing_done", data.healing_given).await?;

            let player_key = format!("user_info:{}:matches", data.uid);
            let _: () = conn.rpush(&player_key, next_match_id).await?;
        }

        Ok(())
    }
}