// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! This module manages validation of the unit tests, in addition to standard compiler
//! checking.

use codespan_reporting::term::{emit, termcolor, termcolor::StandardStream, Config};
use move_model::model::GlobalEnv;
use once_cell::sync::Lazy;
use std::sync::Mutex;

static VALIDATION_HOOK: Lazy<Mutex<Option<Box<dyn Fn(&GlobalEnv) + Send + Sync>>>> =
    Lazy::new(|| Mutex::new(None));

/// Sets a hook which is called to validate the tested modules. The hook gets
/// passed the model containing the unit tests. Any errors during validation
/// should be attached to the model.
pub fn set_validation_hook(p: Box<dyn Fn(&GlobalEnv) + Send + Sync>) {
    *VALIDATION_HOOK.lock().unwrap() = Some(p)
}

/// Returns true if validation is needed. This should be called to avoid building
/// a model unless needed.
pub fn needs_validation() -> bool {
    VALIDATION_HOOK.lock().unwrap().is_some()
}

/// Validates the modules in the env.
pub(crate) fn validate(env: &GlobalEnv) {
    if let Some(h) = &*VALIDATION_HOOK.lock().unwrap() {
        (*h)(env)
    }
}

pub fn has_errors_then_report(model: &GlobalEnv) -> bool {
    let mut has_errors = false;
    let mut writer = StandardStream::stderr(termcolor::ColorChoice::Auto);
    model.report_diag_with_filter(
        |files, diag| {
            emit(&mut writer, &Config::default(), files, diag).expect("emit must not fail")
        },
        |d| {
            let include = d.labels.iter().all(|l| {
                let fname = model.get_file(l.file_id).to_string_lossy();
                !fname.contains("velor-framework/sources")
                    && !fname.contains("velor-stdlib/sources")
            });
            if include && d.severity == codespan_reporting::diagnostic::Severity::Error {
                has_errors = true;
            }
            include
        },
    );
    has_errors
}
