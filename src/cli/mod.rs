mod flags;

pub use flags::Flags;

pub fn parse_args() -> Result<Flags, crate::error::ShellError> {
    use std::env;
    
    let mut flags = Flags::new();
    let args: Vec<String> = env::args().skip(1).collect();
    flags.parse(&args)?;
    
    Ok(flags)
}
