use sentry::ClientInitGuard;
use crate::{ConsulLog, LogLevel};

pub fn init_sentry() -> ClientInitGuard {
    sentry::init(("https://6dac55d0f55700fcf00ea6fd04920923@o4507607687102464.ingest.de.sentry.io/4507607691886672", sentry::ClientOptions {
        release: sentry::release_name!(),
        ..Default::default()
    }))
}

pub fn handle_log(log: ConsulLog) {
    let sentry_level = match log.level {
        LogLevel::Debug => sentry::Level::Debug,
        LogLevel::Info => sentry::Level::Info,
        LogLevel::Warn => sentry::Level::Warning,
        LogLevel::Error => sentry::Level::Error,
    };
    sentry::capture_message(&log.message, sentry_level);
}