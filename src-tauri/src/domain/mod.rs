pub mod group;
pub mod leaderboard;
pub mod log;
pub mod pricing;
pub mod provider;
pub mod upstream_models;

use std::sync::{Arc, Mutex};

use crate::db::DbConn;
use crate::error::AppError;

#[derive(Clone)]
pub struct Stores {
    pub db: DbConn,
    /// 变更订阅回调：insert_log 写入成功后触发（领域层不依赖 tauri，由外层转成事件推送）。
    change_listeners: Arc<Mutex<Vec<Arc<dyn Fn() + Send + Sync>>>>,
}

impl Stores {
    pub fn new(db: DbConn) -> Self {
        let stores = Self {
            db,
            change_listeners: Arc::new(Mutex::new(Vec::new())),
        };
        // 旧库升级：聚合表为空时从现存明细幂等重建（失败仅告警，下次启动重试；不阻断启动）。
        if let Err(error) = stores.backfill_daily_stats() {
            tracing::warn!(%error, "回填 daily_request_stats 失败");
        }
        stores
    }

    /// 注册变更回调，监听请求日志写入。锁异常时静默跳过，不影响业务主路径。
    pub fn subscribe_change(&self, listener: impl Fn() + Send + Sync + 'static) {
        if let Ok(mut listeners) = self.change_listeners.lock() {
            listeners.push(Arc::new(listener));
        }
    }

    /// 通知全部订阅者：克隆监听列表后**锁外**调用，避免回调重入（回调内再写库/加锁）死锁。
    fn notify_changed(&self) {
        let listeners = match self.change_listeners.lock() {
            Ok(l) => l.clone(),
            Err(_) => return,
        };
        for f in &listeners {
            f();
        }
    }

    pub fn with_conn<T>(
        &self,
        f: impl FnOnce(&rusqlite::Connection) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        let guard = self.db.lock().map_err(|_| AppError::LockPoisoned)?;
        f(&guard)
    }
}
