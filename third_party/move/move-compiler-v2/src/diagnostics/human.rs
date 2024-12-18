use crate::diagnostics::Emitter;
use codespan::{FileId, Files};
use codespan_reporting::{
    diagnostic::Diagnostic,
    term::{emit, termcolor::WriteColor, Config},
};

pub struct HumanEmitter<W: WriteColor> {
    writer: W,
}

impl<W> HumanEmitter<W>
where
    W: WriteColor,
{
    pub fn new(writer: W) -> Box<Self> {
        let emitter = HumanEmitter { writer };
        Box::new(emitter)
    }
}

impl<W> Emitter for HumanEmitter<W>
where
    W: WriteColor,
{
    fn emit(&mut self, source_files: &Files<String>, diag: &Diagnostic<FileId>) {
        emit(&mut self.writer, &Config::default(), source_files, diag).expect("emit must not fail")
    }
}
