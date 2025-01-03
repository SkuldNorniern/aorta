use std::collections::BTreeMap;
use super::{HistoryEntry, HistoryError, HistorySearchMode, HistoryStats};
use super::file_ops::FileOps;

pub struct HistoryStorage {
    entries: BTreeMap<u64, HistoryEntry>,
    file_ops: FileOps,
    max_entries: usize,
}

impl HistoryStorage {
    pub fn add_entry(&mut self, entry: HistoryEntry) -> Result<(), HistoryError> {
        let timestamp = match &entry {
            HistoryEntry::Command { timestamp, .. } => *timestamp,
            HistoryEntry::Event { timestamp, .. } => *timestamp,
        };
        
        self.entries.insert(timestamp, entry.clone());
        self.trim_if_needed();
        self.file_ops.append_entry(&entry)
    }

    pub fn search(&self, mode: HistorySearchMode, query: &str) -> Vec<&HistoryEntry> {
        match mode {
            HistorySearchMode::Prefix => self.search_by_prefix(query),
            HistorySearchMode::Contains => self.search_by_contains(query),
            HistorySearchMode::Regex => self.search_by_regex(query),
            HistorySearchMode::TimeRange(start, end) => self.search_by_timerange(start, end),
            HistorySearchMode::LastN(n) => self.get_last_n(n),
        }
    }

    pub fn calculate_stats(&self) -> HistoryStats {
        // Implementation for calculating history statistics
    }
} 