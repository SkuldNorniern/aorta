mod file_ops;

use std::{
    borrow::Cow,
    collections::BTreeSet,
    path::PathBuf,
};

use crate::error::ShellError;
use self::file_ops::FileOps;

pub struct History {
    entries: BTreeSet<Cow<'static, str>>,
    file_ops: FileOps,
    max_entries: usize,
}

impl History {
    pub fn new(history_file: PathBuf, max_entries: usize) -> Result<Self, ShellError> {
        let file_ops = FileOps::new(history_file);
        let entries = file_ops.load_entries()?;

        Ok(History {
            entries,
            file_ops,
            max_entries,
        })
    }

    pub fn add(&mut self, entry: &str) -> Result<(), ShellError> {
        if entry.trim().is_empty() {
            return Ok(());
        }

        self.entries.insert(Cow::Owned(entry.to_owned()));
        self.trim_entries();
        self.file_ops.append_entry(entry)?;
        
        Ok(())
    }

    pub fn search_prefix(&self, prefix: &str) -> Vec<String> {
        self.entries
            .iter()
            .filter(|s| s.starts_with(prefix))
            .map(|s| s.to_string())
            .collect()
    }

    fn trim_entries(&mut self) {
        while self.entries.len() > self.max_entries {
            if let Some(first) = self.entries.iter().next().cloned() {
                self.entries.remove(&first);
            }
        }
    }
} 