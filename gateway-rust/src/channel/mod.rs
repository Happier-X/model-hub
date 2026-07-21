//! 渠道管理。

mod model;
mod service;
mod store;

pub use model::{
    BaseUrl, Channel, ChannelError, ChannelKey, CreateChannelRequest, EnableChannelRequest,
    KeyUpdate, UpdateChannelRequest,
};
pub use service::ChannelService;
pub use store::ChannelStore;
