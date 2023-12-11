
use codespan::{FileId, Files};
use codespan_reporting::diagnostic::Label;
use codespan_reporting::term::{Config, emit};
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use codespan_reporting::diagnostic::Diagnostic;


pub struct VisitorContext{
    pub files: Files<String>,
    pub diagnostics: Vec<Diagnostic<FileId>>,
}
impl VisitorContext {
    pub fn new(files: Files<String>) -> Self {
        Self {
            diagnostics: Vec::new(),
            files,
        }
    }


    pub fn add_diagnostic(&mut self, file_id: FileId, start: usize, end: usize, message: &str, severity: codespan_reporting::diagnostic::Severity) {
        let label = Label::primary(file_id, start..end)
            .with_message(message.to_string());
        
        let diagnostic = Diagnostic::new(severity)
            .with_message(message)
            .with_labels(vec![label]);

        self.diagnostics.push(diagnostic);
    }

    pub fn emit_diagnostics(&self) {
        let writer = StandardStream::stderr(ColorChoice::Always);
        let config = Config::default();

        for diagnostic in &self.diagnostics {
            let _ = emit(
                &mut writer.lock(),
                &config,
                &self.files,
                &diagnostic,
            ).expect("emit must not fail");
        }
    }
}