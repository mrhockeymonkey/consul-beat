use std::{env, io};
use std::fmt::{Display, Formatter};

use WatcherEvent::{NewLogEntry, NoActivity};

use crate::log_parsing::parse_line;
use crate::log_watcher::{LogDirWatcher, WatcherEvent};
use crate::log_watcher::WatcherEvent::NoFileFound;
use crate::sentry_handler::{init_sentry, handle_log, handle_parse_fail};

mod sentry_handler;
mod log_watcher;
mod log_parsing;

const SENTRY_DSN: &str = "SENTRY_DSN";
const SENTRY_ENVIRONMENT: &str = "SENTRY_ENVIRONMENT";
const CONSUL_LOG_DIR: &str = "CONSUL_LOG_DIR";

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    
    let sentry_dsn = env::var(SENTRY_DSN)
        .map_err(|_| AppError::MissingEnvVar(SENTRY_DSN.to_string()))?;
    
    let sentry_env = env::var(SENTRY_ENVIRONMENT).unwrap_or("development".to_string());

    let log_dir = env::var(CONSUL_LOG_DIR).unwrap_or("/var/log".to_string()); 

    let _guard = init_sentry(&sentry_dsn, &sentry_env)
        .map_err(|_| AppError::BadSentryDsn)?;

    let watcher = LogDirWatcher::new(&log_dir)
        .map_err(|io| AppError::WatcherFailed(io))?;
    
    println!("Begin watching '{}' for environment '{}'", &log_dir, &sentry_env);
    _ = watcher.watch();

    for event in watcher {
        match event {
            Ok(NewLogEntry(file, log)) => {
                println!("{}: {}", file.display(), *log);
                match parse_line(&log) { 
                    Ok(consul_log) => handle_log(consul_log),
                    Err(e) => {
                        println!("Failed to parse log, {}", e);
                        handle_parse_fail(&log);
                    }
                }
            },
            Ok(NoActivity(file)) => println!("{}: ... no activity ...", file.display()),
            Ok(NoFileFound) => println!("... no log files found ..."),
            Err(e) => println!("{}", e)
        }
    }
    
    Ok(())
}





#[derive(Debug)]
enum AppError {
    MissingEnvVar(String),
    BadSentryDsn,
    WatcherFailed(io::Error),
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::MissingEnvVar(name) => write!(f, "Missing environment variable '{}'!", name),
            AppError::BadSentryDsn => write!(f, "The sentry DSN provided was incorrect!"),
            AppError::WatcherFailed(io) => write!(f, "Log dir watcher failed to stream logs! {}", io)
        }
    }
}

impl std::error::Error for AppError {}







