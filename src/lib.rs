// src/lib.rs

pub mod args;
pub mod xml;

pub use args::parse_args;
pub use xml::xml_escape;
