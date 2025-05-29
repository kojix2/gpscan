// External library imports
use env_logger::Builder;
use log::LevelFilter;

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

    Builder::from_default_env()
        .format(|buf, record| writeln!(buf, "[gpscan] [{}] {}", record.level(), record.args()))
        .filter(None, log_level)
        .init();
}

/// Entry point of the program.
fn main() {
    // Start measuring execution time
    let start_time = Instant::now();

    // Parse arguments
    let matches = parse_args();
    let quiet_mode = matches.get_flag("quiet");

    // Initialize logger with quiet mode support
    init_logger(quiet_mode);

    // Run the program
    let result = run(matches);

    // Print execution time
    // This will be printed even if quiet mode is enabled
    eprintln!(
        "[gpscan] [INFO] Execution time: {:.2?}",
        start_time.elapsed()
    );

    // Handle the result and set appropriate exit code
    if let Err(e) = result {
        eprintln!("[gpscan] [ERROR] {}", e);
        std::process::exit(1);
    }
}
