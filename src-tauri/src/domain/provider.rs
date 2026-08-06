use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

use super::Stores;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: i64,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub enabled: bool,
    pub created_at: String,
    /// 是否开启后台自动同步（每 24h 拉取模型并持久化到 provider_models）。
    pub auto_sync: bool,
    /// 最后一次成功同步时间（unix 秒，可空；NULL = 从未同步）。
    pub last_sync_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProviderPayload {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub enabled: bool,
    pub auto_sync: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProviderPayload {
    pub id: i64,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub enabled: bool,
    pub auto_sync: bool,
}

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Provider> {
    Ok(Provider {
        id: row.get(0)?,
        name: row.get(1)?,
        base_url: row.get(2)?,
        api_key: row.get(3)?,
        enabled: row.get::<_, i64>(4)? != 0,
        created_at: row.get(5)?,
        auto_sync: row.get::<_, i64>(6)? != 0,
        last_sync_at: row.get(7)?,
    })
}

impl Stores {
    pub fn list_providers(&self) -> Result<Vec<Provider>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, name, base_url, api_key, enabled, created_at, auto_sync, last_sync_at FROM providers ORDER BY id ASC",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            let rows = stmt
                .query_map([], map_row)
                .map_err(|e| AppError::Database(e.to_string()))?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(|e| AppError::Database(e.to_string()))?);
            }
            Ok(out)
        })
    }

    pub fn get_provider(&self, id: i64) -> Result<Option<Provider>, AppError> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, name, base_url, api_key, enabled, created_at, auto_sync, last_sync_at FROM providers WHERE id = ?1",
                [id],
                map_row,
            )
            .optional()
            .map_err(|e| AppError::Database(e.to_string()))
        })
    }

    pub fn create_provider(&self, payload: CreateProviderPayload) -> Result<Provider, AppError> {
        let name = payload.name.trim().to_string();
        let base_url = payload.base_url.trim().trim_end_matches('/').to_string();
        if name.is_empty() {
            return Err(AppError::Business("供应商名称不能为空".into()));
        }
        if base_url.is_empty() {
            return Err(AppError::Business("Base URL 不能为空".into()));
        }
        let created_at = chrono::Utc::now().to_rfc3339();
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO providers (name, base_url, api_key, enabled, created_at, auto_sync) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    name,
                    base_url,
                    payload.api_key,
                    if payload.enabled { 1 } else { 0 },
                    created_at,
                    if payload.auto_sync { 1 } else { 0 },
                ],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            let id = conn.last_insert_rowid();
            Ok(Provider {
                id,
                name,
                base_url,
                api_key: payload.api_key,
                enabled: payload.enabled,
                created_at,
                auto_sync: payload.auto_sync,
                last_sync_at: None,
            })
        })
    }

    pub fn update_provider(&self, payload: UpdateProviderPayload) -> Result<Provider, AppError> {
        let name = payload.name.trim().to_string();
        let base_url = payload.base_url.trim().trim_end_matches('/').to_string();
        if name.is_empty() {
            return Err(AppError::Business("供应商名称不能为空".into()));
        }
        if base_url.is_empty() {
            return Err(AppError::Business("Base URL 不能为空".into()));
        }
        self.with_conn(|conn| {
            let n = conn
                .execute(
                    "UPDATE providers SET name=?1, base_url=?2, api_key=?3, enabled=?4, auto_sync=?5 WHERE id=?6",
                    params![
                        name,
                        base_url,
                        payload.api_key,
                        if payload.enabled { 1 } else { 0 },
                        if payload.auto_sync { 1 } else { 0 },
                        payload.id
                    ],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            if n == 0 {
                return Err(AppError::Business("供应商不存在".into()));
            }
            Ok(())
        })?;
        self.get_provider(payload.id)?
            .ok_or_else(|| AppError::Business("供应商不存在".into()))
    }

    /// 就地切换供应商自动同步开关，返回更新后的完整 Provider。
    pub fn set_provider_auto_sync(&self, id: i64, enabled: bool) -> Result<Provider, AppError> {
        self.with_conn(|conn| {
            let n = conn
                .execute(
                    "UPDATE providers SET auto_sync = ?1 WHERE id = ?2",
                    params![if enabled { 1 } else { 0 }, id],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            if n == 0 {
                return Err(AppError::Business("供应商不存在".into()));
            }
            Ok(())
        })?;
        self.get_provider(id)?
            .ok_or_else(|| AppError::Business("供应商不存在".into()))
    }

    /// 全量替换某供应商的持久化模型列表：事务内 DELETE + 批量 INSERT。
    /// 空白名与重复模型跳过；失败回滚，旧模型列表保留。
    pub fn replace_provider_models(
        &self,
        provider_id: i64,
        models: &[String],
    ) -> Result<(), AppError> {
        self.with_conn(|conn| {
            let tx = conn
                .unchecked_transaction()
                .map_err(|e| AppError::Database(e.to_string()))?;

            tx.execute(
                "DELETE FROM provider_models WHERE provider_id = ?1",
                [provider_id],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

            let mut seen = std::collections::HashSet::new();
            {
                let mut stmt = tx
                    .prepare(
                        "INSERT INTO provider_models (provider_id, model_name, sort_order) VALUES (?1, ?2, ?3)",
                    )
                    .map_err(|e| AppError::Database(e.to_string()))?;
                for (index, model) in models.iter().enumerate() {
                    let model = model.trim();
                    if model.is_empty() || !seen.insert(model.to_string()) {
                        continue;
                    }
                    stmt.execute(params![provider_id, model, index as i64])
                        .map_err(|e| AppError::Database(e.to_string()))?;
                }
            }

            tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
            Ok(())
        })
    }

    /// 读本地持久化的供应商模型列表（按 sort_order 升序）。
    pub fn list_provider_models(&self, provider_id: i64) -> Result<Vec<String>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT model_name FROM provider_models WHERE provider_id = ?1 ORDER BY sort_order ASC",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            let rows = stmt
                .query_map([provider_id], |row| row.get::<_, String>(0))
                .map_err(|e| AppError::Database(e.to_string()))?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(|e| AppError::Database(e.to_string()))?);
            }
            Ok(out)
        })
    }

    /// 记录供应商最近一次成功同步时间（unix 秒）。
    pub fn touch_provider_synced_at(&self, id: i64, unix: i64) -> Result<(), AppError> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE providers SET last_sync_at = ?1 WHERE id = ?2",
                params![unix, id],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(())
        })
    }

    pub fn delete_provider(&self, id: i64) -> Result<(), AppError> {
        self.with_conn(|conn| {
            let tx = conn
                .unchecked_transaction()
                .map_err(|e| AppError::Database(e.to_string()))?;

            // 先解绑相关的分组（字段保留不再写入，防孤儿引用）
            tx.execute(
                "UPDATE groups SET source_provider_id = NULL WHERE source_provider_id = ?1",
                [id],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

            let n = tx
                .execute("DELETE FROM providers WHERE id = ?1", [id])
                .map_err(|e| AppError::Database(e.to_string()))?;

            if n == 0 {
                return Err(AppError::Business("供应商不存在".into()));
            }

            tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_db;
    use tempfile::tempdir;

    fn stores() -> Stores {
        let dir = tempdir().unwrap();
        let db = open_db(&dir.path().join("t.db")).unwrap();
        // leak tempdir for test duration via forget - use open_in_memory style
        std::mem::forget(dir);
        Stores::new(db)
    }

    fn provider_payload(name: &str) -> CreateProviderPayload {
        CreateProviderPayload {
            name: name.into(),
            base_url: "https://api.example.com/v1/".into(),
            api_key: "k".into(),
            enabled: true,
            auto_sync: true,
        }
    }

    #[test]
    fn provider_crud() {
        let s = stores();
        let p = s.create_provider(provider_payload("p1")).unwrap();
        assert_eq!(p.base_url, "https://api.example.com/v1");
        assert!(p.auto_sync);
        assert_eq!(p.last_sync_at, None);
        let list = s.list_providers().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].auto_sync, true);
        assert_eq!(list[0].last_sync_at, None);

        // 就地切换自动同步
        let toggled = s.set_provider_auto_sync(p.id, false).unwrap();
        assert!(!toggled.auto_sync);
        assert_eq!(s.get_provider(p.id).unwrap().unwrap().auto_sync, false);

        // 记录同步时间
        s.touch_provider_synced_at(p.id, 1_700_000_000).unwrap();
        let touched = s.get_provider(p.id).unwrap().unwrap();
        assert_eq!(touched.last_sync_at, Some(1_700_000_000));

        s.update_provider(UpdateProviderPayload {
            id: p.id,
            name: "p2".into(),
            base_url: p.base_url,
            api_key: "k2".into(),
            enabled: false,
            auto_sync: true,
        })
        .unwrap();
        let after = s.get_provider(p.id).unwrap().unwrap();
        assert_eq!(after.name, "p2");
        assert!(!after.enabled);
        assert!(after.auto_sync);
        // update 不覆盖 last_sync_at
        assert_eq!(after.last_sync_at, Some(1_700_000_000));

        s.delete_provider(p.id).unwrap();
        assert!(s.list_providers().unwrap().is_empty());
    }

    #[test]
    fn provider_models_replace_and_list() {
        let s = stores();
        let p = s.create_provider(provider_payload("p")).unwrap();

        s.replace_provider_models(p.id, &["gpt-4o".into(), "gpt-4o-mini".into()])
            .unwrap();
        assert_eq!(
            s.list_provider_models(p.id).unwrap(),
            vec!["gpt-4o", "gpt-4o-mini"]
        );

        // 全量替换：旧模型清空，新列表生效（顺序即 sort_order）
        s.replace_provider_models(p.id, &["claude-3-5-sonnet".into()])
            .unwrap();
        assert_eq!(
            s.list_provider_models(p.id).unwrap(),
            vec!["claude-3-5-sonnet"]
        );

        // 重复模型与空白名跳过
        s.replace_provider_models(p.id, &["a".into(), "a".into(), "  ".into(), "b".into()])
            .unwrap();
        assert_eq!(s.list_provider_models(p.id).unwrap(), vec!["a", "b"]);

        // 未同步过的供应商读空列表
        let p2 = s.create_provider(provider_payload("p2")).unwrap();
        assert!(s.list_provider_models(p2.id).unwrap().is_empty());

        // 删除供应商级联清理
        s.delete_provider(p.id).unwrap();
        assert!(s.list_provider_models(p.id).unwrap().is_empty());
    }
}
