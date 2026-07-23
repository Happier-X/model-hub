use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

use super::Stores;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupItem {
    pub id: i64,
    pub provider_id: i64,
    pub provider_name: Option<String>,
    pub upstream_model: String,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: i64,
    pub name: String,
    pub items: Vec<GroupItem>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct GroupItemInput {
    pub provider_id: i64,
    pub upstream_model: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateGroupPayload {
    pub name: String,
    pub items: Vec<GroupItemInput>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGroupPayload {
    pub id: i64,
    pub name: String,
    pub items: Vec<GroupItemInput>,
}

impl Stores {
    fn load_items(conn: &rusqlite::Connection, group_id: i64) -> Result<Vec<GroupItem>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT gi.id, gi.provider_id, p.name, gi.upstream_model, gi.sort_order
                 FROM group_items gi
                 LEFT JOIN providers p ON p.id = gi.provider_id
                 WHERE gi.group_id = ?1
                 ORDER BY gi.sort_order ASC, gi.id ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        let rows = stmt
            .query_map([group_id], |row| {
                Ok(GroupItem {
                    id: row.get(0)?,
                    provider_id: row.get(1)?,
                    provider_name: row.get(2)?,
                    upstream_model: row.get(3)?,
                    sort_order: row.get(4)?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| AppError::Database(e.to_string()))?);
        }
        Ok(out)
    }

    fn replace_items(
        conn: &rusqlite::Connection,
        group_id: i64,
        items: &[GroupItemInput],
    ) -> Result<(), AppError> {
        conn.execute("DELETE FROM group_items WHERE group_id = ?1", [group_id])
            .map_err(|e| AppError::Database(e.to_string()))?;

        // 旧 gateway-rust 表仍有 channel_id/model_name NOT NULL，迁移采用加列保留旧表，
        // 因此应用写入时需同步填充旧列；新表仅写当前列。
        let mut stmt = conn
            .prepare("PRAGMA table_info(group_items)")
            .map_err(|e| AppError::Database(format!("检查 group_items 表结构失败: {e}")))?;
        let column_rows = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| AppError::Database(format!("读取 group_items 表结构失败: {e}")))?;
        let mut columns = std::collections::HashSet::new();
        for row in column_rows {
            columns.insert(row.map_err(|e| AppError::Database(e.to_string()))?);
        }
        let legacy_columns = columns.contains("channel_id") && columns.contains("model_name");
        drop(stmt);

        for (idx, item) in items.iter().enumerate() {
            let model = item.upstream_model.trim();
            if model.is_empty() {
                return Err(AppError::Business("上游模型名不能为空".into()));
            }
            let exists: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM providers WHERE id = ?1)",
                    [item.provider_id],
                    |row| row.get(0),
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            if !exists {
                return Err(AppError::Business(format!(
                    "供应商 {} 不存在",
                    item.provider_id
                )));
            }
            if legacy_columns {
                // priority 与 weight 同步写入，兼容旧表即使其默认约束不完整。
                conn.execute(
                    "INSERT INTO group_items
                     (group_id, provider_id, upstream_model, sort_order,
                      channel_id, model_name, priority, weight)
                     VALUES (?1, ?2, ?3, ?4, ?2, ?3, ?4, 1)",
                    params![group_id, item.provider_id, model, idx as i64],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            } else {
                conn.execute(
                    "INSERT INTO group_items (group_id, provider_id, upstream_model, sort_order) VALUES (?1, ?2, ?3, ?4)",
                    params![group_id, item.provider_id, model, idx as i64],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            }
        }
        Ok(())
    }

    pub fn list_groups(&self) -> Result<Vec<Group>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare("SELECT id, name, created_at FROM groups ORDER BY id ASC")
                .map_err(|e| AppError::Database(e.to_string()))?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(|e| AppError::Database(e.to_string()))?;
            let mut out = Vec::new();
            for r in rows {
                let (id, name, created_at) = r.map_err(|e| AppError::Database(e.to_string()))?;
                let items = Self::load_items(conn, id)?;
                out.push(Group {
                    id,
                    name,
                    items,
                    created_at,
                });
            }
            Ok(out)
        })
    }

    pub fn get_group_by_name(&self, name: &str) -> Result<Option<Group>, AppError> {
        self.with_conn(|conn| {
            let row = conn
                .query_row(
                    "SELECT id, name, created_at FROM groups WHERE name = ?1",
                    [name],
                    |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    },
                )
                .optional()
                .map_err(|e| AppError::Database(e.to_string()))?;
            let Some((id, name, created_at)) = row else {
                return Ok(None);
            };
            let items = Self::load_items(conn, id)?;
            Ok(Some(Group {
                id,
                name,
                items,
                created_at,
            }))
        })
    }

    pub fn list_group_names(&self) -> Result<Vec<String>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare("SELECT name FROM groups ORDER BY name ASC")
                .map_err(|e| AppError::Database(e.to_string()))?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| AppError::Database(e.to_string()))?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(|e| AppError::Database(e.to_string()))?);
            }
            Ok(out)
        })
    }

    pub fn create_group(&self, payload: CreateGroupPayload) -> Result<Group, AppError> {
        let name = payload.name.trim().to_string();
        if name.is_empty() {
            return Err(AppError::Business("分组名不能为空".into()));
        }
        let created_at = chrono::Utc::now().to_rfc3339();
        self.with_conn(|conn| {
            let tx = conn
                .unchecked_transaction()
                .map_err(|e| AppError::Database(e.to_string()))?;
            tx.execute(
                "INSERT INTO groups (name, created_at) VALUES (?1, ?2)",
                params![name, created_at],
            )
            .map_err(|e| {
                if e.to_string().contains("UNIQUE") {
                    AppError::Business("分组名已存在".into())
                } else {
                    AppError::Database(e.to_string())
                }
            })?;
            let id = tx.last_insert_rowid();
            Self::replace_items(&tx, id, &payload.items)?;
            tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
            Ok(id)
        })
        .and_then(|id| {
            self.get_group_by_name(&name)?
                .filter(|g| g.id == id)
                .ok_or_else(|| AppError::Business("创建分组失败".into()))
        })
    }

    pub fn update_group(&self, payload: UpdateGroupPayload) -> Result<Group, AppError> {
        let name = payload.name.trim().to_string();
        if name.is_empty() {
            return Err(AppError::Business("分组名不能为空".into()));
        }
        self.with_conn(|conn| {
            let tx = conn
                .unchecked_transaction()
                .map_err(|e| AppError::Database(e.to_string()))?;
            let n = tx
                .execute(
                    "UPDATE groups SET name=?1 WHERE id=?2",
                    params![name, payload.id],
                )
                .map_err(|e| {
                    if e.to_string().contains("UNIQUE") {
                        AppError::Business("分组名已存在".into())
                    } else {
                        AppError::Database(e.to_string())
                    }
                })?;
            if n == 0 {
                return Err(AppError::Business("分组不存在".into()));
            }
            Self::replace_items(&tx, payload.id, &payload.items)?;
            tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
            Ok(())
        })?;
        self.get_group_by_name(&name)?
            .ok_or_else(|| AppError::Business("分组不存在".into()))
    }

    pub fn delete_group(&self, id: i64) -> Result<(), AppError> {
        self.with_conn(|conn| {
            let n = conn
                .execute("DELETE FROM groups WHERE id = ?1", [id])
                .map_err(|e| AppError::Database(e.to_string()))?;
            if n == 0 {
                return Err(AppError::Business("分组不存在".into()));
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_db;
    use crate::domain::provider::CreateProviderPayload;
    use tempfile::tempdir;

    #[test]
    fn group_queue_order() {
        let dir = tempdir().unwrap();
        let s = Stores::new(open_db(&dir.path().join("t.db")).unwrap());
        let p1 = s
            .create_provider(CreateProviderPayload {
                name: "a".into(),
                base_url: "https://a.example/v1".into(),
                api_key: "k".into(),
                enabled: true,
            })
            .unwrap();
        let p2 = s
            .create_provider(CreateProviderPayload {
                name: "b".into(),
                base_url: "https://b.example/v1".into(),
                api_key: "k".into(),
                enabled: true,
            })
            .unwrap();
        let g = s
            .create_group(CreateGroupPayload {
                name: "gpt".into(),
                items: vec![
                    GroupItemInput {
                        provider_id: p2.id,
                        upstream_model: "m2".into(),
                    },
                    GroupItemInput {
                        provider_id: p1.id,
                        upstream_model: "m1".into(),
                    },
                ],
            })
            .unwrap();
        assert_eq!(g.items.len(), 2);
        assert_eq!(g.items[0].provider_id, p2.id);
        assert_eq!(g.items[1].upstream_model, "m1");
        assert!(s.get_group_by_name("missing").unwrap().is_none());
    }

    /// 编辑已有分组并加入第二供应商模型时，不得新增 groups 行，仅扩展 items。
    #[test]
    fn update_group_adds_second_provider_item_without_new_group() {
        let dir = tempdir().unwrap();
        let s = Stores::new(open_db(&dir.path().join("t.db")).unwrap());
        let p1 = s
            .create_provider(CreateProviderPayload {
                name: "provider-a".into(),
                base_url: "https://a.example/v1".into(),
                api_key: "k".into(),
                enabled: true,
            })
            .unwrap();
        let p2 = s
            .create_provider(CreateProviderPayload {
                name: "provider-b".into(),
                base_url: "https://b.example/v1".into(),
                api_key: "k".into(),
                enabled: true,
            })
            .unwrap();
        let g = s
            .create_group(CreateGroupPayload {
                name: "routing".into(),
                items: vec![GroupItemInput {
                    provider_id: p1.id,
                    upstream_model: "model-a".into(),
                }],
            })
            .unwrap();
        assert_eq!(s.list_groups().unwrap().len(), 1);
        assert_eq!(g.items.len(), 1);

        let updated = s
            .update_group(UpdateGroupPayload {
                id: g.id,
                name: g.name.clone(),
                items: vec![
                    GroupItemInput {
                        provider_id: p1.id,
                        upstream_model: "model-a".into(),
                    },
                    GroupItemInput {
                        provider_id: p2.id,
                        upstream_model: "model-b".into(),
                    },
                ],
            })
            .unwrap();

        let all = s.list_groups().unwrap();
        assert_eq!(all.len(), 1, "update 不得插入新 groups 行");
        assert_eq!(updated.id, g.id);
        assert_eq!(updated.name, "routing");
        assert_eq!(updated.items.len(), 2);
        assert_eq!(updated.items[0].provider_id, p1.id);
        assert_eq!(updated.items[0].upstream_model, "model-a");
        assert_eq!(updated.items[1].provider_id, p2.id);
        assert_eq!(updated.items[1].upstream_model, "model-b");
    }
}
