use std::{
    borrow::Cow,
    collections::BTreeSet,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::PathBuf,
};

use crate::error::ShellError;

// Separate module for file operations
mod file_ops {
    use super::*;

    pub fn load_entries(file_path: &PathBuf) -> Result<BTreeSet<Cow<'static, str>>, ShellError> {
        let mut entries = BTreeSet::new();

        if file_path.exists() {
            let file = File::open(file_path)?;
            let reader = BufReader::new(file);

            for line in reader.lines() {
                let line = line?;
                if !line.trim().is_empty() {
                    entries.insert(Cow::Owned(line));
                }
            }
        }

        Ok(entries)
    }

    pub fn append_entry(file_path: &PathBuf, entry: &str) -> Result<(), ShellError> {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(file_path)?;

        writeln!(file, "{}", entry)?;
        Ok(())
    }
}

pub struct History {
    entries: BTreeSet<Cow<'static, str>>,
    file_path: PathBuf,
    max_entries: usize,
}

impl History {
    pub fn new(history_file: PathBuf, max_entries: usize) -> Result<Self, ShellError> {
        let entries = file_ops::load_entries(&history_file)?;

        Ok(History {
            entries,
            file_path: history_file,
            max_entries,
        })
    }

    pub fn add(&mut self, entry: &str) -> Result<(), ShellError> {
        if entry.trim().is_empty() {
            return Ok(());
        }

        self.entries.insert(Cow::Owned(entry.to_owned()));

        // Trim if exceeds max size
        while self.entries.len() > self.max_entries {
            if let Some(first) = self.entries.iter().next().cloned() {
                self.entries.remove(&first);
            }
        }

        file_ops::append_entry(&self.file_path, entry)?;
        Ok(())
    }

    pub fn search_prefix(&self, prefix: &str) -> Vec<String> {
        self.entries
            .iter()
            .filter(|s| s.starts_with(prefix))
            .map(|s| s.to_string())
            .collect()
    }
}
