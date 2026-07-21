//! 从 octopus 源库导入到 gateway-rust 目标库。

use std::path::Path;

use rusqlite::{params, Connection};

use super::detect::{classify_connection, SourceKind};
use crate::apikey::{hash_key, mask_key};
use crate::db::migrate;
use crate::error::GatewayError;
use crate::log::truncate_error;

/// 迁移选项。
#[derive(Debug, Clone)]
pub struct MigrateOptions {
    /// 目标业务表非空时是否清空后覆盖。
    pub force: bool,
    /// 是否迁移 relay_logs → request_logs。
    pub with_logs: bool,
}

/// 导入计数摘要。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MigrateSummary {
    pub channels: u64,
    pub channel_base_urls: u64,
    pub channel_keys: u64,
    pub groups: u64,
    pub group_items: u64,
    pub api_keys: u64,
    pub request_logs: u64,
}

impl std::fmt::Display for MigrateSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "channels={}, channel_base_urls={}, channel_keys={}, groups={}, group_items={}, api_keys={}, request_logs={}",
            self.channels,
            self.channel_base_urls,
            self.channel_keys,
            self.groups,
            self.group_items,
            self.api_keys,
            self.request_logs
        )
    }
}

/// 执行 octopus → rust 尽力导入。
pub fn migrate_octopus(
    source: &Path,
    dest: &Path,
    options: &MigrateOptions,
) -> Result<MigrateSummary, GatewayError> {
    if !source.exists() {
        return Err(GatewayError::database(format!(
            "源库不存在: {}",
            source.display()
        )));
    }

    let source_conn = open_source_readonly(source)?;
    match classify_connection(&source_conn) {
        SourceKind::Octopus => {}
        SourceKind::RustGateway => {
            return Err(GatewayError::database(
                "源库已是 gateway-rust schema（存在 schema_migrations 且无 octopus 特征），拒绝迁移",
            ));
        }
        SourceKind::Unknown => {
            return Err(GatewayError::database(
                "无法识别源库为 octopus（需要 users/migration_records 与 channels.base_urls）",
            ));
        }
    }

    let dest_conn = open_dest(dest)?;
    migrate(&dest_conn)?;

    if dest_has_business_data(&dest_conn)? {
        if !options.force {
            return Err(GatewayError::database(
                "目标库业务表非空；请使用 --force 清空业务表后覆盖，或换用空的 --dest",
            ));
        }
        clear_business_tables(&dest_conn)?;
    }

    let tx = dest_conn
        .unchecked_transaction()
        .map_err(|e| GatewayError::database(format!("开启目标事务失败: {e}")))?;

    let mut summary = MigrateSummary::default();
    import_channels(&source_conn, &tx, &mut summary)?;
    import_groups(&source_conn, &tx, &mut summary)?;
    import_api_keys(&source_conn, &tx, &mut summary)?;
    if options.with_logs {
        import_relay_logs(&source_conn, &tx, &mut summary)?;
    }

    fix_sqlite_sequences(&tx)?;

    tx.commit()
        .map_err(|e| GatewayError::database(format!("提交迁移事务失败: {e}")))?;

    Ok(summary)
}

fn open_source_readonly(path: &Path) -> Result<Connection, GatewayError> {
    Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| GatewayError::database(format!("打开源库失败 ({}): {e}", path.display())))
}

fn open_dest(path: &Path) -> Result<Connection, GatewayError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                GatewayError::database(format!("创建目标库目录失败 ({}): {e}", parent.display()))
            })?;
        }
    }
    let conn = Connection::open(path)
        .map_err(|e| GatewayError::database(format!("打开目标库失败 ({}): {e}", path.display())))?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| GatewayError::database(format!("启用 foreign_keys 失败: {e}")))?;
    Ok(conn)
}

fn dest_has_business_data(conn: &Connection) -> Result<bool, GatewayError> {
    for table in ["channels", "api_keys", "groups", "request_logs"] {
        let n: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .map_err(|e| GatewayError::database(format!("检查目标表 {table} 失败: {e}")))?;
        if n > 0 {
            return Ok(true);
        }
    }
    Ok(false)
}

