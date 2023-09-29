// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! This module manages validation of the unit tests, in addition to standard compiler
//! checking.

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
