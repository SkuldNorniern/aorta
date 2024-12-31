use std::collections::BTreeSet;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use crate::error::ShellError;

pub struct History {
    entries: BTreeSet<String>,
    file_path: PathBuf,
    max_entries: usize,
}

impl History {
    pub fn new(history_file: PathBuf, max_entries: usize) -> Result<Self, ShellError> {
        let mut entries = BTreeSet::new();
        
        // Load existing history if file exists
        if history_file.exists() {
            let file = File::open(&history_file)?;
            let reader = BufReader::new(file);
            
            for line in reader.lines() {
                let line = line?;
                if !line.trim().is_empty() {
                    entries.insert(line);
                }
            }
        }

        Ok(History {
            entries,
            file_path: history_file,
            max_entries,
        })
    }

    pub fn add(&mut self, entry: String) -> Result<(), ShellError> {
        if entry.trim().is_empty() {
            return Ok(());
        }

        self.entries.insert(entry.clone());

        // Trim if exceeds max size
        while self.entries.len() > self.max_entries {
            if let Some(first) = self.entries.iter().next().cloned() {
                self.entries.remove(&first);
            }
        }

        // Append to file
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.file_path)?;
            
        writeln!(file, "{}", entry)?;
        Ok(())
    }

    pub fn search_prefix(&self, prefix: &str) -> Vec<String> {
        self.entries
            .range(prefix.to_string()..)
            .take_while(|s| s.starts_with(prefix))
            .cloned()
            .collect()
    }
} 