fn clear_business_tables(conn: &Connection) -> Result<(), GatewayError> {
    // 顺序：先子表后父表
    for table in [
        "group_items",
        "groups",
        "channel_keys",
        "channel_base_urls",
        "channels",
        "api_keys",
        "request_logs",
    ] {
        conn.execute(&format!("DELETE FROM {table}"), [])
            .map_err(|e| GatewayError::database(format!("清空目标表 {table} 失败: {e}")))?;
    }
    Ok(())
}

fn import_channels(
    source: &Connection,
    dest: &Connection,
    summary: &mut MigrateSummary,
) -> Result<(), GatewayError> {
    if !table_exists(source, "channels") {
        return Ok(());
    }

    let mut stmt = source
        .prepare(
            "SELECT id, name, type, enabled, base_urls, model, custom_model, proxy, auto_sync, auto_group, custom_header
             FROM channels ORDER BY id ASC",
        )
        .map_err(|e| GatewayError::database(format!("读取源 channels 失败: {e}")))?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<i64>>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<i64>>(7)?,
                row.get::<_, Option<i64>>(8)?,
                row.get::<_, Option<i64>>(9)?,
                row.get::<_, Option<String>>(10)?,
            ))
        })
        .map_err(|e| GatewayError::database(format!("遍历源 channels 失败: {e}")))?;

    for row in rows {
        let (
            id,
            name,
            ty,
            enabled,
            base_urls_json,
            model,
            custom_model,
            proxy,
            auto_sync,
            auto_group,
            custom_header,
        ) = row.map_err(|e| GatewayError::database(format!("解析 channel 行失败: {e}")))?;

        let header_json = custom_header
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "[]".into());

        dest.execute(
            "INSERT INTO channels (
                id, name, type, enabled, model, custom_model, proxy, auto_sync, auto_group, custom_header_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id,
                name,
                ty.unwrap_or(0),
                enabled.unwrap_or(1),
                model.unwrap_or_default(),
                custom_model.unwrap_or_default(),
                proxy.unwrap_or(0),
                auto_sync.unwrap_or(0),
                auto_group.unwrap_or(0),
                header_json,
            ],
        )
        .map_err(|e| GatewayError::database(format!("写入 channel id={id} 失败: {e}")))?;
        summary.channels += 1;

        let urls = parse_base_urls(base_urls_json.as_deref().unwrap_or("[]"));
        for (i, (url, delay)) in urls.into_iter().enumerate() {
            dest.execute(
                "INSERT INTO channel_base_urls (channel_id, url, delay, sort_order)
                 VALUES (?1, ?2, ?3, ?4)",
                params![id, url, delay, i as i64],
            )
            .map_err(|e| {
                GatewayError::database(format!("写入 channel_base_urls channel_id={id} 失败: {e}"))
            })?;
            summary.channel_base_urls += 1;
        }

        import_channel_keys_for(source, dest, id, summary)?;
    }

    Ok(())
}

