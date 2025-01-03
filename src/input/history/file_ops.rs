use std::{
    borrow::Cow,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::PathBuf,
};

use super::types::HistoryEntry;
use super::HistoryError;

pub struct FileOps {
    file_path: PathBuf,
}

impl FileOps {
    pub fn new(file_path: PathBuf) -> Self {
        Self { file_path }
    }

    pub fn get_path(&self) -> &PathBuf {
        &self.file_path
    }

    pub fn load_entries(&self) -> Result<Vec<HistoryEntry>, HistoryError> {
        let mut entries = Vec::new();

        if self.file_path.exists() {
            let file = File::open(&self.file_path).map_err(HistoryError::IoError)?;
            let reader = BufReader::new(file);

            for line in reader.lines() {
                let line = line.map_err(HistoryError::IoError)?;
                if !line.trim().is_empty() {
                    let parts: Vec<&str> = line.split('|').collect();
                    match parts.as_slice() {
                        [command, timestamp, exit_code, duration] => {
                            let timestamp = timestamp.parse().map_err(|_| {
                                HistoryError::FileOperationError("Invalid timestamp".into())
                            })?;
                            let exit_code = exit_code.parse().map_err(|_| {
                                HistoryError::FileOperationError("Invalid exit code".into())
                            })?;
                            let duration = duration.parse().map_err(|_| {
                                HistoryError::FileOperationError("Invalid duration".into())
                            })?;

                            entries.push(HistoryEntry::Command {
                                command: Cow::Owned(command.to_string()),
                                timestamp,
                                exit_code,
                                duration,
                            });
                        }
                        _ => {
                            entries.push(HistoryEntry::new_command(line, 0, 0));
                        }
                    }
                }
            }
        }

        Ok(entries)
    }

    pub fn append_entry(&self, entry: &HistoryEntry) -> Result<(), HistoryError> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
            .map_err(HistoryError::IoError)?;

        match entry {
            HistoryEntry::Command {
                command,
                timestamp,
                exit_code,
                duration,
            } => {
                writeln!(file, "{}|{}|{}|{}", command, timestamp, exit_code, duration)
                    .map_err(HistoryError::IoError)?;
            }
            HistoryEntry::Event {
                description,
                timestamp,
            } => {
                writeln!(file, "{}|{}|0|0", description, timestamp)
                    .map_err(HistoryError::IoError)?;
            }
        }

        Ok(())
    }
}
