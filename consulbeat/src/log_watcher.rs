use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::{fs, io, thread, time};
use std::fmt::{Formatter, write};
use std::fs::{DirEntry, File};
use std::io::{BufRead, BufReader, Error, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::SystemTime;
use crate::log_watcher::WatcherEvent::NoActivity;

pub struct LogDirWatcher {
    tx: Sender<LogFile>,
    rx: Receiver<LogFile>,
    path: String,
    reader: BufReader<File>,
    curr: LogFile,
    next: LogFile
}

impl LogDirWatcher {

    pub fn new(path: &str) -> io::Result<Self> {
        let (tx,rx) = mpsc::channel();

        let current_log = Self::get_latest(path)?;
        let file = File::open(current_log.path.clone())?;

        let mut reader = BufReader::new(file);
        reader.seek(SeekFrom::End(0))?;

        Ok(
            Self { 
                tx, rx, 
                path: path.to_string(), 
                reader, 
                curr: current_log.clone(), 
                next: current_log
            }
        )
    }

    pub fn watch(&self) {
        let tx = self.tx.clone(); // ???
        let path = self.path.clone();
        thread::spawn(move || loop {
            thread::sleep(time::Duration::from_secs(5));
            let latest = Self::get_latest(&path).unwrap();
            tx.send(latest).unwrap()
        });
    }

    fn get_latest(path: &str) -> io::Result<LogFile> {
        let mut files = fs::read_dir(path)
            .and_then(|items| items
                .map(|x| x
                    .and_then(|y| LogFile::try_from(y)))
                .collect::<Result<Vec<_>, _>>())?;

        files.sort();

        Ok(files.last().unwrap().clone())
    }
}

#[derive(Debug)]
pub enum WatcherError {
    IOError(std::io::Error),
    //TimeoutError(mpsc::RecvTimeoutError),
    DisconnectedError()
}

impl std::error::Error for WatcherError {}

impl std::fmt::Display for WatcherError
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            WatcherError::IOError(io) => write!(f, "IO error: {}", io),
            //WatcherError::TimeoutError(e) => write!(f, "Timeout error: {}", e),
            WatcherError::DisconnectedError() => write!(f, "Disconnected error!")
        }
    }
}

#[derive(Debug)]
pub enum WatcherEvent<`a> {
    NewLogEntry(PathBuf, Log),
    NoActivity(&PathBuf)
}

impl Iterator for LogDirWatcher {
    type Item = Result<WatcherEvent, WatcherError>; // enum? log or waiting

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = "".to_string();

        // keep next log file up to date from watcher thread
        match self.rx.try_recv() {
            Ok(current) => self.next = current,
            Err(TryRecvError::Empty) => {},
            Err(TryRecvError::Disconnected) => return Some(Err(WatcherError::DisconnectedError()))
        };

        let has_rolled_over = self.curr != self.next;

        let event = match self.reader.read_line(&mut buf){

            Ok(0) => {
                if has_rolled_over {
                    let file = File::open(self.next.path.clone()).unwrap();
                    self.curr = self.next.clone();
                    self.reader = BufReader::new(file);
                }
                thread::sleep(core::time::Duration::from_secs(1));
                Ok(NoActivity(&self.curr.path))
            },
            // Ok(0) => // read to the end of log
            //     match self.rx.recv_timeout(core::time::Duration::from_secs(1)) {
            //         Ok(latest) => {
            //             // TODO ensure old file i completely read here
            //             let file = File::open(latest.path.clone()).unwrap();
            //
            //             self.curr = latest;
            //             self.reader = BufReader::new(file);
            //
            //             //println!("Latest is {}", latest.path.display());
            //             Ok(WatcherEvent::NoActivity)
            //         },
            //         Err(mpsc::RecvTimeoutError::Timeout) => Ok(WatcherEvent::NoActivity),
            //         Err(mpsc::RecvTimeoutError::Disconnected) => Err(WatcherError::DisconnectedError()),
            // },
            Ok(_) => {
                let log = Log::new(self.curr.path.clone(), buf);
                Ok(WatcherEvent::NewLogEntry(self.curr.path, log))
            },
            Err(e) => Err(WatcherError::IOError(e))
        };

        // match self.rx.recv_timeout(core::time::Duration::from_secs(1)) {
        //     Ok(latest) => println!("Latest is {}", latest.path.display()),
        //     Err(mpsc::RecvTimeoutError::Timeout) => println!("No update, continue to read file"),
        //     Err(mpsc::RecvTimeoutError::Disconnected) => println!("Something went wrong"),
        // }

        Some(event)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Log {
    path: PathBuf,
    value: String
}

impl Log {
    fn new(path: PathBuf, value: String) -> Self {
        Self {
            path,
            value
        }
    }
    
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl std::fmt::Display for Log {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.value)
    }
}


#[derive(Clone, Ord, Eq, PartialOrd, PartialEq, Debug)]
struct LogFile {
    path: PathBuf,
    modified: SystemTime
}

impl TryFrom<DirEntry> for LogFile {
    type Error = io::Error;

    fn try_from(value: DirEntry) -> Result<Self, Self::Error> {
        let modified =  value
            .metadata()?
            .modified()?;

        Ok(Self {
            path: value.path(),
            modified
        })
    }
}