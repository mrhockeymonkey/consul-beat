use std::{env, fs, io, thread, time};
use std::collections::HashMap;
use std::fmt::{Display, Formatter, write};
use std::fs::{DirEntry, File};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::time::SystemTime;
use chrono::{DateTime, Duration, TimeZone, Utc};
use color_eyre::eyre::Context;
use nom::bytes::complete::{tag, take_until, take_until1, take_while1};
use nom::character::complete::{alpha1, alphanumeric1, char, digit1, line_ending};
use nom::combinator::map_res;
use nom::{Finish, IResult};
use nom::error::Error;
use nom::sequence::{terminated, tuple};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use sentry_handler::{init_sentry};
use WatcherEvent::{NewLogEntry, NoActivity};
use crate::sentry_handler::{handle_log, handle_parse_fail};
use crate::log_watcher::{LogDirWatcher, WatcherEvent};
use crate::log_watcher::WatcherEvent::NoFileFound;

mod sentry_handler;
mod log_watcher;

const SENTRY_DSN: &str = "SENTRY_DSN";
const CONSUL_LOG_DIR: &str = "CONSUL_LOG_DIR";

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    
    let sentry_dsn = env::var(SENTRY_DSN)
        .map_err(|_| AppError::MissingEnvVar(SENTRY_DSN.to_string()))?;
    
    let log_dir = env::var(CONSUL_LOG_DIR).unwrap_or("/var/log".to_string()); 

    let _guard = init_sentry(&sentry_dsn)
        .map_err(|_| AppError::BadSentryDsn)?;

    let watcher = LogDirWatcher::new(&log_dir)
        .map_err(|io| AppError::WatcherFailed(io))?;
    
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






#[derive(Debug)]
struct ConsulLog {
    timestamp: DateTime<Utc>,
    level: LogLevel,
    sub_sys: String,
    message: String
}

fn parse_line(input: &str) -> Result<ConsulLog, Error<&str>> {
    let space = take_while1(|c| c == ' ');
    
    //let (_, (timestamp, _, level, _, sub_sys, _, message)) = tuple((
    let result = tuple((
        parse_timestamp,
        &space,
        parse_log_level,
        &space,
        parse_system
    ))(input).finish().map(|(remaining, (timestamp, _, level, _, sub_sys))| ConsulLog {
        timestamp,
        level,
        sub_sys: String::from(sub_sys),
        message: String::from(remaining)
    });
    
    result
    
    // match result {
    //     Ok((_, (timestamp, _, level, _, sub_sys, _, message))) => Ok(ConsulLog {
    //         timestamp,
    //         level,
    //         sub_sys: String::from(sub_sys),
    //         message: String::from(message)
    //     }),
    //     _ => Err()
        
    // }
    
    // let log = ConsulLog {
    //     timestamp,
    //     level,
    //     sub_sys: String::from(sub_sys),
    //     message: String::from(message)
    // };
    // 
    // Some(log)
}

// parses timestamps in the format 2024-07-13T18:14:37.959Z
fn parse_timestamp(input: &str) -> IResult<&str, DateTime<Utc>> {
    let (input, (y, _, m, _, d, _, h, _, min, _, s, _, n, _)) = tuple((
        map_res(digit1::<&str, _>, |s: &str| s.parse::<i32>()),
        char('-'),
        map_res(digit1::<&str, _>, |s: &str| s.parse::<u32>()),
        char('-'),
        map_res(digit1::<&str, _>, |s: &str| s.parse::<u32>()),
        char('T'),
        map_res(digit1::<&str, _>, |s: &str| s.parse::<u32>()),
        char(':'),
        map_res(digit1::<&str, _>, |s: &str| s.parse::<u32>()),
        char(':'),
        map_res(digit1::<&str, _>, |s: &str| s.parse::<u32>()),
        char('.'),
        map_res(digit1::<&str, _>, |s: &str| s.parse::<u32>()),
        char('Z')
    ))(input)?;

    // TODO missing nano second precision here
    let datetime = Utc.with_ymd_and_hms(y, m, d, h, min, s).unwrap();

    Ok((input, datetime))
}

#[derive(Debug)]
enum LogLevel {
    Debug,
    Info,
    Warn,
    Error
}

fn parse_log_level(input: &str) -> IResult<&str, LogLevel> {
    let (input, (_, level, _)) = tuple((
        char('['),
        alpha1,
        char(']')
    ))(input)?;
    
    let log_level = match level {
        "DEBUG" => LogLevel::Debug,
        "INFO" => LogLevel::Info,
        "WARN" => LogLevel::Warn,
        "ERROR" => LogLevel::Error,
        _ => panic!("Unknwon log level")
    };
    
    Ok((input, log_level))
}

fn parse_system(input: &str) -> IResult<&str, &str> {
    let (input, system) = terminated(
        take_until1(":"),
        tag(":")
    )(input)?;
    Ok((input, system))
}

fn take_until_eol(input: &str) -> IResult<&str, &str> {
    terminated(alphanumeric1, line_ending)(input)
}