fn import_channel_keys_for(
    source: &Connection,
    dest: &Connection,
    channel_id: i64,
    summary: &mut MigrateSummary,
) -> Result<(), GatewayError> {
    if !table_exists(source, "channel_keys") {
        return Ok(());
    }

    let mut stmt = source
        .prepare(
            "SELECT id, channel_id, enabled, channel_key, remark
             FROM channel_keys WHERE channel_id = ?1 ORDER BY id ASC",
        )
        .map_err(|e| GatewayError::database(format!("读取源 channel_keys 失败: {e}")))?;

    let rows = stmt
        .query_map([channel_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Option<i64>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(|e| GatewayError::database(format!("遍历源 channel_keys 失败: {e}")))?;

    for row in rows {
        let (id, ch_id, enabled, key, remark) =
            row.map_err(|e| GatewayError::database(format!("解析 channel_key 行失败: {e}")))?;
        dest.execute(
            "INSERT INTO channel_keys (id, channel_id, enabled, channel_key, remark)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                id,
                ch_id,
                enabled.unwrap_or(1),
                key.unwrap_or_default(),
                remark.unwrap_or_default(),
            ],
        )
        .map_err(|e| GatewayError::database(format!("写入 channel_key id={id} 失败: {e}")))?;
        summary.channel_keys += 1;
    }
    Ok(())
}

fn import_groups(
    source: &Connection,
    dest: &Connection,
    summary: &mut MigrateSummary,
) -> Result<(), GatewayError> {
    if !table_exists(source, "groups") {
        return Ok(());
    }

    let mut stmt = source
        .prepare("SELECT id, name, mode, match_regex FROM groups ORDER BY id ASC")
        .map_err(|e| GatewayError::database(format!("读取源 groups 失败: {e}")))?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<i64>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(|e| GatewayError::database(format!("遍历源 groups 失败: {e}")))?;

    for row in rows {
        let (id, name, mode, match_regex) =
            row.map_err(|e| GatewayError::database(format!("解析 group 行失败: {e}")))?;
        dest.execute(
            "INSERT INTO groups (id, name, mode, match_regex) VALUES (?1, ?2, ?3, ?4)",
            params![id, name, mode.unwrap_or(1), match_regex.unwrap_or_default(),],
        )
        .map_err(|e| GatewayError::database(format!("写入 group id={id} 失败: {e}")))?;
        summary.groups += 1;
    }

    if !table_exists(source, "group_items") {
        return Ok(());
    }

    let mut stmt = source
        .prepare(
            "SELECT id, group_id, channel_id, model_name, priority, weight
             FROM group_items ORDER BY id ASC",
        )
        .map_err(|e| GatewayError::database(format!("读取源 group_items 失败: {e}")))?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, Option<i64>>(5)?,
            ))
        })
        .map_err(|e| GatewayError::database(format!("遍历源 group_items 失败: {e}")))?;

    for row in rows {
        let (id, group_id, channel_id, model_name, priority, weight) =
            row.map_err(|e| GatewayError::database(format!("解析 group_item 行失败: {e}")))?;
        dest.execute(
            "INSERT INTO group_items (id, group_id, channel_id, model_name, priority, weight)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                group_id,
                channel_id,
                model_name,
                priority.unwrap_or(1),
                weight.unwrap_or(1),
            ],
        )
        .map_err(|e| GatewayError::database(format!("写入 group_item id={id} 失败: {e}")))?;
        summary.group_items += 1;
    }

    Ok(())
}

fn import_api_keys(
    source: &Connection,
    dest: &Connection,
    summary: &mut MigrateSummary,
) -> Result<(), GatewayError> {
    if !table_exists(source, "api_keys") {
        return Ok(());
    }

    let mut stmt = source
        .prepare(
            "SELECT id, name, api_key, enabled, expire_at, max_cost, supported_models
             FROM api_keys ORDER BY id ASC",
        )
        .map_err(|e| GatewayError::database(format!("读取源 api_keys 失败: {e}")))?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<i64>>(3)?,
                // expire_at 在 octopus 中为 integer（0 表示无过期）
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, Option<f64>>(5)?,
                row.get::<_, Option<String>>(6)?,
            ))
        })
        .map_err(|e| GatewayError::database(format!("遍历源 api_keys 失败: {e}")))?;

    for row in rows {
        let (id, name, raw_key, enabled, expire_at, max_cost, supported_models) =
            row.map_err(|e| GatewayError::database(format!("解析 api_key 行失败: {e}")))?;

        let key_hash = hash_key(&raw_key);
        let masked = mask_key(&raw_key);
        let models_json = normalize_supported_models(supported_models.as_deref());
        let expire_text = match expire_at {
            Some(0) | None => None,
            Some(ts) => Some(ts.to_string()),
        };
        let max_cost = max_cost.filter(|c| *c != 0.0);

        dest.execute(
            "INSERT INTO api_keys (
                id, name, api_key_masked, key_hash, enabled, expire_at, max_cost, supported_models_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                name,
                masked,
                key_hash,
                enabled.unwrap_or(1),
                expire_text,
                max_cost,
                models_json,
            ],
        )
        .map_err(|e| GatewayError::database(format!("写入 api_key id={id} 失败: {e}")))?;

        // 防御：目标库绝不能出现完整明文
        debug_assert_ne!(masked, raw_key);
        debug_assert!(!key_hash.starts_with("sk-octopus-") || key_hash.len() == 64);

        summary.api_keys += 1;
    }

    Ok(())
}

