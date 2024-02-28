pub mod lint;
use std::path::PathBuf;

use codespan::{FileId, Files};
use codespan_reporting::diagnostic::Diagnostic;

pub fn move_lint(path: PathBuf) -> (Vec<Diagnostic<FileId>>, Files<String>) {
    lint::main(path)
}
