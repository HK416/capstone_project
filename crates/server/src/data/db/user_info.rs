use redis::{FromRedisValue, RedisError, RedisResult};


#[derive(Debug)]
pub struct UserInfo {
    pub name: String,
    pub tier: u8,
    pub profile_icon: u8,
}

impl FromRedisValue for UserInfo {
    fn from_redis_value(value: &redis::Value) -> RedisResult<Self> {
        if let redis::Value::Array(data) = value {
            let mut user_info = UserInfo {
                name: String::new(),
                tier: 0,
                profile_icon: 0,
            };

            let iter = data.chunks_exact(2);
            for chunk in iter {
                if let [key, val] = chunk {
                    let key_str: String = redis::from_redis_value(key)?;
                    match key_str.as_str() {
                        "name" => user_info.name = redis::from_redis_value(val)?,
                        "tier" => user_info.tier = redis::from_redis_value(val)?,
                        "prifile_icon" => user_info.profile_icon = redis::from_redis_value(val)?,
                        _ => {}
                    }
                }
                else {
                    return Err(RedisError::from((
                        redis::ErrorKind::TypeError, 
                        "Expected key-value pairs in array"
                    )));
                }
            }

            Ok(user_info)
        }
        else {
            Err(RedisError::from((
                redis::ErrorKind::TypeError, 
                "Cannot convert to UserInfo"
            )))
        }
    }
}