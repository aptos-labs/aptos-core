pub mod lint;
use std::path::PathBuf;

pub fn move_lint(path: PathBuf) {
    lint::main(path)
}
