use aorta::flags::Flags;
use aorta::shell::Shell;
use std::env;

fn main() -> Result<(), aorta::error::ShellError> {
    let mut flags = Flags::new();
    let args: Vec<String> = env::args().skip(1).collect();
    flags.parse(&args)?;

    if flags.is_set("help") {
        flags.print_help();
        return Ok(());
    }

    if flags.is_set("version") {
        println!("Aorta {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if !flags.is_set("quiet") {
        // FEAT: TODO: Add Support of useing .motd or .aorta_motd to display a message
        // | or maybe use a .config/aorta/aorta.toml and direct the motd file to display a message
    }

    let mut shell = Shell::new(flags)?;
    shell.run()
}
