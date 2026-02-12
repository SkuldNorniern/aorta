mod expander;

pub use expander::PathExpander;

pub fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(std::path::PathBuf::from)
}
