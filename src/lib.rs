// src/lib.rs
use pyo3::prelude::*;

// Re-export your Rust modules for the CLI and for external users
pub mod analysis;
pub mod config;
pub mod experiments;
pub mod generators;
pub mod lambda;
pub mod supercollider;
pub mod utils;

// New Python wrapper module
mod python;

#[pymodule]
fn alchemy(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    python::register(m)
}
