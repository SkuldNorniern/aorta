use super::{Command, CommandError};
use crate::input::history::{History, HistoryEntry, HistorySearchMode};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct HistoryCommand {
    history: Arc<Mutex<History>>,
}

impl HistoryCommand {
    pub fn new(history: Arc<Mutex<History>>) -> Self {
        Self { history }
    }

    fn show_recent(&self, count: usize) -> Result<(), CommandError> {
        let history = self
            .history
            .lock()
            .map_err(|_| CommandError::ExecutionError("Failed to lock history".to_string()))?;

        for entry in history.get_recent(count) {
            println!("{}", self.format_entry(entry));
        }
        Ok(())
    }

    fn search(&self, args: &[String]) -> Result<(), CommandError> {
        let mode = match args.first().map(|s| s.as_str()) {
            Some("--prefix") => HistorySearchMode::Prefix,
            Some("--contains") => HistorySearchMode::Contains,
            Some("--last") => {
                let n = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10);
                HistorySearchMode::LastN(n)
            }
            _ => HistorySearchMode::Contains,
        };

        let query = args.last().map(String::as_str).unwrap_or("");

        let history = self
            .history
            .lock()
            .map_err(|_| CommandError::ExecutionError("Failed to lock history".to_string()))?;

        for entry in history.search(mode, query) {
            println!("{}", self.format_entry(entry));
        }
        Ok(())
    }

    fn show_statistics(&self) -> Result<(), CommandError> {
        let history = self
            .history
            .lock()
            .map_err(|_| CommandError::ExecutionError("Failed to lock history".to_string()))?;

        let stats = history.calculate_stats();
        println!("History Statistics:");
        println!("Total commands: {}", stats.total_commands);
        println!("Unique commands: {}", stats.unique_commands);
        println!("Failed commands: {}", stats.failed_commands);
        println!("Average duration: {}ms", stats.average_duration);
        println!("\nMost used commands:");
        for (cmd, count) in stats.most_used.iter().take(5) {
            println!("  {} ({}x)", cmd, count);
        }
        Ok(())
    }

    fn format_entry(&self, entry: &HistoryEntry) -> String {
        match entry {
            HistoryEntry::Command {
                command,
                timestamp,
                exit_code,
                duration,
            } => {
                let time = format_timestamp(*timestamp);
                format!(
                    "{} [{}] ({}) {} [{}ms]",
                    time,
                    if *exit_code == 0 { "✓" } else { "✗" },
                    exit_code,
                    command,
                    duration
                )
            }
            HistoryEntry::Event {
                description,
                timestamp,
            } => {
                let time = format_timestamp(*timestamp);
                format!("{} [EVENT] {}", time, description)
            }
        }
    }

    fn delete_entry(&self, args: &[String]) -> Result<(), CommandError> {
        if args.is_empty() {
            return Err(CommandError::InvalidArguments("Index required".to_string()));
        }

        let index = args[0]
            .parse::<usize>()
            .map_err(|_| CommandError::InvalidArguments("Invalid index".to_string()))?;

        let mut history = self
            .history
            .lock()
            .map_err(|_| CommandError::ExecutionError("Failed to lock history".to_string()))?;

        history.delete_at(index).map_err(CommandError::HistoryError)
    }
}

impl Command for HistoryCommand {
    fn execute(&self, args: &[String]) -> Result<(), CommandError> {
        if args.is_empty() {
            return self.show_recent(10);
        }

        match args[0].as_str() {
            "search" => self.search(&args[1..]),
            "stats" => self.show_statistics(),
            "clear" => {
                let mut history = self.history.lock().map_err(|_| {
                    CommandError::ExecutionError("Failed to lock history".to_string())
                })?;
                history
                    .clear()
                    .map_err(|e| CommandError::ExecutionError(e.to_string()))
            }
            "delete" => self.delete_entry(&args[1..]),
            _ => Err(CommandError::InvalidArguments(
                "Unknown history subcommand".to_string(),
            )),
        }
    }
}

fn format_timestamp(timestamp: u64) -> String {
    let secs = timestamp % 60;
    let mins = (timestamp / 60) % 60;
    let hours = (timestamp / 3600) % 24;
    format!("{:02}:{:02}:{:02}", hours, mins, secs)
}
