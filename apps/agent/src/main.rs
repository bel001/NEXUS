use std::io::{self, Write};

fn main() {
    let mut args = std::env::args().skip(1);
    let first = args.next();
    if args.next().is_some() || !matches!(first.as_deref(), None | Some("--telemetry-memory")) {
        eprintln!("supported option: --telemetry-memory");
        std::process::exit(2);
    }
    let output = if first.as_deref() == Some("--telemetry-memory") {
        adaptive_agent::memory_telemetry_json()
    } else {
        adaptive_agent::run()
    };
    match output {
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
