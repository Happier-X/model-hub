use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use model_hub_gateway::{run, GatewayConfig, DEFAULT_CONFIG_PATH};
use tracing_subscriber::EnvFilter;

/// Model Hub Rust 原生网关（model-hub-gateway）。
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
}

#[tokio::main]
async fn main() -> ExitCode {
    init_tracing();

    let cli = Cli::parse();

    match cli.command {
        None => run_serve(cli.config).await,
        Some(Commands::Serve { config }) => run_serve(config).await,
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

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}
