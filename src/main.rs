use aorta::cli;
use aorta::shell::Shell;

fn main() -> Result<(), aorta::error::ShellError> {
    // Parse command-line flags
    let flags = cli::parse_args()?;

    if flags.is_set("help") {
        flags.print_help();
        return Ok(());
    }

    if flags.is_set("version") {
        println!("Aorta {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if !flags.is_set("quiet") {
        // FEAT: TODO: Add Support of using .motd or .aorta_motd to display a message
        // | or maybe use a .config/aorta/aorta.toml and direct the motd file to display a message
    }

    let mut shell = Shell::new(flags)?;
    shell.run()
}
