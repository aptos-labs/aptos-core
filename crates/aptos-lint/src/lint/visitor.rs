use codespan_reporting::{
    diagnostic::{Diagnostic, Label},
    term::{
        emit,
        termcolor::{ColorChoice, StandardStream},
        Config,
    },
};
use move_model::model::{FunctionEnv, GlobalEnv, Loc, ModuleEnv, Parameter};

pub trait LintUtilities {
    fn add_diagnostic_and_emit(
        &self,
        loc: &Loc,
        message: &str,
        severity: codespan_reporting::diagnostic::Severity,
        env: &GlobalEnv,
    ) {
        let writer = StandardStream::stderr(ColorChoice::Always);
        let config = Config::default();
        let label = Label::primary(
            loc.file_id(),
            loc.span().start().to_usize()..loc.span().end().to_usize(),
        )
        .with_message(message.to_string());

        let diagnostic = Diagnostic::new(severity)
            .with_message(message)
            .with_labels(vec![label]);
        emit(
            &mut writer.lock(),
            &config,
            &env.get_source_files(),
            &diagnostic,
        )
        .expect("emit must not fail");
    }
}

pub trait ExpDataVisitor {
    fn get_var_info_from_func_param(
        &self,
        index: &usize,
        params: Vec<Parameter>,
    ) -> Option<Parameter> {
        Some(params[*index].clone())
    }
    fn visit(&mut self, _func_env: &FunctionEnv, _env: &GlobalEnv) {}
    fn visit_module(&mut self, _module: &ModuleEnv, _env: &GlobalEnv) {}
}
