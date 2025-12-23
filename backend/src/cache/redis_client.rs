use anyhow::Result;
use redis::{aio::ConnectionManager, Client};

#[derive(Clone)]
pub struct RedisClient {
    conn: ConnectionManager,
}

impl RedisClient {
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = Client::open(redis_url)?;
        let conn = ConnectionManager::new(client).await?;
        Ok(Self { conn })
    }

    pub fn connection(&self) -> ConnectionManager {
        self.conn.clone()
    }
}
