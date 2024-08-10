use std::{fs, io, thread};
use std::cell::RefCell;
use std::fmt::Formatter;
use std::fs::{DirEntry, File};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::mpsc;
use std::time::{SystemTime, Duration};

use crate::log_watcher::WatcherEvent::NoActivity;

const FIVE_SECONDS: Duration = Duration::from_secs(5);
const ONE_SECONDS: Duration = Duration::from_secs(1);

/// A single log line read from a file
#[derive(Debug, Eq, PartialEq)]
pub struct Log(String);

impl Deref for Log {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A single log file within the specified log dir
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
struct LogFile {
    path: PathBuf,
    modified: SystemTime // TODO do we need this?
}

impl TryFrom<DirEntry> for LogFile {
    type Error = io::Error;

    fn try_from(value: DirEntry) -> Result<Self, Self::Error> {
        let modified =  value
            .metadata()?
            .modified()?;

        Ok(Self {
            path: value.path(),
            modified,
        })
    }
}

/// Watches for all files in a given directory and can be iterated through
/// to receive log events as they occur
pub struct LogDirWatcher {
    tx: Sender<Option<LogFile>>,
    rx: Receiver<Option<LogFile>>,
    path: String,
    reader: Option<RefCell<BufReader<File>>>,
    curr: Option<LogFile>,
    next: Option<LogFile>
}

impl LogDirWatcher {

    pub fn new(path: &str) -> io::Result<Self> {
        let (tx, rx) = mpsc::channel();

        let current_log = Self::get_latest(path)?;
        
        let reader = if let Some(ref cl) = current_log {
            // when starting to watch a dir, we want to start from the latest current log
            let file = File::open(&cl.path)?;
            let mut r = BufReader::new(file);
            r.seek(SeekFrom::End(0))?;
            Some(RefCell::new(r))
        } else { None };

        Ok(
            Self {
                tx, rx,
                path: path.to_string(),
                reader,
                curr: current_log,
                next: None
            }
        )
    }

    pub fn watch(&self) {
        let tx = self.tx.clone();
        let path = self.path.clone();
        thread::spawn(move || {
            loop {
                thread::sleep(FIVE_SECONDS);
                match Self::get_latest(&path) {
                    Ok(latest_log_file) => tx.send(latest_log_file)
                        .unwrap_or_else(|e| eprintln!("Failed to send message to receiver! Error was {}", e)),
                    Err(e) => eprintln!("Failed to detect latest log file! Error was {}", e)
                }
            }
        });
    }

    fn get_latest(path: &str) -> io::Result<Option<LogFile>> {
        let mut files = fs::read_dir(path)
            .and_then(|items| items
                .map(|x| x
                    .and_then(|y| LogFile::try_from(y)))
                .collect::<Result<Vec<LogFile>, _>>())?;

        files.sort();

        Ok(files.into_iter().last())
    }
}

#[derive(Debug)]
pub enum WatcherError {
    IOError(io::Error),
    DisconnectedError()
}

impl std::error::Error for WatcherError {}

impl std::fmt::Display for WatcherError
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            WatcherError::IOError(io) => write!(f, "IO error: {}", io),
            WatcherError::DisconnectedError() => write!(f, "Disconnected error!")
        }
    }
}

#[derive(Debug)]
pub enum WatcherEvent {
    NewLogEntry(PathBuf, Log),
    NoActivity(PathBuf),
    NoFileFound
}

impl Iterator for LogDirWatcher {
    type Item = Result<WatcherEvent, WatcherError>;

    fn next(&mut self) -> Option<Self::Item> {
        
        // keep next log file up to date from watcher thread
        match self.rx.try_recv() {
            Ok(current) => self.next = current,
            Err(TryRecvError::Empty) => {},
            Err(TryRecvError::Disconnected) => return Some(Err(WatcherError::DisconnectedError()))
        };
        
        // get the current log and reader
        let (current_log, current_reader) = match &self.curr {
            None => {
                // there is no current file so wait for one to appear instead of spinning
                thread::sleep(ONE_SECONDS);
                return Some(Ok(WatcherEvent::NoFileFound))
            },
            Some(cl) => match &self.reader {
                Some(cr) => (cl, cr),
                None => {
                    // we have a log but no reader yet so try to create one
                    match File::open(&cl.path) {
                        Ok(file) => {
                            self.reader = Some(RefCell::new(BufReader::new(file)));
                            (cl, self.reader.as_ref().expect("We should have a guaranteed reader at this point!"))
                        },
                        Err(e) => return Some(Err(WatcherError::IOError(e)))
                    }
                }
            }
        };

        // check to see if the log has rolled over
        let has_rolled_over = self.next.as_ref()
            .map(|n| current_log.path != n.path)
            .unwrap_or(false);

        let mut buf = "".to_string();
        let read = current_reader.borrow_mut().read_line(&mut buf);
        let event = match read {
            Ok(0) => {
                // there is no more data to read, check if there is a new log
                let path = current_log.path.clone();
                
                if has_rolled_over {
                    self.curr = self.next.clone();
                    self.reader = None;
                }
                else {
                    thread::sleep(ONE_SECONDS);
                }

                Ok(NoActivity(path))
            },
            Ok(_) => {
                // we have read a line from the log
                Ok(WatcherEvent::NewLogEntry(current_log.path.clone(), Log(buf)))
            },
            Err(e) => Err(WatcherError::IOError(e))
        };

        Some(event)
    }
}

