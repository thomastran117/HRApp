use anyhow::Result;
use redis::AsyncCommands;
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;
use uuid::Uuid;

use crate::redis_client::RedisClient;

#[derive(Clone)]
pub struct CacheService {
    redis: RedisClient,
    prefix: String,
}

impl CacheService {
    pub fn new(redis: RedisClient, prefix: impl Into<String>) -> Self {
        Self {
            redis,
            prefix: prefix.into(),
        }
    }

    fn key(&self, key: &str) -> String {
        format!("{}:{}", self.prefix, key)
    }

    pub async fn set<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl: Option<Duration>,
    ) -> Result<()> {
        let mut conn = self.redis.connection();
        let payload = serde_json::to_string(value)?;

        match ttl {
            Some(ttl) => conn
                .set_ex(self.key(key), payload, ttl.as_secs() as usize)
                .await?,
            None => conn.set(self.key(key), payload).await?,
        }

        Ok(())
    }

    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self.redis.connection();
        let value: Option<String> = conn.get(self.key(key)).await?;

        match value {
            Some(v) => Ok(Some(serde_json::from_str(&v)?)),
            None => Ok(None),
        }
    }

    pub async fn delete(&self, key: &str) -> Result<()> {
        let mut conn = self.redis.connection();
        conn.del(self.key(key)).await?;
        Ok(())
    }

    pub async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn = self.redis.connection();
        Ok(conn.exists(self.key(key)).await?)
    }

    pub async fn increment(&self, key: &str, by: i64, ttl: Option<Duration>) -> Result<i64> {
        let mut conn = self.redis.connection();
        let full_key = self.key(key);

        let value: i64 = conn.incr(&full_key, by).await?;

        if value == by {
            if let Some(ttl) = ttl {
                conn.expire(&full_key, ttl.as_secs() as usize).await?;
            }
        }

        Ok(value)
    }

    pub async fn decrement(&self, key: &str, by: i64) -> Result<i64> {
        let mut conn = self.redis.connection();
        Ok(conn.decr(self.key(key), by).await?)
    }

    pub async fn set_if_not_exists(
        &self,
        key: &str,
        value: &str,
        ttl: Duration,
    ) -> Result<bool> {
        let mut conn = self.redis.connection();
        let result: bool = conn
            .set_nx(self.key(key), value)
            .await?;

        if result {
            conn.expire(self.key(key), ttl.as_secs() as usize)
                .await?;
        }

        Ok(result)
    }

    pub async fn acquire_lock(
        &self,
        key: &str,
        ttl: Duration,
    ) -> Result<Option<String>> {
        let lock_value = Uuid::new_v4().to_string();
        let mut conn = self.redis.connection();

        let acquired: bool = conn
            .set_nx(self.key(key), &lock_value)
            .await?;

        if acquired {
            conn.expire(self.key(key), ttl.as_secs() as usize)
                .await?;
            Ok(Some(lock_value))
        } else {
            Ok(None)
        }
    }

    pub async fn release_lock(
        &self,
        key: &str,
        lock_value: &str,
    ) -> Result<()> {
        let mut conn = self.redis.connection();
        let current: Option<String> = conn.get(self.key(key)).await?;

        if current.as_deref() == Some(lock_value) {
            conn.del(self.key(key)).await?;
        }

        Ok(())
    }

    pub async fn blacklist_token(
        &self,
        jti: &str,
        ttl: Duration,
    ) -> Result<()> {
        self.set(
            &format!("jwt:blacklist:{jti}"),
            &true,
            Some(ttl),
        )
        .await
    }

    pub async fn is_token_blacklisted(&self, jti: &str) -> Result<bool> {
        self.exists(&format!("jwt:blacklist:{jti}")).await
    }
}
