mod expander;
mod glob;

pub use expander::PathExpander;
pub use glob::expand_glob;

pub fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(std::path::PathBuf::from)
}
