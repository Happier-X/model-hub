# 执行计划：数据迁移

## 清单

1. [x] clap subcommands：serve / migrate-octopus（保持 `--config` 默认可 serve）
2. [x] migrate_octopus 模块：检测、导入、force、with-logs
3. [x] api_keys 明文→哈希；channels base_urls JSON 拆表
4. [x] 单测：临时源/目标库
5. [x] README 文档
6. [x] fmt/check/test/clippy（未 commit，由主会话归档）

## 验证

```powershell
cargo test --manifest-path gateway-rust/Cargo.toml
cargo clippy --manifest-path gateway-rust/Cargo.toml --all-targets -- -D warnings
```

可选：

```powershell
cargo run --manifest-path gateway-rust/Cargo.toml -- migrate-octopus --source tools/octopus/testdata-smoke/data/data.db --dest $env:TEMP\rust-migrated.db --force --with-logs
```

## 审查门

- 不默认自动改写用户 db
- 客户端 key 可校验且不落明文
- 限制写清
