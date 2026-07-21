//! HTTP 路由处理器。

mod admin_user;
mod apikey;
mod channel;
mod group;
mod v1_chat;
mod v1_models;

pub use admin_user::{login_handler, status_handler};
pub use apikey::{
    create_apikey_handler, delete_apikey_handler, list_apikey_handler, update_apikey_handler,
};
pub use channel::{
    create_channel_handler, delete_channel_handler, enable_channel_handler, list_channel_handler,
    update_channel_handler,
};
pub use group::{
    create_group_handler, delete_group_handler, list_group_handler, update_group_handler,
};
pub use v1_chat::chat_completions_handler;
pub use v1_models::models_handler;
