//! 分组 SQLite 存储。

use rusqlite::{params, OptionalExtension};

use super::model::{CreateGroupRequest, Group, GroupError, GroupItem, UpdateGroupRequest};
use crate::db::DbConn;

#[derive(Clone)]
pub struct GroupStore {
    db: DbConn,
}

impl GroupStore {
    pub fn new(db: DbConn) -> Self {
        Self { db }
    }

    pub fn list(&self) -> Result<Vec<Group>, GroupError> {
        let conn = self.db.lock().map_err(|_| GroupError::Internal)?;
        let mut stmt = conn
            .prepare("SELECT id, name, mode, match_regex FROM groups ORDER BY id ASC")
            .map_err(|_| GroupError::Internal)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|_| GroupError::Internal)?;

        let mut out = Vec::new();
        for row in rows {
            let (id, name, mode, match_regex) = row.map_err(|_| GroupError::Internal)?;
            let items = load_items(&conn, id)?;
            out.push(Group {
                id,
                name,
                mode,
                match_regex,
                items,
            });
        }
        Ok(out)
    }

    pub fn get(&self, id: i64) -> Result<Group, GroupError> {
        let conn = self.db.lock().map_err(|_| GroupError::Internal)?;
        let row = conn
            .query_row(
                "SELECT id, name, mode, match_regex FROM groups WHERE id = ?1",
                [id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| GroupError::Internal)?
            .ok_or(GroupError::NotFound)?;
        let (id, name, mode, match_regex) = row;
        let items = load_items(&conn, id)?;
        Ok(Group {
            id,
            name,
            mode,
            match_regex,
            items,
        })
    }

    pub fn create(&self, req: CreateGroupRequest) -> Result<Group, GroupError> {
        let name = req.name.trim().to_string();
        if name.is_empty() {
            return Err(GroupError::InvalidName);
        }
        let conn = self.db.lock().map_err(|_| GroupError::Internal)?;
        conn.execute(
            "INSERT INTO groups (name, mode, match_regex) VALUES (?1, ?2, ?3)",
            params![name, req.mode, req.match_regex],
        )
        .map_err(|_| GroupError::Internal)?;
        let id = conn.last_insert_rowid();
        for item in &req.items {
            insert_item(&conn, id, item)?;
        }
        drop(conn);
        self.get(id)
    }

    pub fn update(&self, req: UpdateGroupRequest) -> Result<Group, GroupError> {
        let conn = self.db.lock().map_err(|_| GroupError::Internal)?;
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM groups WHERE id = ?1)",
                [req.id],
                |row| row.get(0),
            )
            .map_err(|_| GroupError::Internal)?;
        if !exists {
            return Err(GroupError::NotFound);
        }

        if let Some(name) = &req.name {
            let name = name.trim();
            if name.is_empty() {
                return Err(GroupError::InvalidName);
            }
            conn.execute(
                "UPDATE groups SET name = ?1 WHERE id = ?2",
                params![name, req.id],
            )
            .map_err(|_| GroupError::Internal)?;
        }
        if let Some(mode) = req.mode {
            conn.execute(
                "UPDATE groups SET mode = ?1 WHERE id = ?2",
                params![mode, req.id],
            )
            .map_err(|_| GroupError::Internal)?;
        }
        if let Some(match_regex) = &req.match_regex {
            conn.execute(
                "UPDATE groups SET match_regex = ?1 WHERE id = ?2",
                params![match_regex, req.id],
            )
            .map_err(|_| GroupError::Internal)?;
        }

        for item_id in &req.items_to_delete {
            let n = conn
                .execute(
                    "DELETE FROM group_items WHERE id = ?1 AND group_id = ?2",
                    params![item_id, req.id],
                )
                .map_err(|_| GroupError::Internal)?;
            if n == 0 {
                return Err(GroupError::InvalidInput(format!(
                    "分组 item {item_id} 不存在或不属于该分组"
                )));
            }
        }
        for item in &req.items_to_add {
            insert_item(&conn, req.id, item)?;
        }
        drop(conn);
        self.get(req.id)
    }

    pub fn delete(&self, id: i64) -> Result<(), GroupError> {
        let conn = self.db.lock().map_err(|_| GroupError::Internal)?;
        let n = conn
            .execute("DELETE FROM groups WHERE id = ?1", [id])
            .map_err(|_| GroupError::Internal)?;
        if n == 0 {
            return Err(GroupError::NotFound);
        }
        Ok(())
    }
}

fn load_items(conn: &rusqlite::Connection, group_id: i64) -> Result<Vec<GroupItem>, GroupError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, group_id, channel_id, model_name, priority, weight
             FROM group_items WHERE group_id = ?1 ORDER BY id ASC",
        )
        .map_err(|_| GroupError::Internal)?;
    let rows = stmt
        .query_map([group_id], |row| {
            Ok(GroupItem {
                id: Some(row.get(0)?),
                group_id: Some(row.get(1)?),
                channel_id: row.get(2)?,
                model_name: row.get(3)?,
                priority: row.get(4)?,
                weight: row.get(5)?,
            })
        })
        .map_err(|_| GroupError::Internal)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|_| GroupError::Internal)?);
    }
    Ok(out)
}

fn insert_item(
    conn: &rusqlite::Connection,
    group_id: i64,
    item: &GroupItem,
) -> Result<(), GroupError> {
    conn.execute(
        "INSERT INTO group_items (group_id, channel_id, model_name, priority, weight)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            group_id,
            item.channel_id,
            item.model_name,
            item.priority,
            item.weight,
        ],
    )
    .map_err(|_| GroupError::Internal)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_tempfile_db;

    #[test]
    fn crud_and_rebind_items() {
        let (db, _dir) = open_tempfile_db();
        let store = GroupStore::new(db);

        let g = store
            .create(CreateGroupRequest {
                name: "smoke-group".into(),
                mode: 1,
                match_regex: "".into(),
                items: vec![GroupItem {
                    id: None,
                    group_id: None,
                    channel_id: 1,
                    model_name: "gpt-4o-mini".into(),
                    priority: 1,
                    weight: 1,
                }],
            })
            .unwrap();
        assert_eq!(g.mode, 1);
        assert_eq!(g.items.len(), 1);
        let item_id = g.items[0].id.unwrap();

        let updated = store
            .update(UpdateGroupRequest {
                id: g.id,
                name: Some("renamed".into()),
                mode: None,
                match_regex: None,
                items_to_delete: vec![item_id],
                items_to_add: vec![GroupItem {
                    id: None,
                    group_id: None,
                    channel_id: 2,
                    model_name: "gpt-4o".into(),
                    priority: 1,
                    weight: 1,
                }],
            })
            .unwrap();
        assert_eq!(updated.name, "renamed");
        assert_eq!(updated.items.len(), 1);
        assert_eq!(updated.items[0].channel_id, 2);
        assert_eq!(updated.items[0].model_name, "gpt-4o");

        store.delete(g.id).unwrap();
        assert!(store.list().unwrap().is_empty());
    }
}
