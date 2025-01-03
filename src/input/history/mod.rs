mod file_ops;
pub mod types;

use self::file_ops::FileOps;
pub use self::types::{HistoryEntry, HistorySearchMode, HistoryStats};
use std::{
    collections::{HashMap, VecDeque},
    fmt,
    path::PathBuf,
};

#[derive(Debug)]
pub enum HistoryError {
    IoError(std::io::Error),
    InvalidIndex(usize),
    LockError(String),
    FileOperationError(String),
    EmptyCommand,
}

impl From<std::io::Error> for HistoryError {
    fn from(err: std::io::Error) -> Self {
        HistoryError::IoError(err)
    }
}

impl fmt::Display for HistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HistoryError::IoError(e) => write!(f, "IO error: {}", e),
            HistoryError::InvalidIndex(idx) => write!(f, "Invalid history index: {}", idx),
            HistoryError::LockError(msg) => write!(f, "Lock error: {}", msg),
            HistoryError::FileOperationError(msg) => write!(f, "File operation error: {}", msg),
            HistoryError::EmptyCommand => write!(f, "Empty command"),
        }
    }
}

pub struct History {
    entries: VecDeque<HistoryEntry>,
    command_frequencies: HashMap<String, usize>,
    file_ops: FileOps,
    max_entries: usize,
}

impl History {
    pub fn new(history_file: PathBuf, max_entries: usize) -> Result<Self, HistoryError> {
        let file_ops = FileOps::new(history_file);
        let raw_entries = file_ops
            .load_entries()
            .map_err(|e| HistoryError::FileOperationError(e.to_string()))?;

        let mut command_frequencies = HashMap::new();
        let entries: VecDeque<_> = raw_entries
            .into_iter()
            .map(|entry| {
                if let HistoryEntry::Command { command, .. } = &entry {
                    *command_frequencies.entry(command.to_string()).or_insert(0) += 1;
                }
                entry
            })
            .collect();

        Ok(History {
            entries,
            command_frequencies,
            file_ops,
            max_entries,
        })
    }

    pub fn add(&mut self, command: &str) -> Result<(), HistoryError> {
        self.add_with_details(command, 0, 0)
    }

    pub fn get_recent(&self, count: usize) -> Vec<&HistoryEntry> {
        self.entries.iter().rev().take(count).collect()
    }

    pub fn clear(&mut self) -> Result<(), HistoryError> {
        self.entries.clear();
        self.command_frequencies.clear();
        self.file_ops = FileOps::new(self.file_ops.get_path().to_path_buf());
        Ok(())
    }

    pub fn delete_at(&mut self, index: usize) -> Result<(), HistoryError> {
        if index >= self.entries.len() {
            return Err(HistoryError::InvalidIndex(index));
        }

        if let Some(HistoryEntry::Command { command, .. }) = self.entries.remove(index) {
            let command_str = command.into_owned();
            if let Some(count) = self.command_frequencies.get_mut(&command_str) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    self.command_frequencies.remove(&command_str);
                }
            }
        }

        self.rewrite_history_file()?;
        Ok(())
    }

    fn rewrite_history_file(&mut self) -> Result<(), HistoryError> {
        self.file_ops = FileOps::new(self.file_ops.get_path().to_path_buf());
        for entry in &self.entries {
            self.file_ops
                .append_entry(entry)
                .map_err(|e| HistoryError::FileOperationError(e.to_string()))?;
        }
        Ok(())
    }

    pub fn search(&self, mode: HistorySearchMode, query: &str) -> Vec<&HistoryEntry> {
        match mode {
            HistorySearchMode::Prefix => self.search_by_prefix(query),
            HistorySearchMode::Contains => self.search_by_contains(query),
            HistorySearchMode::TimeRange(start, end) => self.search_by_timerange(start, end),
            HistorySearchMode::LastN(n) => self.get_recent(n),
        }
    }

    fn search_by_prefix(&self, prefix: &str) -> Vec<&HistoryEntry> {
        self.entries
            .iter()
            .filter(|entry| match entry {
                HistoryEntry::Command { command, .. } => command.starts_with(prefix),
                HistoryEntry::Event { description, .. } => description.starts_with(prefix),
            })
            .collect()
    }

    fn search_by_contains(&self, substring: &str) -> Vec<&HistoryEntry> {
        self.entries
            .iter()
            .filter(|entry| match entry {
                HistoryEntry::Command { command, .. } => command.contains(substring),
                HistoryEntry::Event { description, .. } => description.contains(substring),
            })
            .collect()
    }

    fn search_by_timerange(&self, start: u64, end: u64) -> Vec<&HistoryEntry> {
        self.entries
            .iter()
            .filter(|entry| match entry {
                HistoryEntry::Command { timestamp, .. } | HistoryEntry::Event { timestamp, .. } => {
                    *timestamp >= start && *timestamp <= end
                }
            })
            .collect()
    }

    fn trim_entries(&mut self) {
        while self.entries.len() > self.max_entries {
            if let Some(HistoryEntry::Command { command, .. }) = self.entries.pop_front() {
                let command_str = command.into_owned();
                if let Some(count) = self.command_frequencies.get_mut(&command_str) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        self.command_frequencies.remove(&command_str);
                    }
                }
            }
        }
    }

    pub fn add_with_details(
        &mut self,
        command: &str,
        exit_code: i32,
        duration: u64,
    ) -> Result<(), HistoryError> {
        if command.trim().is_empty() {
            return Err(HistoryError::EmptyCommand);
        }

        let entry = HistoryEntry::new_command(
            command.to_string(),
            exit_code,
            duration,
        );

        // Save to file first
        self.file_ops
            .append_entry(&entry)
            .map_err(|e| HistoryError::FileOperationError(e.to_string()))?;

        // Update frequency counter
        *self.command_frequencies
            .entry(command.to_string())
            .or_insert(0) += 1;

        // Then update memory
        self.entries.push_back(entry);
        self.trim_entries();

        Ok(())
    }

    pub fn calculate_stats(&self) -> HistoryStats {
        let mut stats = HistoryStats::default();
        let mut total_duration = 0u64;

        stats.total_commands = self
            .entries
            .iter()
            .filter(|entry| matches!(entry, HistoryEntry::Command { .. }))
            .count();

        stats.unique_commands = self.command_frequencies.len();

        for entry in &self.entries {
            if let HistoryEntry::Command {
                exit_code,
                duration,
                ..
            } = entry
            {
                if *exit_code != 0 {
                    stats.failed_commands += 1;
                }
                total_duration += duration;
            }
        }

        stats.average_duration = if stats.total_commands > 0 {
            total_duration / (stats.total_commands as u64)
        } else {
            0
        };

        let mut commands: Vec<_> = self
            .command_frequencies
            .iter()
            .filter(|(_, &count)| count > 0)
            .map(|(cmd, &count)| (cmd.clone(), count))
            .collect();

        commands.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        stats.most_used = commands.into_iter().take(10).collect();

        stats
    }
}
