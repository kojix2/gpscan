// Standard library imports
use std::io;

// Import functions
use gpscan::parse_args;
use gpscan::run;

/// Entry point of the program.
fn main() -> io::Result<()> {
    let matches = parse_args();
    run(matches)
}
