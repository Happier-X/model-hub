//! 请求日志业务服务。

use std::sync::Arc;

use super::model::{NewRelayLog, RelayLog};
use super::store::{LogStore, LogStoreError};

/// `page_size` 上限（对齐 UI / 侧车 v0.9.28）。
pub const LOG_PAGE_SIZE_MAX: u32 = 100;

#[derive(Clone)]
pub struct LogService {
    store: Arc<LogStore>,
}

impl LogService {
    pub fn new(store: LogStore) -> Self {
        Self {
            store: Arc::new(store),
        }
    }

    pub fn insert(&self, log: &NewRelayLog) -> Result<i64, LogStoreError> {
        self.store.insert(log)
    }

    /// 写入失败只记 warn，不向上抛（避免影响 chat 主路径）。
    pub fn insert_best_effort(&self, log: &NewRelayLog) {
        if let Err(err) = self.insert(log) {
            tracing::warn!(error = %err, "写入请求日志失败");
        }
    }

    /// `page` 从 1；`page_size` clamp 到 1..=100。
    pub fn list(&self, page: u32, page_size: u32) -> Result<Vec<RelayLog>, LogStoreError> {
        let page = page.max(1);
        let page_size = clamp_page_size(page_size);
        self.store.list(page, page_size)
    }

    pub fn clear(&self) -> Result<(), LogStoreError> {
        self.store.clear()
    }
}

pub fn clamp_page_size(page_size: u32) -> u32 {
    if page_size == 0 {
        return 1;
    }
    page_size.min(LOG_PAGE_SIZE_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_page_size_bounds() {
        assert_eq!(clamp_page_size(0), 1);
        assert_eq!(clamp_page_size(20), 20);
        assert_eq!(clamp_page_size(100), 100);
        assert_eq!(clamp_page_size(101), 100);
        assert_eq!(clamp_page_size(9999), 100);
    }
}