fn import_relay_logs(
    source: &Connection,
    dest: &Connection,
    summary: &mut MigrateSummary,
) -> Result<(), GatewayError> {
    if !table_exists(source, "relay_logs") {
        return Ok(());
    }

    let mut stmt = source
        .prepare(
            "SELECT id, time, request_model_name, channel_name, actual_model_name,
                    input_tokens, output_tokens, use_time, cost, error
             FROM relay_logs ORDER BY id ASC",
        )
        .map_err(|e| GatewayError::database(format!("读取源 relay_logs 失败: {e}")))?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<i64>>(5)?,
                row.get::<_, Option<i64>>(6)?,
                row.get::<_, Option<i64>>(7)?,
                row.get::<_, Option<f64>>(8)?,
                row.get::<_, Option<String>>(9)?,
            ))
        })
        .map_err(|e| GatewayError::database(format!("遍历源 relay_logs 失败: {e}")))?;

    for row in rows {
        let (
            id,
            time,
            request_model_name,
            channel_name,
            actual_model_name,
            input_tokens,
            output_tokens,
            use_time,
            cost,
            error,
        ) = row.map_err(|e| GatewayError::database(format!("解析 relay_log 行失败: {e}")))?;

        let error = truncate_error(error.as_deref().unwrap_or(""));

        if let Some(id) = id {
            dest.execute(
                "INSERT INTO request_logs (
                    id, time, request_model_name, channel_name, actual_model_name,
                    input_tokens, output_tokens, use_time, cost, error
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    id,
                    time.unwrap_or(0),
                    request_model_name.unwrap_or_default(),
                    channel_name.unwrap_or_default(),
                    actual_model_name.unwrap_or_default(),
                    input_tokens.unwrap_or(0),
                    output_tokens.unwrap_or(0),
                    use_time.unwrap_or(0),
                    cost.unwrap_or(0.0),
                    error,
                ],
            )
            .map_err(|e| GatewayError::database(format!("写入 request_log id={id} 失败: {e}")))?;
        } else {
            dest.execute(
                "INSERT INTO request_logs (
                    time, request_model_name, channel_name, actual_model_name,
                    input_tokens, output_tokens, use_time, cost, error
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    time.unwrap_or(0),
                    request_model_name.unwrap_or_default(),
                    channel_name.unwrap_or_default(),
                    actual_model_name.unwrap_or_default(),
                    input_tokens.unwrap_or(0),
                    output_tokens.unwrap_or(0),
                    use_time.unwrap_or(0),
                    cost.unwrap_or(0.0),
                    error,
                ],
            )
            .map_err(|e| GatewayError::database(format!("写入 request_log 失败: {e}")))?;
        }
        summary.request_logs += 1;
    }

    Ok(())
}

fn parse_base_urls(json: &str) -> Vec<(String, i64)> {
    let trimmed = json.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
        return Vec::new();
    };
    let Some(arr) = value.as_array() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in arr {
        let url = item
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if url.is_empty() {
            continue;
        }
        let delay = item
            .get("delay")
            .and_then(|v| v.as_i64())
            .or_else(|| item.get("delay").and_then(|v| v.as_f64()).map(|f| f as i64))
            .unwrap_or(0);
        out.push((url, delay));
    }
    out
}

/// 空字符串 → None；JSON 数组 → 序列化字符串；其它非空原样尝试解析。
fn normalize_supported_models(raw: Option<&str>) -> Option<String> {
    let s = raw?.trim();
    if s.is_empty() {
        return None;
    }
    if let Ok(arr) = serde_json::from_str::<Vec<String>>(s) {
        return serde_json::to_string(&arr).ok();
    }
    // 非 JSON 时不硬塞，记为 None
    None
}

fn table_exists(conn: &Connection, name: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
        [name],
        |row| row.get::<_, i64>(0),
    )
    .map(|n| n > 0)
    .unwrap_or(false)
}

