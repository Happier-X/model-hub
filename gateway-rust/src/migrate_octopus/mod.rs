//! octopus v0.9.28 SQLite → gateway-rust schema 尽力导入。
//!
//! 不在 serve 路径自动改写用户数据库；仅由 CLI `migrate-octopus` 显式调用。

mod detect;
mod import;

pub use detect::{detect_source, SourceKind};
pub use import::{migrate_octopus, MigrateOptions, MigrateSummary};
