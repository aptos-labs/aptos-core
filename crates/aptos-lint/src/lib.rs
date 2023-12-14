pub mod lint;
use std::path::PathBuf;

pub fn aptos_lint(path: PathBuf) {
    lint::main(path)
}
