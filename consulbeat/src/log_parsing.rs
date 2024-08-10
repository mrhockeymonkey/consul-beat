use chrono::{DateTime, TimeZone, Utc};
use nom::bytes::complete::{tag, take_until1, take_while1};
use nom::error::{Error, ErrorKind};
use nom::Err::Failure;
use nom::{Finish, IResult};
use nom::character::complete::{alpha1, char, digit1};
use nom::combinator::map_res;
use nom::sequence::{terminated, tuple};

/// Represent a single log line from Consul, parsed into components
#[derive(Debug)]
pub struct ConsulLog {
    _timestamp: DateTime<Utc>,
    level: ConsulLogLevel,
    _sub_sys: String,
    message: String
}

impl ConsulLog {
    pub fn level(&self) -> &ConsulLogLevel {
        &self.level
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}


#[derive(Debug)]
pub enum ConsulLogLevel {
    Debug,
    Info,
    Warn,
    Error
}

pub fn parse_line(input: &str) -> Result<ConsulLog, Error<&str>> {
    let space = take_while1(|c| c == ' ');

    let result = tuple((
        parse_timestamp,
        &space,
        parse_log_level,
        &space,
        parse_system
    ))(input).finish().map(|(remaining, (timestamp, _, level, _, sub_sys))| ConsulLog {
        _timestamp: timestamp,
        level,
        _sub_sys: String::from(sub_sys),
        message: String::from(remaining)
    });

    result
}

/// parses timestamps in the format 2024-07-13T18:14:37.959Z
fn parse_timestamp(input: &str) -> IResult<&str, DateTime<Utc>> {
    let (input, (y, _, m, _, d, _, h, _, min, _, s, _, _, _)) = tuple((
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

fn parse_log_level(input: &str) -> IResult<&str, ConsulLogLevel> {
    let (input, (_, level, _)) = tuple((
        char('['),
        alpha1,
        char(']')
    ))(input)?;
    
    let log_level = match level {
        "DEBUG" => ConsulLogLevel::Debug,
        "INFO" => ConsulLogLevel::Info,
        "WARN" => ConsulLogLevel::Warn,
        "ERROR" => ConsulLogLevel::Error,
        _ => return Err(Failure(Error::new(input, ErrorKind::Fail)))
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
