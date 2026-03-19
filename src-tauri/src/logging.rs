use std::path::Path;
use std::sync::OnceLock;

use tracing_appender::rolling;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

static TRACING_INITIALIZED: OnceLock<()> = OnceLock::new();

/// Initialize tracing once for the application process.
pub fn init_tracing(log_dir: &Path) {
    if TRACING_INITIALIZED.get().is_some() {
        return;
    }

    if let Err(err) = std::fs::create_dir_all(log_dir) {
        eprintln!(
            "failed to create log directory {}: {err}",
            log_dir.display()
        );
        return;
    }

    let file_appender = rolling::daily(log_dir, "eyezen.log");
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn,eyezen=info"));

    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().with_target(true))
        .with(
            fmt::layer()
                .with_ansi(false)
                .with_target(true)
                .with_writer(file_appender),
        );

    if subscriber.try_init().is_ok() {
        let _ = TRACING_INITIALIZED.set(());
        tracing::info!("tracing initialized at {}", log_dir.display());
    }
}
