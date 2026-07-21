//! 渠道 SQLite 存储。

use rusqlite::{params, OptionalExtension};

use super::model::{
    BaseUrl, Channel, ChannelError, ChannelKey, CreateChannelRequest, KeyUpdate,
    UpdateChannelRequest,
};
use crate::db::DbConn;

#[derive(Clone)]
pub struct ChannelStore {
    db: DbConn,
}

impl ChannelStore {
    pub fn new(db: DbConn) -> Self {
        Self { db }
    }

    pub fn list(&self) -> Result<Vec<Channel>, ChannelError> {
        let conn = self.db.lock().map_err(|_| ChannelError::Internal)?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, type, enabled, model, custom_model, proxy, auto_sync, auto_group, custom_header_json
                 FROM channels ORDER BY id ASC",
            )
            .map_err(|_| ChannelError::Internal)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, String>(9)?,
                ))
            })
            .map_err(|_| ChannelError::Internal)?;

        let mut out = Vec::new();
        for row in rows {
            let (id, name, ty, enabled, model, custom_model, proxy, auto_sync, auto_group, header) =
                row.map_err(|_| ChannelError::Internal)?;
            let base_urls = load_base_urls(&conn, id)?;
            let keys = load_keys(&conn, id)?;
            let custom_header =
                serde_json::from_str(&header).unwrap_or_else(|_| serde_json::json!([]));
            out.push(Channel {
                id,
                name,
                channel_type: ty,
                enabled: enabled != 0,
                base_urls,
                keys,
                model,
                custom_model,
                proxy: proxy != 0,
                auto_sync: auto_sync != 0,
                auto_group,
                custom_header,
            });
        }
        Ok(out)
    }

    pub fn get(&self, id: i64) -> Result<Channel, ChannelError> {
        let conn = self.db.lock().map_err(|_| ChannelError::Internal)?;
        let row = conn
            .query_row(
                "SELECT id, name, type, enabled, model, custom_model, proxy, auto_sync, auto_group, custom_header_json
                 FROM channels WHERE id = ?1",
                [id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, i64>(8)?,
                        row.get::<_, String>(9)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| ChannelError::Internal)?
            .ok_or(ChannelError::NotFound)?;

        let (id, name, ty, enabled, model, custom_model, proxy, auto_sync, auto_group, header) =
            row;
        let base_urls = load_base_urls(&conn, id)?;
        let keys = load_keys(&conn, id)?;
        let custom_header = serde_json::from_str(&header).unwrap_or_else(|_| serde_json::json!([]));
        Ok(Channel {
            id,
            name,
            channel_type: ty,
            enabled: enabled != 0,
            base_urls,
            keys,
            model,
            custom_model,
            proxy: proxy != 0,
            auto_sync: auto_sync != 0,
            auto_group,
            custom_header,
        })
    }

    pub fn create(&self, req: CreateChannelRequest) -> Result<Channel, ChannelError> {
        let name = req.name.trim().to_string();
        if name.is_empty() {
            return Err(ChannelError::InvalidName);
        }
        let header_json = serde_json::to_string(&req.custom_header).unwrap_or_else(|_| "[]".into());

        let conn = self.db.lock().map_err(|_| ChannelError::Internal)?;
        conn.execute(
            "INSERT INTO channels (name, type, enabled, model, custom_model, proxy, auto_sync, auto_group, custom_header_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                name,
                req.channel_type,
                if req.enabled { 1 } else { 0 },
                req.model,
                req.custom_model,
                if req.proxy { 1 } else { 0 },
                if req.auto_sync { 1 } else { 0 },
                req.auto_group,
                header_json,
            ],
        )
        .map_err(|_| ChannelError::Internal)?;
        let id = conn.last_insert_rowid();

        for (i, bu) in req.base_urls.iter().enumerate() {
            conn.execute(
                "INSERT INTO channel_base_urls (channel_id, url, delay, sort_order) VALUES (?1, ?2, ?3, ?4)",
                params![id, bu.url, bu.delay, i as i64],
            )
            .map_err(|_| ChannelError::Internal)?;
        }
        for key in &req.keys {
            // 上游 channel_key 本机可明文存；日志禁止打印完整 Key
            conn.execute(
                "INSERT INTO channel_keys (channel_id, enabled, channel_key, remark) VALUES (?1, ?2, ?3, ?4)",
                params![
                    id,
                    if key.enabled { 1 } else { 0 },
                    key.channel_key,
                    key.remark,
                ],
            )
            .map_err(|_| ChannelError::Internal)?;
        }
        drop(conn);
        self.get(id)
    }

    pub fn update(&self, req: UpdateChannelRequest) -> Result<Channel, ChannelError> {
        let conn = self.db.lock().map_err(|_| ChannelError::Internal)?;
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM channels WHERE id = ?1)",
                [req.id],
                |row| row.get(0),
            )
            .map_err(|_| ChannelError::Internal)?;
        if !exists {
            return Err(ChannelError::NotFound);
        }

        if let Some(name) = &req.name {
            let name = name.trim();
            if name.is_empty() {
                return Err(ChannelError::InvalidName);
            }
            conn.execute(
                "UPDATE channels SET name = ?1 WHERE id = ?2",
                params![name, req.id],
            )
            .map_err(|_| ChannelError::Internal)?;
        }
        if let Some(ty) = req.channel_type {
            conn.execute(
                "UPDATE channels SET type = ?1 WHERE id = ?2",
                params![ty, req.id],
            )
            .map_err(|_| ChannelError::Internal)?;
        }
        if let Some(enabled) = req.enabled {
            conn.execute(
                "UPDATE channels SET enabled = ?1 WHERE id = ?2",
                params![if enabled { 1 } else { 0 }, req.id],
            )
            .map_err(|_| ChannelError::Internal)?;
        }
        if let Some(model) = &req.model {
            conn.execute(
                "UPDATE channels SET model = ?1 WHERE id = ?2",
                params![model, req.id],
            )
            .map_err(|_| ChannelError::Internal)?;
        }
        if let Some(custom_model) = &req.custom_model {
            conn.execute(
                "UPDATE channels SET custom_model = ?1 WHERE id = ?2",
                params![custom_model, req.id],
            )
            .map_err(|_| ChannelError::Internal)?;
        }
        if let Some(proxy) = req.proxy {
            conn.execute(
                "UPDATE channels SET proxy = ?1 WHERE id = ?2",
                params![if proxy { 1 } else { 0 }, req.id],
            )
            .map_err(|_| ChannelError::Internal)?;
        }
        if let Some(auto_sync) = req.auto_sync {
            conn.execute(
                "UPDATE channels SET auto_sync = ?1 WHERE id = ?2",
                params![if auto_sync { 1 } else { 0 }, req.id],
            )
            .map_err(|_| ChannelError::Internal)?;
        }
        if let Some(auto_group) = req.auto_group {
            conn.execute(
                "UPDATE channels SET auto_group = ?1 WHERE id = ?2",
                params![auto_group, req.id],
            )
            .map_err(|_| ChannelError::Internal)?;
        }
        if let Some(header) = &req.custom_header {
            let header_json = serde_json::to_string(header).unwrap_or_else(|_| "[]".into());
            conn.execute(
                "UPDATE channels SET custom_header_json = ?1 WHERE id = ?2",
                params![header_json, req.id],
            )
            .map_err(|_| ChannelError::Internal)?;
        }
        if let Some(base_urls) = &req.base_urls {
            conn.execute(
                "DELETE FROM channel_base_urls WHERE channel_id = ?1",
                [req.id],
            )
            .map_err(|_| ChannelError::Internal)?;
            for (i, bu) in base_urls.iter().enumerate() {
                conn.execute(
                    "INSERT INTO channel_base_urls (channel_id, url, delay, sort_order) VALUES (?1, ?2, ?3, ?4)",
                    params![req.id, bu.url, bu.delay, i as i64],
                )
                .map_err(|_| ChannelError::Internal)?;
            }
        }

        for ku in &req.keys_to_update {
            apply_key_update(&conn, req.id, ku)?;
        }
        for key in &req.keys_to_add {
            conn.execute(
                "INSERT INTO channel_keys (channel_id, enabled, channel_key, remark) VALUES (?1, ?2, ?3, ?4)",
                params![
                    req.id,
                    if key.enabled { 1 } else { 0 },
                    key.channel_key,
                    key.remark,
                ],
            )
            .map_err(|_| ChannelError::Internal)?;
        }
        drop(conn);
        self.get(req.id)
    }

    pub fn set_enabled(&self, id: i64, enabled: bool) -> Result<Channel, ChannelError> {
        let conn = self.db.lock().map_err(|_| ChannelError::Internal)?;
        let n = conn
            .execute(
                "UPDATE channels SET enabled = ?1 WHERE id = ?2",
                params![if enabled { 1 } else { 0 }, id],
            )
            .map_err(|_| ChannelError::Internal)?;
        if n == 0 {
            return Err(ChannelError::NotFound);
        }
        drop(conn);
        self.get(id)
    }

    pub fn delete(&self, id: i64) -> Result<(), ChannelError> {
        let conn = self.db.lock().map_err(|_| ChannelError::Internal)?;
        let n = conn
            .execute("DELETE FROM channels WHERE id = ?1", [id])
            .map_err(|_| ChannelError::Internal)?;
        if n == 0 {
            return Err(ChannelError::NotFound);
        }
        Ok(())
    }
}

