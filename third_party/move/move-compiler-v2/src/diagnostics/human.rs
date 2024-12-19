use crate::diagnostics::Emitter;
use codespan::{FileId, Files};
use codespan_reporting::{
    diagnostic::Diagnostic,
    term::{emit, termcolor::WriteColor, Config},
};

pub struct HumanEmitter<'w, W: WriteColor> {
    writer: &'w mut W,
}

impl<'w, W> HumanEmitter<'w, W>
where
    W: WriteColor,
{
    pub fn new(writer: &'w mut W) -> Self {
        HumanEmitter { writer }
    }
}

impl<'w, W> Emitter for HumanEmitter<'w, W>
where
    W: WriteColor,
{
    fn emit(&mut self, source_files: &Files<String>, diag: &Diagnostic<FileId>) {
        emit(&mut self.writer, &Config::default(), source_files, diag).expect("emit must not fail")
    }
}
