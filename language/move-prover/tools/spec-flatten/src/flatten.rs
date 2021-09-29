// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use std::collections::BTreeMap;

use bytecode::{function_target::FunctionTarget, function_target_pipeline::FunctionTargetsHolder};
use move_model::{
    ast::{ConditionKind, Spec},
    model::{FunId, QualifiedId},
};

use crate::{
    options::FlattenOptions,
    workflow::{prepare_with_override, prove},
};

pub(crate) fn flatten_spec<'env>(
    options: &FlattenOptions,
    fun_target: FunctionTarget<'env>,
    _targets: &'env FunctionTargetsHolder,
) -> Result<Spec> {
    let fun_env = fun_target.func_env;
    let mut fun_options = options.clone();
    fun_options.target = Some(fun_env.get_simple_name_string().to_string());

    let spec = fun_target.get_spec();
    let new_spec = remove_redundant_aborts_ifs_since(
        &fun_options,
        fun_env.get_qualified_id(),
        spec.clone(),
        0,
    )?;

    if fun_options.verbose {
        println!("fun {}", fun_env.get_full_name_str());
        println!(
            "  Number of aborts_if trimmed: {} (out of {})",
            spec.conditions.len() - new_spec.conditions.len(),
            spec.conditions.len(),
        );
    }
    Ok(new_spec)
}

fn remove_first_aborts_if_since(spec: &Spec, pos: usize) -> (Spec, bool) {
    let Spec {
        loc,
        conditions,
        properties,
        on_impl,
    } = spec.clone();

    let mut new_conditions = vec![];
    let mut changed = false;
    for (i, cond) in conditions.into_iter().enumerate() {
        if changed || i < pos || !matches!(cond.kind, ConditionKind::AbortsIf) {
            new_conditions.push(cond);
        } else {
            changed = true;
        }
    }

    let new_spec = Spec {
        loc,
        conditions: new_conditions,
        properties,
        on_impl,
    };
    (new_spec, changed)
}

fn remove_first_aborts_if_and_prove_since(
    options: &FlattenOptions,
    fun_id: QualifiedId<FunId>,
    spec: &Spec,
    pos: usize,
) -> Result<Option<(Spec, bool)>> {
    let (new_spec, changed) = remove_first_aborts_if_since(spec, pos);
    if !changed {
        return Ok(None);
    }

    let mut spec_override = BTreeMap::new();
    spec_override.insert(fun_id, new_spec.clone());
    let (env, targets) = prepare_with_override(options, spec_override)?;
    let proved = prove(options, &env, &targets)?;
    Ok(Some((new_spec, proved)))
}

fn remove_redundant_aborts_ifs_since(
    options: &FlattenOptions,
    fun_id: QualifiedId<FunId>,
    spec: Spec,
    pos: usize,
) -> Result<Spec> {
    match remove_first_aborts_if_and_prove_since(options, fun_id, &spec, pos)? {
        None => {
            // no more aborts_if conditions to remove
            Ok(spec)
        }
        Some((new_spec, true)) => {
            // removing one aborts_if does not affect the proving
            remove_redundant_aborts_ifs_since(options, fun_id, new_spec, pos)
        }
        Some((_, false)) => {
            // removing one aborts_if makes the proving failed
            remove_redundant_aborts_ifs_since(options, fun_id, spec, pos + 1)
        }
    }
}