fn load_base_urls(
    conn: &rusqlite::Connection,
    channel_id: i64,
) -> Result<Vec<BaseUrl>, ChannelError> {
    let mut stmt = conn
        .prepare(
            "SELECT url, delay FROM channel_base_urls WHERE channel_id = ?1 ORDER BY sort_order ASC, id ASC",
        )
        .map_err(|_| ChannelError::Internal)?;
    let rows = stmt
        .query_map([channel_id], |row| {
            Ok(BaseUrl {
                url: row.get(0)?,
                delay: row.get(1)?,
            })
        })
        .map_err(|_| ChannelError::Internal)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|_| ChannelError::Internal)?);
    }
    Ok(out)
}

fn load_keys(
    conn: &rusqlite::Connection,
    channel_id: i64,
) -> Result<Vec<ChannelKey>, ChannelError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, channel_id, enabled, channel_key, remark FROM channel_keys WHERE channel_id = ?1 ORDER BY id ASC",
        )
        .map_err(|_| ChannelError::Internal)?;
    let rows = stmt
        .query_map([channel_id], |row| {
            Ok(ChannelKey {
                id: Some(row.get(0)?),
                channel_id: Some(row.get(1)?),
                enabled: row.get::<_, i64>(2)? != 0,
                channel_key: row.get(3)?,
                remark: row.get(4)?,
            })
        })
        .map_err(|_| ChannelError::Internal)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|_| ChannelError::Internal)?);
    }
    Ok(out)
}

