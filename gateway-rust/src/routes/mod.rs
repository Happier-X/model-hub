//! HTTP 路由处理器。

mod admin_user;
mod apikey;
mod v1_models;

pub use admin_user::{login_handler, status_handler};
pub use apikey::{
    create_apikey_handler, delete_apikey_handler, list_apikey_handler, update_apikey_handler,
};
pub use v1_models::models_handler;
