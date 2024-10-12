// Standard library imports
use std::io;
use std::time::Instant; // For execution time measurement

// Import functions
use gpscan::parse_args;
use gpscan::run;

/// Entry point of the program.
fn main() -> io::Result<()> {
    // Start timer
    let start_time = Instant::now();

    // Parse arguments and run the program
    let matches = parse_args();
    let result = run(matches);

    // Print execution time
    eprintln!("[gpscan] Execution time: {:.2?}", start_time.elapsed());

    result
}