fn fix_sqlite_sequences(conn: &Connection) -> Result<(), GatewayError> {
    // sqlite_sequence 仅在表用过 AUTOINCREMENT 后存在
    let has_seq: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sqlite_sequence'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false);
    if !has_seq {
        return Ok(());
    }

    for table in [
        "channels",
        "channel_base_urls",
        "channel_keys",
        "groups",
        "group_items",
        "api_keys",
        "request_logs",
    ] {
        let max_id: Option<i64> = conn
            .query_row(&format!("SELECT MAX(id) FROM {table}"), [], |row| {
                row.get(0)
            })
            .map_err(|e| GatewayError::database(format!("查询 {table} MAX(id) 失败: {e}")))?;
        if let Some(max_id) = max_id {
            let exists: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_sequence WHERE name = ?1)",
                    [table],
                    |row| row.get(0),
                )
                .unwrap_or(false);
            if exists {
                conn.execute(
                    "UPDATE sqlite_sequence SET seq = ?1 WHERE name = ?2",
                    params![max_id, table],
                )
                .map_err(|e| {
                    GatewayError::database(format!("更新 sqlite_sequence({table}) 失败: {e}"))
                })?;
            } else {
                let _ = conn.execute(
                    "INSERT INTO sqlite_sequence(name, seq) VALUES (?1, ?2)",
                    params![table, max_id],
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apikey::{hash_key, ApiKeyStore, SqliteApiKeyStore};
    use crate::channel::ChannelStore;
    use crate::db::{open_path, DbConn};
    use crate::group::GroupStore;

    fn build_mini_octopus(path: &Path) {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE users (id INTEGER PRIMARY KEY, username TEXT, password TEXT NOT NULL);
            CREATE TABLE migration_records (version INTEGER PRIMARY KEY, status INTEGER);
            CREATE TABLE channels (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              name TEXT NOT NULL,
              type INTEGER,
              enabled INTEGER DEFAULT 1,
              base_urls TEXT,
              model TEXT,
              custom_model TEXT,
              proxy INTEGER DEFAULT 0,
              auto_sync INTEGER DEFAULT 0,
              auto_group INTEGER DEFAULT 0,
              custom_header TEXT
            );
            CREATE TABLE channel_keys (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              channel_id INTEGER,
              enabled INTEGER DEFAULT 1,
              channel_key TEXT,
              remark TEXT
            );
            CREATE TABLE groups (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              name TEXT NOT NULL,
              mode INTEGER NOT NULL,
              match_regex TEXT
            );
            CREATE TABLE group_items (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              group_id INTEGER NOT NULL,
              channel_id INTEGER NOT NULL,
              model_name TEXT NOT NULL,
              priority INTEGER,
              weight INTEGER
            );
            CREATE TABLE api_keys (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              name TEXT NOT NULL,
              api_key TEXT NOT NULL,
              enabled INTEGER DEFAULT 1,
              expire_at INTEGER,
              max_cost REAL,
              supported_models TEXT
            );
            CREATE TABLE relay_logs (
              id INTEGER PRIMARY KEY,
              time INTEGER,
              request_model_name TEXT,
              channel_name TEXT,
              actual_model_name TEXT,
              input_tokens INTEGER,
              output_tokens INTEGER,
              use_time INTEGER,
              cost REAL,
              error TEXT
            );
            CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE stats_totals (id INTEGER PRIMARY KEY, input_token INTEGER);

            INSERT INTO users (id, username, password) VALUES (1, 'admin', 'hashed');
            INSERT INTO settings (key, value) VALUES ('theme', 'dark');
            INSERT INTO stats_totals (id, input_token) VALUES (1, 99);

            INSERT INTO channels (id, name, type, enabled, base_urls, model, custom_model, proxy, auto_sync, auto_group, custom_header)
            VALUES (1, 'smoke-openai', 0, 1, '[{"url":"https://api.openai.com/v1","delay":0}]',
                    'gpt-4o-mini', '', 0, 0, 0, '[]');
            INSERT INTO channel_keys (id, channel_id, enabled, channel_key, remark)
            VALUES (1, 1, 1, 'sk-test-fake', 'smoke');
            INSERT INTO groups (id, name, mode, match_regex) VALUES (1, 'smoke-group', 1, '');
            INSERT INTO group_items (id, group_id, channel_id, model_name, priority, weight)
            VALUES (1, 1, 1, 'gpt-4o-mini', 1, 1);
            INSERT INTO api_keys (id, name, api_key, enabled, expire_at, max_cost, supported_models)
            VALUES (1, 'smoke-client', 'sk-octopus-kUW6SFvzZX58ObMO126jGGogusXE3am5e7pcWYkY9hUiOvGe',
                    1, 0, 0.0, '');
            INSERT INTO relay_logs (id, time, request_model_name, channel_name, actual_model_name,
                                    input_tokens, output_tokens, use_time, cost, error)
            VALUES (1, 1700000000, 'smoke-group', 'smoke-openai', 'gpt-4o-mini', 3, 5, 1, 0.0, '');
            "#,
        )
        .unwrap();
    }

    #[test]
    fn migrate_mini_octopus_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("octopus.db");
        let dest = dir.path().join("rust.db");
        build_mini_octopus(&source);

        let raw_key = "sk-octopus-kUW6SFvzZX58ObMO126jGGogusXE3am5e7pcWYkY9hUiOvGe";
        let summary = migrate_octopus(
            &source,
            &dest,
            &MigrateOptions {
                force: false,
                with_logs: true,
            },
        )
        .unwrap();

        assert_eq!(summary.channels, 1);
        assert_eq!(summary.channel_base_urls, 1);
        assert_eq!(summary.channel_keys, 1);
        assert_eq!(summary.groups, 1);
        assert_eq!(summary.group_items, 1);
        assert_eq!(summary.api_keys, 1);
        assert_eq!(summary.request_logs, 1);

        let db: DbConn = open_path(dest.to_str().unwrap()).unwrap();
        let channels = ChannelStore::new(db.clone()).list().unwrap();
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].name, "smoke-openai");
        assert_eq!(channels[0].base_urls[0].url, "https://api.openai.com/v1");
        assert_eq!(channels[0].keys[0].channel_key, "sk-test-fake");

        let groups = GroupStore::new(db.clone()).list().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "smoke-group");
        assert_eq!(groups[0].items[0].channel_id, 1);

        let store = SqliteApiKeyStore::new(db.clone());
        let found = store.find_by_raw_key(raw_key).expect("key should verify");
        assert_eq!(found.id, 1);
        assert_eq!(found.key_hash, hash_key(raw_key));

        // 目标库无明文列/无明文值
        {
            let conn = db.lock().unwrap();
            let cols: Vec<String> = {
                let mut stmt = conn.prepare("PRAGMA table_info(api_keys)").unwrap();
                stmt.query_map([], |row| row.get::<_, String>(1))
                    .unwrap()
                    .filter_map(Result::ok)
                    .collect()
            };
            assert!(!cols.iter().any(|c| c == "api_key"));
            let masked: String = conn
                .query_row(
                    "SELECT api_key_masked FROM api_keys WHERE id = 1",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_ne!(masked, raw_key);
            assert!(masked.contains("****"));
            let dump: String = conn
                .query_row(
                    "SELECT group_concat(api_key_masked || key_hash) FROM api_keys",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(!dump.contains(raw_key));
        }

        // users / settings / stats 未迁移
        {
            let conn = db.lock().unwrap();
            let users: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='users'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(users, 0);
        }
    }

    #[test]
    fn refuse_nonempty_dest_without_force() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("octopus.db");
        let dest = dir.path().join("rust.db");
        build_mini_octopus(&source);
        migrate_octopus(
            &source,
            &dest,
            &MigrateOptions {
                force: false,
                with_logs: false,
            },
        )
        .unwrap();

        let err = migrate_octopus(
            &source,
            &dest,
            &MigrateOptions {
                force: false,
                with_logs: false,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("--force"));
    }

    #[test]
    fn force_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("octopus.db");
        let dest = dir.path().join("rust.db");
        build_mini_octopus(&source);
        migrate_octopus(
            &source,
            &dest,
            &MigrateOptions {
                force: false,
                with_logs: false,
            },
        )
        .unwrap();
        let summary = migrate_octopus(
            &source,
            &dest,
            &MigrateOptions {
                force: true,
                with_logs: false,
            },
        )
        .unwrap();
        assert_eq!(summary.channels, 1);
        assert_eq!(summary.request_logs, 0);
    }

    #[test]
    fn reject_rust_source() {
        let dir = tempfile::tempdir().unwrap();
        let rust_db = dir.path().join("already-rust.db");
        let dest = dir.path().join("out.db");
        {
            let _ = open_path(rust_db.to_str().unwrap()).unwrap();
        }
        let err = migrate_octopus(
            &rust_db,
            &dest,
            &MigrateOptions {
                force: false,
                with_logs: false,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("gateway-rust"));
    }

    #[test]
    fn parse_base_urls_ok() {
        let urls = parse_base_urls(r#"[{"url":"https://a","delay":2},{"url":"https://b"}]"#);
        assert_eq!(urls.len(), 2);
        assert_eq!(urls[0], ("https://a".into(), 2));
        assert_eq!(urls[1], ("https://b".into(), 0));
    }
}
