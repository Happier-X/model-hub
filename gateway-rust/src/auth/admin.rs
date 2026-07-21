//! 默认 admin 账号校验与 JWT 服务封装。

use std::sync::Arc;

use crate::config::AuthConfig;

use super::jwt::JwtService;

#[derive(Debug, Clone)]
pub struct AuthService {
    username: String,
    password: String,
    jwt: Arc<JwtService>,
}

impl AuthService {
    pub fn from_config(auth: &AuthConfig) -> Self {
        let secret = auth.resolved_jwt_secret();
        if auth.jwt_secret.is_none() {
            tracing::warn!("auth.jwt_secret 未配置，已生成随机密钥；进程重启后管理 Token 将失效");
        }
        Self {
            username: auth.admin_username.clone(),
            password: auth.admin_password.clone(),
            jwt: Arc::new(JwtService::new(secret, auth.jwt_default_expire_seconds)),
        }
    }

    pub fn for_tests(username: &str, password: &str, secret: &str) -> Self {
        Self {
            username: username.to_string(),
            password: password.to_string(),
            jwt: Arc::new(JwtService::new(secret, 86400)),
        }
    }

    pub fn verify_password(&self, username: &str, password: &str) -> bool {
        // 默认实验账号，简单常量时间比较（长度先对齐到固定缓冲区会更复杂，此处用 equal）
        username == self.username && password == self.password
    }

    pub fn issue_token(
        &self,
        subject: &str,
        expire_seconds: Option<u64>,
    ) -> Result<(String, i64), crate::error::GatewayError> {
        self.jwt.issue(subject, expire_seconds)
    }

    pub fn verify_token(
        &self,
        token: &str,
    ) -> Result<super::jwt::AdminClaims, crate::error::GatewayError> {
        self.jwt.verify(token)
    }

    pub fn jwt(&self) -> &JwtService {
        &self.jwt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_admin_password_check() {
        let auth = AuthService::for_tests("admin", "admin", "secret");
        assert!(auth.verify_password("admin", "admin"));
        assert!(!auth.verify_password("admin", "wrong"));
        assert!(!auth.verify_password("other", "admin"));
    }
}
