use crate::{
    diagnostics::{human::HumanEmitter, json::JsonEmitter},
    options, Experiment,
};
use anyhow::bail;
use codespan::{FileId, Files};
use codespan_reporting::{
    diagnostic::{Diagnostic, Severity},
    term::termcolor::StandardStream,
};
use move_model::model::GlobalEnv;

pub mod human;
pub mod json;

impl options::Options {
    pub fn to_emitter<'w>(&self, stderr: &'w mut StandardStream) -> Box<dyn Emitter + 'w> {
        if self.experiment_on(Experiment::MESSAGE_FORMAT_JSON) {
            Box::new(JsonEmitter::new(stderr))
        } else {
            Box::new(HumanEmitter::new(stderr))
        }
    }
}

pub trait Emitter {
    fn emit(&mut self, source_files: &Files<String>, diag: &Diagnostic<FileId>);

    /// Writes accumulated diagnostics of given or higher severity.
    fn report_diag(&mut self, global_env: &GlobalEnv, severity: Severity) {
        global_env.report_diag_with_filter(
            |files, diag| self.emit(files, diag),
            |d| d.severity >= severity,
        );
    }

    /// Helper function to report diagnostics, check for errors, and fail with a message on
    /// errors. This function is idempotent and will not report the same diagnostics again.
    fn check_diag(
        &mut self,
        global_env: &GlobalEnv,
        report_severity: Severity,
        msg: &str,
    ) -> anyhow::Result<()> {
        self.report_diag(global_env, report_severity);
        if global_env.has_errors() {
            bail!("exiting with {}", msg);
        } else {
            Ok(())
        }
    }
}
