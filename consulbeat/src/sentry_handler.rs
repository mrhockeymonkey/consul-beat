use std::str::FromStr;
use sentry::ClientInitGuard;
use sentry::types::{Dsn, ParseDsnError};
use crate::{ConsulLog, LogLevel};

pub fn init_sentry(dsn: &str) -> Result<ClientInitGuard, ParseDsnError> {
    let sentry_dsn = Dsn::from_str(dsn)?;
    let guard = sentry::init(sentry::ClientOptions {
        dsn: Some(sentry_dsn),
        release: sentry::release_name!(),
        ..Default::default()
    });
    Ok(guard)
}

pub fn handle_log(log: ConsulLog) {
    let sentry_level = match log.level {
        LogLevel::Debug => sentry::Level::Debug,
        LogLevel::Info => sentry::Level::Info,
        LogLevel::Warn => sentry::Level::Warning,
        LogLevel::Error => sentry::Level::Error,
    };
    
    // this could be made configurable but unlikely to ever want info of debug
    if matches!(sentry_level, sentry::Level::Warning | sentry::Level::Error | sentry::Level::Fatal) {
        sentry::capture_message(&log.message, sentry_level);
    }
}