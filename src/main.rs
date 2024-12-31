use aorta::shell::Shell;
use aorta::flags::Flags;
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
        println!("Aorta v0.1.0");
        return Ok(());
    }

    if !flags.is_set("quiet") {
        println!("Welcome to Aorta Shell!");
        println!("Type 'exit' to quit.");
    }

    let mut shell = Shell::new(flags)?;
    shell.run()
}
