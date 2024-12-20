// External library imports
use env_logger::Builder;
use log::LevelFilter;
use std::env;

// Standard library imports
use std::io;
use std::io::Write;
use std::time::Instant; // For execution time measurement

// Import functions
use gpscan::parse_args;
use gpscan::run;

fn init_logger(quiet_mode: bool) {
    let log_level = if quiet_mode {
        LevelFilter::Warn
    } else {
        LevelFilter::Info
    };

    Builder::from_env(env::var("LOG_LEVEL").unwrap_or_else(|_| "INFO".to_string()))
        .format(|buf, record| writeln!(buf, "[gpscan] [{}] {}", record.level(), record.args()))
        .filter(None, log_level)
        .init();
}

/// Entry point of the program.
fn main() -> io::Result<()> {
    // Start measuring execution time
    let start_time = Instant::now();

    // Parse arguments
    let matches = parse_args();
    let quiet_mode = matches.get_flag("quiet");

    // Initialize logger with quiet mode support
    init_logger(quiet_mode);

    // Parse arguments and run the program
    let matches = parse_args();
    let result = run(matches);

    // Print execution time
    // This will be printed even if quiet mode is enabled
    eprintln!(
        "[gpscan] [INFO] Execution time: {:.2?}",
        start_time.elapsed()
    );

    result
}
