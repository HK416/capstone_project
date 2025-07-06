use futures::executor::block_on;
use mod_network::components::UserId;
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
}