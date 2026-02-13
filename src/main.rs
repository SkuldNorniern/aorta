use aorta::flags::Flags;
use aorta::path;
use aorta::shell::Shell;
use std::env;
use std::fs;
use std::io;

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
        if let Err(e) = display_motd() {
            if e.kind() != io::ErrorKind::NotFound {
                eprintln!("Warning: MOTD: {}", e);
            }
        }
    }

    let mut shell = Shell::new(flags)?;
    shell.run()
}

fn display_motd() -> io::Result<()> {
    let home = path::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Home directory not found"))?;

    let candidates = [
        std::path::PathBuf::from("/etc/aorta/motd"),
        home.join(".aorta_motd"),
        home.join(".motd"),
    ];

    for path in &candidates {
        if path.exists() {
            let content = fs::read_to_string(path)?;
            if !content.trim().is_empty() {
                print!("{}", content);
                if !content.ends_with('\n') {
                    println!();
                }
            }
            return Ok(());
        }
    }

    Err(io::Error::new(io::ErrorKind::NotFound, "No MOTD file"))
}
