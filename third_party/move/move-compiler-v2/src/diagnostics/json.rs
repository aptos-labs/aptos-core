use crate::diagnostics::Emitter;
use codespan::{FileId, Files};
use codespan_reporting::diagnostic::Diagnostic;
use std::io::Write;

pub struct JsonEmitter<'w, W: Write> {
    writer: &'w mut W,
}

impl<'w, W: Write> JsonEmitter<'w, W> {
    pub fn new(writer: &'w mut W) -> Self {
        JsonEmitter { writer }
    }
}

impl<'w, W: Write> Emitter for JsonEmitter<'w, W> {
    fn emit(&mut self, _source_files: &Files<String>, diag: &Diagnostic<FileId>) {
        serde_json::to_writer(&mut self.writer, diag).expect("emit must not fail");
        writeln!(&mut self.writer).unwrap();
    }
}
