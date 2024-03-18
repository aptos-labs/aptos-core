use anyhow::Ok;
use codespan::{FileId, Span};
use codespan_reporting::{
    diagnostic::{Diagnostic, Label},
    term::{
        emit,
        termcolor::{ColorChoice, StandardStream},
        Config,
    },
};
use move_model::{
    ast::{ExpData, Operation},
    model::{FunctionEnv, GlobalEnv, Parameter},
};
use serde::{Deserialize, Serialize};
use std::{fs::OpenOptions, io::Read, path::Path};
use toml;

// LintConfig is a struct that holds the default configuration for the linter.
#[derive(Deserialize, Serialize, Debug)]
pub struct LintConfig {
    pub statement_count: usize,
    pub usage_frequency: usize,
}

impl Default for LintConfig {
    fn default() -> Self {
        LintConfig {
            statement_count: 10,
            usage_frequency: 2,
        }
    }
}

// Get the variable name or function name from a given expression. This will be used to
// print out the message for this lint.
pub fn get_var_name_or_func_name_from_exp(
    exp: &ExpData,
    func_env: &FunctionEnv,
    env: &GlobalEnv,
) -> Option<String> {
    match exp {
        ExpData::Temporary(_, index) => {
            let parameters = func_env.get_parameters();
            let param = get_var_info_from_func_param(*index, &parameters)
                .expect("variable information not found");
            Some(env.symbol_pool().string(param.0).to_string())
        },
        ExpData::LocalVar(_, sym) => Some(env.symbol_pool().string(*sym).to_string()),
        ExpData::Call(_, Operation::MoveFunction(module_id, func_id), _) => {
            let module = env.get_module(*module_id);
            let func_env = module.get_function(*func_id);
            let func_name = func_env
                .get_name()
                .display(func_env.symbol_pool())
                .to_string();
            Some(func_name)
        },
        ExpData::Call(_, Operation::Not, vec_exp) => {
            if vec_exp.len() == 1 {
                let var_name =
                    get_var_name_or_func_name_from_exp(vec_exp.get(0).unwrap(), func_env, env);
                if let Some(name) = var_name {
                    return Some(format!("!{}", name));
                } else {
                    Some("".to_string())
                }
            } else {
                Some("".to_string())
            }
        },
        _ => Some("".to_string()),
    }
}

pub fn add_diagnostic_and_emit(
    loc: &move_model::model::Loc,
    message: &str,
    severity: codespan_reporting::diagnostic::Severity,
    env: &GlobalEnv,
    diags: &mut Vec<Diagnostic<FileId>>,
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

    diags.push(diagnostic.clone());
    emit(
        &mut writer.lock(),
        &config,
        &env.get_source_files(),
        &diagnostic,
    )
    .expect("emit must not fail");
}

pub fn add_diagnostic_and_emit_by_span(
    start: Span,
    end: Span,
    file_id: FileId,
    message: &str,
    severity: codespan_reporting::diagnostic::Severity,
    env: &GlobalEnv,
    diags: &mut Vec<Diagnostic<FileId>>,
) {
    let writer = StandardStream::stderr(ColorChoice::Always);
    let config = Config::default();
    let label = Label::primary(file_id, start.start().to_usize()..end.end().to_usize())
        .with_message(message.to_string());

    let diagnostic = Diagnostic::new(severity)
        .with_message(message)
        .with_labels(vec![label]);
    diags.push(diagnostic.clone());

    emit(
        &mut writer.lock(),
        &config,
        &env.get_source_files(),
        &diagnostic,
    )
    .expect("emit must not fail");
}

pub fn get_var_info_from_func_param(index: usize, params: &[Parameter]) -> Option<&Parameter> {
    params.get(index)
}

pub fn read_config_or_default(path: &Path) -> Result<LintConfig, anyhow::Error> {
    let binding = path.join("lint.toml");
    let exist_path = Path::new(&binding);
    if exist_path.exists() {
        let mut file = OpenOptions::new().read(true).open(exist_path)?;

        let mut content = String::new();
        file.read_to_string(&mut content)?;

        if content.is_empty() {
            Ok(LintConfig::default())
        } else {
            let config: LintConfig = toml::from_str(&content)?;
            Ok(config)
        }
    } else {
        Ok(LintConfig::default())
    }
}
