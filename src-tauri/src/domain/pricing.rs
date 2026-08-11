use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

use super::Stores;

/// 模型单价（每百万 token 美元）。OpenRouter 同步而来，未覆盖模型无行（视为 0 价）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelPrice {
    pub model_name: String,
    pub prompt_price_per_mtok: f64,
    pub completion_price_per_mtok: f64,
}

/// 设置页展示用：全部单价 + 同步信息。
#[derive(Debug, Clone, Serialize)]
pub struct PricingInfo {
    pub items: Vec<ModelPrice>,
    pub count: i64,
    /// 最后同步 unix 秒；NULL = 从未同步。
    pub updated_at: Option<i64>,
}

/// 立即同步结果。
#[derive(Debug, Clone, Serialize)]
pub struct PricingSyncInfo {
    pub count: i64,
    pub updated_at: i64,
}

/// 解析 OpenRouter `/api/v1/models` 响应（`data[].{id, pricing.{prompt,completion}}`，
/// pricing 为每 token 美元）。非法行跳过；每 token → 每百万（×1e6）并取整到 6 位小数。
pub fn parse_openrouter_pricing(body: &[u8]) -> Vec<ModelPrice> {
    let Ok(json) = serde_json::from_slice::<serde_json::Value>(body) else {
        return Vec::new();
    };
    let Some(data) = json.get("data").and_then(|v| v.as_array()) else {
        return Vec::new();
    };

    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(id) = item.get("id").and_then(|v| v.as_str()).map(str::trim) else {
            continue;
        };
        if id.is_empty() {
            continue;
        }
        let prompt_per_token = item
            .get("pricing")
            .and_then(|p| p.get("prompt"))
            .and_then(parse_price_value)
            .unwrap_or(0.0);
        let completion_per_token = item
            .get("pricing")
            .and_then(|p| p.get("completion"))
            .and_then(parse_price_value)
            .unwrap_or(0.0);
        out.push(ModelPrice {
            model_name: id.to_string(),
            prompt_price_per_mtok: round6(prompt_per_token * 1_000_000.0),
            completion_price_per_mtok: round6(completion_per_token * 1_000_000.0),
        });
    }
    out
}

fn parse_price_value(value: &serde_json::Value) -> Option<f64> {
    let price = value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.trim().parse::<f64>().ok()))?;
    price.is_finite().then_some(price)
}

fn round6(v: f64) -> f64 {
    (v * 1_000_000.0).round() / 1_000_000.0
}

