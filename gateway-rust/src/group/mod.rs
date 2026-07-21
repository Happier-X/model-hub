//! 分组管理。

mod model;
mod service;
mod store;

pub use model::{CreateGroupRequest, Group, GroupError, GroupItem, UpdateGroupRequest};
pub use service::GroupService;
pub use store::GroupStore;
