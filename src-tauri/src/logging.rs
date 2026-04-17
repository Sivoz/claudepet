use tracing_appender::rolling;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// 初始化 tracing 日志系统
/// 返回 WorkerGuard，调用方必须持有到进程结束，否则缓冲日志会丢失
pub fn init() -> anyhow::Result<tracing_appender::non_blocking::WorkerGuard> {
    let log_dir = log_dir()?;
    std::fs::create_dir_all(&log_dir)?;

    let appender = rolling::daily(&log_dir, "claudepet.log");
    let (nb, guard) = tracing_appender::non_blocking(appender);

    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,claude_pet_lib=debug")),
        )
        .with(
            fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(true),
        )
        .with(
            fmt::layer()
                .with_writer(nb)
                .with_ansi(false)
                .json(),
        )
        .init();

    Ok(guard)
}

/// 日志目录路径
pub fn log_dir() -> anyhow::Result<std::path::PathBuf> {
    dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("no home dir"))
        .map(|h| h.join(".claude-pet").join("logs"))
}
