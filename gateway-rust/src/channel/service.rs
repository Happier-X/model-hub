//! 渠道业务服务。

use std::sync::Arc;

use super::model::{
    Channel, ChannelError, CreateChannelRequest, EnableChannelRequest, UpdateChannelRequest,
};
use super::store::ChannelStore;

#[derive(Clone)]
pub struct ChannelService {
    store: Arc<ChannelStore>,
}

impl ChannelService {
    pub fn new(store: ChannelStore) -> Self {
        Self {
            store: Arc::new(store),
        }
    }

    pub fn list(&self) -> Result<Vec<Channel>, ChannelError> {
        self.store.list()
    }

    pub fn get(&self, id: i64) -> Result<Channel, ChannelError> {
        self.store.get(id)
    }

    pub fn create(&self, req: CreateChannelRequest) -> Result<Channel, ChannelError> {
        self.store.create(req)
    }

    pub fn update(&self, req: UpdateChannelRequest) -> Result<Channel, ChannelError> {
        self.store.update(req)
    }

    pub fn enable(&self, req: EnableChannelRequest) -> Result<Channel, ChannelError> {
        self.store.set_enabled(req.id, req.enabled)
    }

    pub fn delete(&self, id: i64) -> Result<(), ChannelError> {
        self.store.delete(id)
    }
}
