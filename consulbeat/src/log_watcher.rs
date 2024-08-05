use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::{fs, io, thread, time};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::fmt::{Formatter, write};
use std::fs::{DirEntry, File};
use std::io::{BufRead, BufReader, Error, Seek, SeekFrom};
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::SystemTime;
use crate::log_watcher::WatcherEvent::NoActivity;

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

        let mut current_log = Self::get_latest(path)?;
        
        let reader = if let Some(ref cl) = current_log {
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
        let tx = self.tx.clone(); // ???
        let path = self.path.clone();
        thread::spawn(move || loop {
            thread::sleep(time::Duration::from_secs(5));
            let latest = Self::get_latest(&path).unwrap();
            tx.send(latest).unwrap() // todo
        });
    }

    fn get_latest(path: &str) -> io::Result<Option<LogFile>> {
        let mut files = fs::read_dir(path)
            .and_then(|items| items
                .map(|x| x
                    .and_then(|y| LogFile::try_from(y)))
                .collect::<Result<Vec<_>, _>>())?;

        files.sort();

        Ok(files.into_iter().last())
    }
}

#[derive(Debug)]
pub enum WatcherError {
    IOError(std::io::Error),
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
pub enum WatcherEvent {
    NewLogEntry(PathBuf, Log),
    NoActivity(PathBuf),
    NoFileFound
}

impl Iterator for LogDirWatcher {
    type Item = Result<WatcherEvent, WatcherError>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = "".to_string();

        // keep next log file up to date from watcher thread
        match self.rx.try_recv() {
            Ok(current) => self.next = current,
            Err(TryRecvError::Empty) => {},
            Err(TryRecvError::Disconnected) => return Some(Err(WatcherError::DisconnectedError()))
        };
        
        // get the current log and reader
        let (current_log, current_reader) = match &self.curr {
            None => {
                thread::sleep(core::time::Duration::from_secs(1));
                return Some(Ok(WatcherEvent::NoFileFound))
            },
            Some(cl) => match &self.reader {
                Some(cr) => (cl, cr),
                None => {
                    if let Ok(file) = File::open(&cl.path) {
                        let cr = RefCell::new(BufReader::new(file));
                        self.reader = Some(cr);
                        (cl, self.reader.as_ref().unwrap())
                    } else { return Some(Ok(WatcherEvent::NoFileFound)) } // lies but im tired
                }
            }
        };

        // check to see if the log has rolled over
        let has_rolled_over = self.next.as_ref()
            .map(|n| current_log.path != n.path)
            .unwrap_or(false);

        let read = current_reader.borrow_mut().read_line(&mut buf);
        let event = match read {
            Ok(0) => {
                // there is no more data to read
                let path = current_log.path.clone();
                if has_rolled_over {
                    self.curr = self.next.clone();
                    self.reader = None;
                }
                thread::sleep(core::time::Duration::from_secs(1)); // why?
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

#[derive(Debug, Eq, PartialEq)]
pub struct Log(String);

impl Deref for Log {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
struct LogFile {
    path: PathBuf,
    modified: SystemTime,
    //reader: BufReader<File>
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