fn apply_key_update(
    conn: &rusqlite::Connection,
    channel_id: i64,
    ku: &KeyUpdate,
) -> Result<(), ChannelError> {
    let owned: Option<(i64,)> = conn
        .query_row(
            "SELECT id FROM channel_keys WHERE id = ?1 AND channel_id = ?2",
            params![ku.id, channel_id],
            |row| Ok((row.get(0)?,)),
        )
        .optional()
        .map_err(|_| ChannelError::Internal)?;
    if owned.is_none() {
        return Err(ChannelError::InvalidInput(format!(
            "渠道 Key {} 不存在或不属于该渠道",
            ku.id
        )));
    }

    conn.execute(
        "UPDATE channel_keys SET channel_key = ?1 WHERE id = ?2",
        params![ku.channel_key, ku.id],
    )
    .map_err(|_| ChannelError::Internal)?;

    if let Some(enabled) = ku.enabled {
        conn.execute(
            "UPDATE channel_keys SET enabled = ?1 WHERE id = ?2",
            params![if enabled { 1 } else { 0 }, ku.id],
        )
        .map_err(|_| ChannelError::Internal)?;
    }
    if let Some(remark) = &ku.remark {
        conn.execute(
            "UPDATE channel_keys SET remark = ?1 WHERE id = ?2",
            params![remark, ku.id],
        )
        .map_err(|_| ChannelError::Internal)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_tempfile_db;

    fn sample_create(name: &str) -> CreateChannelRequest {
        CreateChannelRequest {
            name: name.into(),
            channel_type: 0,
            enabled: true,
            base_urls: vec![BaseUrl {
                url: "https://api.openai.com/v1".into(),
                delay: 0,
            }],
            keys: vec![ChannelKey {
                id: None,
                channel_id: None,
                enabled: true,
                channel_key: "sk-upstream-test".into(),
                remark: "smoke".into(),
            }],
            model: "gpt-4o-mini".into(),
            custom_model: "".into(),
            proxy: false,
            auto_sync: false,
            auto_group: 0,
            custom_header: serde_json::json!([]),
        }
    }

    #[test]
    fn crud_and_key_rotation() {
        let (db, _dir) = open_tempfile_db();
        let store = ChannelStore::new(db);

        let ch = store.create(sample_create("c1")).unwrap();
        assert_eq!(ch.channel_type, 0);
        assert_eq!(ch.base_urls.len(), 1);
        assert_eq!(ch.keys.len(), 1);
        let key_id = ch.keys[0].id.unwrap();

        let list = store.list().unwrap();
        assert_eq!(list.len(), 1);

        let updated = store
            .update(UpdateChannelRequest {
                id: ch.id,
                name: Some("c1-renamed".into()),
                channel_type: None,
                enabled: None,
                base_urls: Some(vec![BaseUrl {
                    url: "https://example.com/v1".into(),
                    delay: 10,
                }]),
                model: Some("gpt-4o".into()),
                custom_model: None,
                proxy: None,
                auto_sync: None,
                auto_group: None,
                custom_header: None,
                keys_to_update: vec![KeyUpdate {
                    id: key_id,
                    channel_key: "sk-rotated".into(),
                    enabled: None,
                    remark: None,
                }],
                keys_to_add: vec![ChannelKey {
                    id: None,
                    channel_id: None,
                    enabled: true,
                    channel_key: "sk-extra".into(),
                    remark: "".into(),
                }],
            })
            .unwrap();
        assert_eq!(updated.name, "c1-renamed");
        assert_eq!(updated.model, "gpt-4o");
        assert_eq!(updated.base_urls[0].url, "https://example.com/v1");
        assert_eq!(updated.keys.len(), 2);
        assert_eq!(updated.keys[0].channel_key, "sk-rotated");

        let disabled = store.set_enabled(ch.id, false).unwrap();
        assert!(!disabled.enabled);

        store.delete(ch.id).unwrap();
        assert!(store.list().unwrap().is_empty());
        assert_eq!(store.get(ch.id), Err(ChannelError::NotFound));
    }
}
