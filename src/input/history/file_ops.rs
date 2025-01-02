use std::{
    borrow::Cow,
    collections::BTreeSet,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::PathBuf,
};

use crate::error::ShellError;

pub struct FileOps {
    file_path: PathBuf,
}

impl FileOps {
    pub fn new(file_path: PathBuf) -> Self {
        Self { file_path }
    }

    pub fn load_entries(&self) -> Result<BTreeSet<Cow<'static, str>>, ShellError> {
        let mut entries = BTreeSet::new();

        if self.file_path.exists() {
            let file = File::open(&self.file_path)?;
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

    pub fn append_entry(&self, entry: &str) -> Result<(), ShellError> {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.file_path)?;

        writeln!(file, "{}", entry)?;
        Ok(())
    }
}
