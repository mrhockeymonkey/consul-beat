use std::fs::File;
use std::io::{BufRead, BufReader};
use chrono::{DateTime, TimeZone, Utc};
use nom::bytes::complete::{tag, take_until, take_until1, take_while1};
use nom::character::complete::{alpha1, alphanumeric1, char, digit1, line_ending};
use nom::combinator::map_res;
use nom::{Finish, IResult};
use nom::error::Error;
use nom::sequence::{terminated, tuple};
use sentry_handler::{init_sentry};
use crate::sentry_handler::handle_log;

mod sentry_handler;

fn main() {
    let _guard = init_sentry();

    let file = File::open("/home/scott/code/consul-beat/consul-1721078564262596963.log").unwrap();
    let reader = BufReader::new(file);

    for line in reader.lines() {
        if let Ok(line) = line {
            // if let Some(log) = parse_line(line.as_str()){
            //     dbg!(&log);
            //     handle_log(log);
            // }
            
            match parse_line(line.as_str()) {
                Ok(log) => handle_log(log),
                Err(e) => println!("Error: {}", e)
            }
        }
    }
}

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
