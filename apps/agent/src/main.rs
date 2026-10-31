use std::io::{self, Write};

fn main() {
    match adaptive_agent::run() {
        Ok(json) => {
            if let Err(error) = writeln!(io::stdout().lock(), "{json}")
                && error.kind() != io::ErrorKind::BrokenPipe
            {
                eprintln!("hardware inventory output failed: {error}");
                std::process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("hardware inventory serialization failed: {error}");
            std::process::exit(1);
        }
    }
}
