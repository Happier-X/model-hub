//! HS256 管理 JWT 签发与校验。

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::error::GatewayError;

/// 管理 JWT claims。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminClaims {
    pub sub: String,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Debug, Clone)]
pub struct JwtService {
    secret: String,
    default_expire_seconds: u64,
}

impl JwtService {
    pub fn new(secret: impl Into<String>, default_expire_seconds: u64) -> Self {
        Self {
            secret: secret.into(),
            default_expire_seconds: default_expire_seconds.max(60),
        }
    }

    pub fn default_expire_seconds(&self) -> u64 {
        self.default_expire_seconds
    }

    /// 签发管理 JWT，返回 (token, expire_at unix 秒)。
    pub fn issue(
        &self,
        subject: &str,
        expire_seconds: Option<u64>,
    ) -> Result<(String, i64), GatewayError> {
        let now = Utc::now();
        let ttl = expire_seconds
            .unwrap_or(self.default_expire_seconds)
            .max(60);
        let exp = now + Duration::seconds(ttl as i64);
        let claims = AdminClaims {
            sub: subject.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|err| GatewayError::invalid_config(format!("签发 JWT 失败: {err}")))?;
        Ok((token, claims.exp))
    }

    pub fn verify(&self, token: &str) -> Result<AdminClaims, GatewayError> {
        let data = decode::<AdminClaims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| GatewayError::invalid_config("无效或过期的管理 Token"))?;
        Ok(data.claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_and_verify_roundtrip() {
        let jwt = JwtService::new("test-secret", 3600);
        let (token, exp) = jwt.issue("admin", Some(120)).unwrap();
        assert!(!token.is_empty());
        assert!(exp > Utc::now().timestamp());
        let claims = jwt.verify(&token).unwrap();
        assert_eq!(claims.sub, "admin");
    }

    #[test]
    fn bad_token_fails() {
        let jwt = JwtService::new("test-secret", 3600);
        assert!(jwt.verify("not.a.jwt").is_err());
        let other = JwtService::new("other-secret", 3600);
        let (token, _) = jwt.issue("admin", None).unwrap();
        assert!(other.verify(&token).is_err());
    }
}
