use std::borrow::Cow;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub enum HistoryEntry {
    Command {
        command: Cow<'static, str>,
        timestamp: u64,
        exit_code: i32,
        duration: u64,
    },
    Event {
        description: Cow<'static, str>,
        timestamp: u64,
    },
}

impl HistoryEntry {
    pub fn new_command(command: impl Into<Cow<'static, str>>, exit_code: i32, duration: u64) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        HistoryEntry::Command {
            command: command.into(),
            timestamp,
            exit_code,
            duration,
        }
    }

    pub fn new_event(description: impl Into<Cow<'static, str>>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        HistoryEntry::Event {
            description: description.into(),
            timestamp,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HistorySearchMode {
    Prefix,
    Contains,
    TimeRange(u64, u64),
    LastN(usize),
}

#[derive(Debug, Default)]
pub struct HistoryStats {
    pub total_commands: usize,
    pub unique_commands: usize,
    pub failed_commands: usize,
    pub average_duration: u64,
    pub most_used: Vec<(String, usize)>,
}
