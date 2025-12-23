use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

use anyhow::Result;


#[derive(Clone)]
pub struct TokenService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    access_token_ttl: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub sub: String,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Debug)]
pub struct RefreshToken {
    pub session_id: Uuid,
    pub secret: String,
}

#[derive(Debug)]
pub struct RefreshTokenHash {
    pub session_id: Uuid,
    pub hash: String,
}


impl TokenService {
    pub fn new(jwt_secret: &str, access_token_ttl: Duration) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(jwt_secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(jwt_secret.as_bytes()),
            access_token_ttl,
        }
    }

    pub fn issue_access_token(
        &self,
        user_id: impl Into<String>,
        role: impl Into<String>,
    ) -> Result<String> {
        let now = current_timestamp();
        let exp = now + self.access_token_ttl.as_secs() as usize;

        let claims = AccessTokenClaims {
            sub: user_id.into(),
            role: role.into(),
            iat: now,
            exp,
        };

        Ok(encode(&Header::default(), &claims, &self.encoding_key)?)
    }

    pub fn verify_access_token(
        &self,
        token: &str,
    ) -> Result<AccessTokenClaims> {
        let data = decode::<AccessTokenClaims>(
            token,
            &self.decoding_key,
            &Validation::default(),
        )?;

        Ok(data.claims)
    }

    pub fn create_refresh_token(&self) -> (RefreshToken, RefreshTokenHash) {
        let session_id = Uuid::new_v4();
        let secret = generate_secret();

        let hash = hash_secret(&secret);

        (
            RefreshToken {
                session_id,
                secret: secret.clone(),
            },
            RefreshTokenHash {
                session_id,
                hash,
            },
        )
    }

    pub fn format_refresh_token(
        &self,
        session_id: Uuid,
        secret: &str,
    ) -> String {
        format!("{}.{}", session_id, secret)
    }

    pub fn parse_refresh_token(token: &str) -> Option<RefreshToken> {
        let (id, secret) = token.split_once('.')?;
        let session_id = Uuid::parse_str(id).ok()?;

        Some(RefreshToken {
            session_id,
            secret: secret.to_string(),
        })
    }

    pub fn verify_refresh_secret(
        &self,
        secret: &str,
        stored_hash: &str,
    ) -> bool {
        verify_secret(secret, stored_hash)
    }
}

fn current_timestamp() -> usize {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_secs() as usize
}

fn generate_secret() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    base64::encode(bytes)
}

fn hash_secret(secret: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(secret.as_bytes(), &salt)
        .expect("hashing failed")
        .to_string()
}

fn verify_secret(secret: &str, hash: &str) -> bool {
    let parsed = PasswordHash::new(hash);
    if parsed.is_err() {
        return false;
    }

    Argon2::default()
        .verify_password(secret.as_bytes(), &parsed.unwrap())
        .is_ok()
}
