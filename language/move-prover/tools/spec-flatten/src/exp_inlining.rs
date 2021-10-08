// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;

use bytecode::function_target::FunctionTarget;
use move_model::ast::Spec;

use crate::workflow::WorkflowOptions;

pub(crate) fn inline_all_exp_in_spec(
    _options: &WorkflowOptions,
    _target: FunctionTarget,
    spec: Spec,
) -> Result<Spec> {
    Ok(spec)
}
