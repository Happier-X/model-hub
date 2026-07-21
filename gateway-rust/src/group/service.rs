//! 分组业务服务。

use std::sync::Arc;

use super::model::{CreateGroupRequest, Group, GroupError, UpdateGroupRequest};
use super::store::GroupStore;

#[derive(Clone)]
pub struct GroupService {
    store: Arc<GroupStore>,
}

impl GroupService {
    pub fn new(store: GroupStore) -> Self {
        Self {
            store: Arc::new(store),
        }
    }

    pub fn list(&self) -> Result<Vec<Group>, GroupError> {
        self.store.list()
    }

    pub fn get(&self, id: i64) -> Result<Group, GroupError> {
        self.store.get(id)
    }

    pub fn find_by_name(&self, name: &str) -> Result<Group, GroupError> {
        self.store.find_by_name(name)
    }

    pub fn create(&self, req: CreateGroupRequest) -> Result<Group, GroupError> {
        self.store.create(req)
    }

    pub fn update(&self, req: UpdateGroupRequest) -> Result<Group, GroupError> {
        self.store.update(req)
    }

    pub fn delete(&self, id: i64) -> Result<(), GroupError> {
        self.store.delete(id)
    }
}
