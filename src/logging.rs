use tracing::Level;
use tracing_subscriber::{fmt, EnvFilter};

pub fn init(log_level: &str) {
    let level = match log_level.to_lowercase().as_str() {
        "error" => Level::ERROR,
        "warn" => Level::WARN,
        "info" => Level::INFO,
        "debug" => Level::DEBUG,
        "trace" => Level::TRACE,
        _ => Level::INFO,
    };

    let filter = EnvFilter::from_default_env()
        .add_directive(format!("mbell={}", level).parse().unwrap())
        .add_directive("zbus=warn".parse().unwrap())
        .add_directive("rodio=warn".parse().unwrap());

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();
}
