//! 分组名 → 上游渠道与模型解析。

mod round_robin;

use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use serde_json::Value;

use crate::channel::{ChannelError, ChannelService};
use crate::group::{GroupError, GroupService};

pub use round_robin::select_item_index;

/// 解析后的上游转发目标。
#[derive(Debug, Clone)]
pub struct RouteTarget {
    pub group_id: i64,
    pub group_name: String,
    pub channel_id: i64,
    pub upstream_model: String,
    pub base_url: String,
    pub channel_key: String,
    pub custom_header: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteError {
    GroupNotFound,
    EmptyItems,
    NoAvailableChannel,
    Internal(String),
}

impl std::fmt::Display for RouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GroupNotFound => write!(f, "分组不存在"),
            Self::EmptyItems => write!(f, "分组未绑定任何渠道模型"),
            Self::NoAvailableChannel => write!(f, "分组无可用渠道（均已禁用或缺少 base_url/key）"),
            Self::Internal(msg) => write!(f, "{msg}"),
        }
    }
}

/// 路由服务：按分组名选择渠道与上游模型。
#[derive(Clone)]
pub struct RouterService {
    groups: Arc<GroupService>,
    channels: Arc<ChannelService>,
    counter: Arc<AtomicU64>,
}

impl RouterService {
    pub fn new(groups: Arc<GroupService>, channels: Arc<ChannelService>) -> Self {
        Self {
            groups,
            channels,
            counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// 测试用：注入共享计数器。
    pub fn with_counter(
        groups: Arc<GroupService>,
        channels: Arc<ChannelService>,
        counter: Arc<AtomicU64>,
    ) -> Self {
        Self {
            groups,
            channels,
            counter,
        }
    }

    pub fn resolve(&self, group_name: &str) -> Result<RouteTarget, RouteError> {
        let group = self
            .groups
            .find_by_name(group_name)
            .map_err(|err| match err {
                GroupError::NotFound => RouteError::GroupNotFound,
                GroupError::Internal => RouteError::Internal("读取分组失败".into()),
                other => RouteError::Internal(other.to_string()),
            })?;

        if group.items.is_empty() {
            return Err(RouteError::EmptyItems);
        }

        let start = select_item_index(group.mode, group.items.len(), &self.counter);
        let n = group.items.len();

        for offset in 0..n {
            let idx = (start + offset) % n;
            let item = &group.items[idx];
            match self.try_channel(item.channel_id, &item.model_name, &group) {
                Ok(target) => return Ok(target),
                Err(RouteError::NoAvailableChannel) => continue,
                Err(err) => return Err(err),
            }
        }

        Err(RouteError::NoAvailableChannel)
    }

    fn try_channel(
        &self,
        channel_id: i64,
        model_name: &str,
        group: &crate::group::Group,
    ) -> Result<RouteTarget, RouteError> {
        let channel = self.channels.get(channel_id).map_err(|err| match err {
            ChannelError::NotFound => RouteError::NoAvailableChannel,
            ChannelError::Internal => RouteError::Internal("读取渠道失败".into()),
            other => RouteError::Internal(other.to_string()),
        })?;

        if !channel.enabled {
            return Err(RouteError::NoAvailableChannel);
        }

        let base_url = channel
            .base_urls
            .iter()
            .map(|b| b.url.trim())
            .find(|u| !u.is_empty())
            .ok_or(RouteError::NoAvailableChannel)?
            .to_string();

        let channel_key = channel
            .keys
            .iter()
            .find(|k| k.enabled && !k.channel_key.trim().is_empty())
            .map(|k| k.channel_key.clone())
            .ok_or(RouteError::NoAvailableChannel)?;

        Ok(RouteTarget {
            group_id: group.id,
            group_name: group.name.clone(),
            channel_id: channel.id,
            upstream_model: model_name.to_string(),
            base_url,
            channel_key,
            custom_header: channel.custom_header.clone(),
        })
    }
}

/// 拼接上游 chat completions URL。
/// `base` 去尾斜杠后：若已以 `/v1` 结尾则拼 `/chat/completions`，否则拼 `/chat/completions`
/// （与 PRD 一致：`{base_url 去尾斜杠}/chat/completions`；base 已含 `/v1` 时同样拼 `/chat/completions`）。
pub fn build_chat_url(base_url: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    format!("{base}/chat/completions")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn build_chat_url_strips_trailing_slash() {
        assert_eq!(
            build_chat_url("https://api.openai.com/v1/"),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            build_chat_url("https://api.openai.com/v1"),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            build_chat_url("http://127.0.0.1:9"),
            "http://127.0.0.1:9/chat/completions"
        );
    }

    #[test]
    fn round_robin_advances_counter() {
        let counter = AtomicU64::new(0);
        assert_eq!(select_item_index(1, 3, &counter), 0);
        assert_eq!(select_item_index(1, 3, &counter), 1);
        assert_eq!(select_item_index(1, 3, &counter), 2);
        assert_eq!(select_item_index(1, 3, &counter), 0);
        assert_eq!(counter.load(Ordering::Relaxed), 4);
    }

    #[test]
    fn non_round_robin_uses_index_zero() {
        let counter = AtomicU64::new(99);
        assert_eq!(select_item_index(0, 5, &counter), 0);
        assert_eq!(select_item_index(2, 5, &counter), 0);
        assert_eq!(counter.load(Ordering::Relaxed), 99);
    }
}
