pub mod lint;
use std::path::PathBuf;
use anyhow::{Result, Ok};
use lint::context::VisitorContext;

pub fn aptos_lint(path: PathBuf) -> Result<VisitorContext> {
    lint::main(path).and_then(|c| Ok(c))
}