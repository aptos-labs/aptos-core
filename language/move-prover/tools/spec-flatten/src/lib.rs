// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;

mod flatten;
mod options;
mod workflow;

pub use options::FlattenOptions;

//**************************************************************************************************
// Entrypoint
//**************************************************************************************************

pub fn run(options: &FlattenOptions) -> Result<()> {
    let (env, targets) = workflow::prepare(options)?;

    // make sure the original verification works
    workflow::prove(options, &env, &targets)?;

    // collect spec in target modules
    for (fid, variant) in targets.get_funs_and_variants() {
        if !variant.is_verified() {
            // only care for functions that are marked as verified
            continue;
        }

        let fun_env = env.get_function(fid);
        if !fun_env.module_env.is_target() {
            // only run on specs in target module
            continue;
        }
        if !fun_env.has_unknown_callers() {
            // only run on specs for external-facing functions
            continue;
        }

        let target = targets.get_target(&fun_env, &variant);
        flatten::flatten_spec(target, &targets);
    }

    // everything is OK
    Ok(())
}
