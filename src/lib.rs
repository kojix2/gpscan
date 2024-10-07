// src/lib.rs

pub mod args;
pub mod filesystem;
pub mod xml;

pub use args::parse_args;
pub use filesystem::run;
pub use xml::xml_escape;
