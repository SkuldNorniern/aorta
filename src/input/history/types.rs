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
    pub fn new_command(
        command: impl Into<Cow<'static, str>>,
        exit_code: i32,
        duration: u64,
    ) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_entry_new_command() {
        let entry = HistoryEntry::new_command("echo hello", 0, 100);
        match &entry {
            HistoryEntry::Command {
                command,
                exit_code,
                duration,
                ..
            } => {
                assert_eq!(command.as_ref(), "echo hello");
                assert_eq!(*exit_code, 0);
                assert_eq!(*duration, 100);
            }
            _ => panic!("expected Command variant"),
        }
    }

    #[test]
    fn test_history_entry_new_event() {
        let entry = HistoryEntry::new_event("session start");
        match &entry {
            HistoryEntry::Event { description, .. } => {
                assert_eq!(description.as_ref(), "session start");
            }
            _ => panic!("expected Event variant"),
        }
    }

    #[test]
    fn test_history_search_mode_eq() {
        assert_eq!(HistorySearchMode::Prefix, HistorySearchMode::Prefix);
        assert_ne!(HistorySearchMode::Prefix, HistorySearchMode::Contains);
    }

    #[test]
    fn test_history_stats_default() {
        let stats = HistoryStats::default();
        assert_eq!(stats.total_commands, 0);
        assert_eq!(stats.unique_commands, 0);
        assert_eq!(stats.average_duration, 0);
    }
}
