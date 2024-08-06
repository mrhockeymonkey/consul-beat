use std::str::FromStr;
use sentry::ClientInitGuard;
use sentry::protocol::Value;
use sentry::types::{Dsn, ParseDsnError};

use crate::log_parsing::{ConsulLog, ConsulLogLevel};

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
    let sentry_level = match log.level() {
        ConsulLogLevel::Debug => sentry::Level::Debug,
        ConsulLogLevel::Info => sentry::Level::Info,
        ConsulLogLevel::Warn => sentry::Level::Warning,
        ConsulLogLevel::Error => sentry::Level::Error,
    };

    // this could be made configurable but unlikely to ever want info or debug
    if matches!(sentry_level, sentry::Level::Warning | sentry::Level::Error | sentry::Level::Fatal) {
        sentry::capture_message(&log.message(), sentry_level);
    }
}

pub fn handle_parse_fail(log: &str) {
    let message = format!("Failed to parse consul log! {}", log);
    sentry::with_scope(
        |scope| {
            scope.set_extra("log_message", Value::String(log.to_string()));
            scope.set_fingerprint(Some(&["parse-error"])); // to group all parse errors together
        },
        || sentry::capture_message(&message, sentry::Level::Warning)
    );
}