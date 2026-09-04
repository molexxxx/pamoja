//! Regenerates the type stub for the native `pamoja._native` module.
//!
//! Run with `cargo run --bin stub_gen`. The stub is written to
//! `python/pamoja/_native.pyi` and is checked into the tree, drift-checked in CI so
//! it can never fall behind the Rust source.

use pyo3_stub_gen::Result;

fn main() -> Result<()> {
    let stub = pamoja_python::stub_info()?;
    stub.generate()?;
    Ok(())
}
