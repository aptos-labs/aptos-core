use crate::diagnostics::Emitter;
use codespan::{FileId, Files};
use codespan_reporting::diagnostic::Diagnostic;
use std::io;

pub struct JsonEmitter<W: io::Write> {
    writer: W,
}

impl<W: io::Write> JsonEmitter<W> {
    pub fn new(writer: W) -> Self {
        JsonEmitter { writer }
    }
}

impl<W: io::Write> Emitter for JsonEmitter<W> {
    fn emit(&mut self, _source_files: &Files<String>, diag: &Diagnostic<FileId>) {
        serde_json::to_writer(&mut self.writer, diag).expect("emit must not fail");
        writeln!(&mut self.writer).unwrap();
    }
}