impl Stores {
    /// 全量替换单价表：upsert 新价格，删除表中不再出现的模型。失败不影响原表内容。
    pub fn replace_pricing(&self, prices: &[ModelPrice]) -> Result<(), AppError> {
        let now = chrono::Utc::now().timestamp();
        self.with_conn(|conn| {
            let tx = conn
                .unchecked_transaction()
                .map_err(|e| AppError::Database(e.to_string()))?;
            for p in prices {
                tx.execute(
                    "INSERT INTO model_pricing (model_name, prompt_price_per_mtok, completion_price_per_mtok, updated_at)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(model_name) DO UPDATE SET
                       prompt_price_per_mtok = excluded.prompt_price_per_mtok,
                       completion_price_per_mtok = excluded.completion_price_per_mtok,
                       updated_at = excluded.updated_at",
                    params![
                        p.model_name,
                        p.prompt_price_per_mtok,
                        p.completion_price_per_mtok,
                        now
                    ],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            }
            if prices.is_empty() {
                tx.execute("DELETE FROM model_pricing", [])
                    .map_err(|e| AppError::Database(e.to_string()))?;
            } else {
                let placeholders: Vec<String> =
                    (0..prices.len()).map(|i| format!("?{}", i + 1)).collect();
                let sql = format!(
                    "DELETE FROM model_pricing WHERE model_name NOT IN ({})",
                    placeholders.join(",")
                );
                let names: Vec<&str> = prices.iter().map(|p| p.model_name.as_str()).collect();
                tx.execute(&sql, rusqlite::params_from_iter(names))
                    .map_err(|e| AppError::Database(e.to_string()))?;
            }
            tx.commit()
                .map_err(|e| AppError::Database(e.to_string()))
        })
    }

    pub fn list_pricing(&self) -> Result<Vec<ModelPrice>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT model_name, prompt_price_per_mtok, completion_price_per_mtok
                     FROM model_pricing ORDER BY model_name ASC",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            let rows = stmt
                .query_map([], |row| {
                    Ok(ModelPrice {
                        model_name: row.get(0)?,
                        prompt_price_per_mtok: row.get(1)?,
                        completion_price_per_mtok: row.get(2)?,
                    })
                })
                .map_err(|e| AppError::Database(e.to_string()))?;
            let mut items = Vec::new();
            for row in rows {
                items.push(row.map_err(|e| AppError::Database(e.to_string()))?);
            }
            Ok(items)
        })
    }

    pub fn pricing_info(&self) -> Result<PricingInfo, AppError> {
        let items = self.list_pricing()?;
        let count = items.len() as i64;
        let updated_at = self.with_conn(|conn| {
            conn.query_row(
                "SELECT MAX(updated_at) FROM model_pricing",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))
        })?;
        Ok(PricingInfo {
            items,
            count,
            updated_at,
        })
    }

    /// 最近一次同步时间（unix）；从未同步返回 None。
    pub fn last_pricing_sync_at(&self) -> Result<Option<i64>, AppError> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT MAX(updated_at) FROM model_pricing",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_openrouter_pricing_supports_string_prices() {
        let body = br#"{"data":[
            {"id":"deepseek/deepseek-chat","pricing":{"prompt":"0.00000125","completion":" 0.00000425 "}}
        ]}"#;
        let prices = parse_openrouter_pricing(body);
        assert_eq!(prices.len(), 1);
        assert_eq!(prices[0].prompt_price_per_mtok, 1.25);
        assert_eq!(prices[0].completion_price_per_mtok, 4.25);
    }

    #[test]
    fn parse_openrouter_pricing_invalid_string_falls_back_individually() {
        let body = br#"{"data":[
            {"id":"invalid-prompt","pricing":{"prompt":"not-a-number","completion":"0.00000425"}},
            {"id":"empty-prompt","pricing":{"prompt":"  ","completion":""}},
            {"id":"valid-completion","pricing":{"prompt":"0.00000125","completion":"invalid"}}
        ]}"#;
        let prices = parse_openrouter_pricing(body);
        assert_eq!(prices.len(), 3);
        assert_eq!(prices[0].prompt_price_per_mtok, 0.0);
        assert_eq!(prices[0].completion_price_per_mtok, 4.25);
        assert_eq!(prices[1].prompt_price_per_mtok, 0.0);
        assert_eq!(prices[1].completion_price_per_mtok, 0.0);
        assert_eq!(prices[2].prompt_price_per_mtok, 1.25);
        assert_eq!(prices[2].completion_price_per_mtok, 0.0);
    }

    #[test]
    fn parse_openrouter_pricing_converts_per_token_to_per_mtok() {
        let body = br#"{"data":[
            {"id":"deepseek/deepseek-chat","pricing":{"prompt":0.00000125,"completion":0.00000425}},
            {"id":"openai/gpt-4o","pricing":{"prompt":0.0000025,"completion":0.00001}}
        ]}"#;
        let prices = parse_openrouter_pricing(body);
        assert_eq!(prices.len(), 2);
        assert_eq!(prices[0].model_name, "deepseek/deepseek-chat");
        assert_eq!(prices[0].prompt_price_per_mtok, 1.25);
        assert_eq!(prices[0].completion_price_per_mtok, 4.25);
        assert_eq!(prices[1].prompt_price_per_mtok, 2.5);
        assert_eq!(prices[1].completion_price_per_mtok, 10.0);
    }

    #[test]
    fn parse_openrouter_pricing_free_and_missing_fields() {
        let body = br#"{"data":[
            {"id":"meta/llama:free","pricing":{"prompt":0,"completion":0}},
            {"id":"no-pricing-here"},
            {"id":""},
            {"pricing":{"prompt":1,"completion":1}}
        ]}"#;
        let prices = parse_openrouter_pricing(body);
        // 只保留第 1 行（free），其余：缺 pricing 补 0 / 空 id 跳过 / 缺 id 跳过。
        assert_eq!(prices.len(), 2);
        assert_eq!(prices[0].model_name, "meta/llama:free");
        assert_eq!(prices[0].prompt_price_per_mtok, 0.0);
        assert_eq!(prices[1].model_name, "no-pricing-here");
        assert_eq!(prices[1].prompt_price_per_mtok, 0.0);
    }

    #[test]
    fn parse_openrouter_pricing_invalid_body_returns_empty() {
        assert!(parse_openrouter_pricing(b"not-json").is_empty());
        assert!(parse_openrouter_pricing(b"{}").is_empty());
        assert!(parse_openrouter_pricing(b"").is_empty());
    }

    #[test]
    fn replace_pricing_upserts_and_prunes() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::open_db(&dir.path().join("t.db")).unwrap();
        let stores = Stores::new(db);

        stores
            .replace_pricing(&[
                ModelPrice { model_name: "a/x".into(), prompt_price_per_mtok: 1.0, completion_price_per_mtok: 2.0 },
                ModelPrice { model_name: "b/y".into(), prompt_price_per_mtok: 3.0, completion_price_per_mtok: 4.0 },
            ])
            .unwrap();
        assert_eq!(stores.list_pricing().unwrap().len(), 2);

        // 全量替换：b/y 更新，a/x 被清除，新增 c/z。
        stores
            .replace_pricing(&[
                ModelPrice { model_name: "b/y".into(), prompt_price_per_mtok: 30.0, completion_price_per_mtok: 40.0 },
                ModelPrice { model_name: "c/z".into(), prompt_price_per_mtok: 5.0, completion_price_per_mtok: 6.0 },
            ])
            .unwrap();
        let items = stores.list_pricing().unwrap();
        assert_eq!(items.len(), 2);
        let b = items.iter().find(|p| p.model_name == "b/y").unwrap();
        assert_eq!(b.prompt_price_per_mtok, 30.0);
        assert!(items.iter().all(|p| p.model_name != "a/x"));
        assert!(stores.last_pricing_sync_at().unwrap().is_some());
    }
}
