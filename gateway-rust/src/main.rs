use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use model_hub_gateway::migrate_octopus::{migrate_octopus, MigrateOptions};
use model_hub_gateway::{run, GatewayConfig, DEFAULT_CONFIG_PATH};
use tracing_subscriber::EnvFilter;

/// Model Hub Rust 原生网关实验骨架（不可替代当前发布版 octopus）。
#[derive(Debug, Parser)]
#[command(name = "model-hub-gateway", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// 配置文件路径（相对当前工作目录或绝对路径）。无 subcommand 时用于 serve。
    #[arg(long, default_value = DEFAULT_CONFIG_PATH, global = true)]
    config: PathBuf,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// 启动 HTTP 网关（默认；也可省略本 subcommand，仅用 --config）
    Serve {
        /// 配置文件路径
        #[arg(long, default_value = DEFAULT_CONFIG_PATH)]
        config: PathBuf,
    },
    /// 将 octopus v0.9.28 SQLite 尽力导入 gateway-rust schema
    MigrateOctopus {
        /// octopus 源库路径（只读打开）
        #[arg(long)]
        source: PathBuf,
        /// 目标 gateway-rust 库路径（不存在则创建）
        #[arg(long)]
        dest: PathBuf,
        /// 目标业务表非空时清空后覆盖
        #[arg(long, default_value_t = false)]
        force: bool,
        /// 迁移 relay_logs → request_logs
        #[arg(long, default_value_t = false)]
        with_logs: bool,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    init_tracing();

    let cli = Cli::parse();

    match cli.command {
        None => run_serve(cli.config).await,
        Some(Commands::Serve { config }) => run_serve(config).await,
        Some(Commands::MigrateOctopus {
            source,
            dest,
            force,
            with_logs,
        }) => run_migrate(source, dest, force, with_logs),
    }
}

async fn run_serve(config_path: PathBuf) -> ExitCode {
    tracing::info!(config = %config_path.display(), "启动 model-hub-gateway");

    let config = match GatewayConfig::load_from_path(&config_path) {
        Ok(config) => config,
        Err(err) => {
            tracing::error!(error = %err, "加载配置失败");
            eprintln!("错误: {err}");
            return ExitCode::from(1);
        }
    };

    tracing::info!(
        host = %config.server.host,
        port = config.server.port,
        "配置已加载"
    );

    match run(config).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            tracing::error!(error = %err, "网关运行失败");
            eprintln!("错误: {err}");
            ExitCode::from(1)
        }
    }
}

fn run_migrate(source: PathBuf, dest: PathBuf, force: bool, with_logs: bool) -> ExitCode {
    tracing::info!(
        source = %source.display(),
        dest = %dest.display(),
        force,
        with_logs,
        "开始 octopus → gateway-rust 迁移"
    );

    match migrate_octopus(&source, &dest, &MigrateOptions { force, with_logs }) {
        Ok(summary) => {
            println!("迁移成功: {summary}");
            tracing::info!(%summary, "迁移完成");
            ExitCode::SUCCESS
        }
        Err(err) => {
            tracing::error!(error = %err, "迁移失败");
            eprintln!("错误: {err}");
            ExitCode::from(1)
        }
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}
