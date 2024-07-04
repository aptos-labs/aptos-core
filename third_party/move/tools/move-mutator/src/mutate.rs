// Copyright © Eiger
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    cli,
    configuration::{Configuration, IncludeFunctions},
    mutant::Mutant,
    operator::MutationOp,
    operators::{
        binary::Binary, binary_swap::BinarySwap, break_continue::BreakContinue,
        delete_stmt::DeleteStmt, ifelse::IfElse, literal::Literal, unary::Unary, ExpLoc,
    },
};
use move_model::{
    ast::{Exp, ExpData, Operation},
    model::{FunctionEnv, GlobalEnv, ModuleEnv},
};
use move_package::source_package::layout::SourcePackageLayout;
use std::path::Path;

/// Traverses the AST, identifies places where mutation operators can be applied
/// and returns a list of mutants.
pub fn mutate(env: &GlobalEnv, conf: &Configuration) -> anyhow::Result<Vec<Mutant>> {
    trace!("Starting mutation process");
    let mutants = env
        .get_modules()
        .map(|module| traverse_module_with_check(&module, conf))
        .collect::<Result<Vec<_>, _>>()?
        .concat();

    trace!("Found {} possible mutations", mutants.len());

    Ok(mutants)
}

/// Traverses a single module and returns a list of mutants - helper function which filter out modules
/// that are not included in the configuration.
#[inline]
fn traverse_module_with_check(
    module: &ModuleEnv<'_>,
    conf: &Configuration,
) -> anyhow::Result<Vec<Mutant>> {
    let module_name = module.env.symbol_pool().string(module.get_name().name());

    // We need to check if module comes from our source tree or from the deps, as we don't want to traverse
    // all the dependencies. That's a bit tricky as global deps are easy to identify but local deps can be
    // anywhere near the project tree.
    let filename_path = Path::new(module.get_source_path());

    if !conf.project.move_sources.is_empty()
        && !conf
            .project
            .move_sources
            .contains(&filename_path.to_path_buf())
    {
        trace!("Skipping module {module_name} as it does not come from source project");
        return Ok(vec![]);
    }

    if conf.project.move_sources.is_empty() {
        let test_root = SourcePackageLayout::try_find_root(&filename_path.canonicalize()?)?;
        if let Some(project_path) = &conf.project_path {
            let project_path = project_path.canonicalize()?;
            if test_root != project_path {
                trace!(
                    "Skipping module: \n {module_name} \n root: {} \n as it does not come from source project {}",
                    test_root.to_string_lossy(),
                    project_path.to_string_lossy()
                );
                return Ok(vec![]);
            }
        }
    }

    // Now we need to check if the module is included in the configuration.
    if let cli::ModuleFilter::Selected(mods) = &conf.project.mutate_modules {
        if !mods.contains(&module_name) {
            trace!("Skipping module {module_name}");
            return Ok(vec![]);
        }
    }

    traverse_module(module, conf)
}

/// Traverses a single module and returns a list of mutants.
/// Checks all the functions and constants defined in the module.
#[allow(clippy::unnecessary_to_owned)]
fn traverse_module(module: &ModuleEnv<'_>, conf: &Configuration) -> anyhow::Result<Vec<Mutant>> {
    let module_name = module.get_name().display(module.env);

    trace!("Traversing module {}", &module_name);
    let mut mutants = module
        .get_functions()
        .map(|func| traverse_function(&func, conf))
        .collect::<Result<Vec<_>, _>>()?
        .concat();

    // Set the module name for all the mutants.
    mutants
        .iter_mut()
        .for_each(|m| m.set_module_name(module_name.to_string()));

    trace!(
        "Found {} possible mutations in module {}",
        mutants.len(),
        module_name
    );
    Ok(mutants)
}

/// Traverses a single function and returns a list of mutants.
/// Checks the body of the function by traversing its definition.
#[allow(clippy::unnecessary_wraps)]
fn traverse_function(
    function: &FunctionEnv<'_>,
    conf: &Configuration,
) -> anyhow::Result<Vec<Mutant>> {
    let attrs = function.get_attributes();
    for attr in attrs {
        // Omit all functions with test attribute.
        if attr
            .name()
            .display(function.module_env.symbol_pool())
            .to_string()
            .contains("test")
        {
            trace!("Skipping test function {}", &function.get_name_str());
            return Ok(vec![]);
        }
    }

    let mut is_inside_spec = false;

    let function_name = function.get_name_str();
    let filename = function.module_env.get_source_path();

    // Check if function is included in individual configuration.
    if let Some(ind) = conf.get_file_configuration(Path::new(filename)) {
        if let IncludeFunctions::Selected(funcs) = &ind.include_functions {
            if !funcs.contains(&function_name) {
                trace!("Skipping function {}", &function_name);
                return Ok(vec![]);
            }
        }
    }

    trace!("Traversing function {}", &function_name);
    let mut result = Vec::<Mutant>::new();
    if let Some(exp) = function.get_def() {
        exp.visit_pre_post(&mut |asc, exp_data| {
            // Collect the spec blocks locations.
            if let ExpData::SpecBlock(_, _) = exp_data {
                // Mark that we are inside of the spec block when going desc - and remove that when going asc.
                is_inside_spec = !asc;
            }

            // Parse only during the descend phase and when we are not inside the spec block.
            if !asc && !is_inside_spec {
                result.extend(parse_expression_and_find_mutants(function, exp_data));
            }

            true
        });
    };

    result
        .iter_mut()
        .for_each(|m| m.set_function_name(function_name.clone()));

    Ok(result)
}

/// This function does the actual parsing of the expression and checks if any of the mutation operators
/// can be applied to it.
/// When Move language is extended with new expressions, this function needs to be updated to support them.
#[allow(clippy::too_many_lines)]
fn parse_expression_and_find_mutants(function: &FunctionEnv<'_>, exp: &ExpData) -> Vec<Mutant> {
    let convert_exps_to_explocs = |exps: &[Exp]| -> Vec<ExpLoc> {
        exps.iter()
            .map(|e| ExpLoc {
                exp: e.clone(),
                loc: function.module_env.env.get_node_loc(e.node_id()),
            })
            .collect::<Vec<ExpLoc>>()
    };

    trace!("Parsing expression {exp:?}");
    match exp {
        ExpData::Call(node_id, op, exps) => match op {
            Operation::MoveTo | Operation::Abort => {
                vec![Mutant::new(MutationOp::new(Box::new(DeleteStmt::new(
                    exp.clone().into_exp(),
                    function.module_env.env.get_node_loc(*node_id),
                ))))]
            },
            Operation::Add
            | Operation::Sub
            | Operation::Mul
            | Operation::Div
            | Operation::Mod
            | Operation::And
            | Operation::Or
            | Operation::Eq
            | Operation::Neq
            | Operation::Ge
            | Operation::Gt
            | Operation::Le
            | Operation::Lt
            | Operation::BitAnd
            | Operation::BitOr
            | Operation::Shl
            | Operation::Shr
            | Operation::Xor => {
                let exps_loc = convert_exps_to_explocs(exps);
                let mut result = vec![Mutant::new(MutationOp::new(Box::new(Binary::new(
                    op.clone(),
                    function.module_env.env.get_node_loc(*node_id),
                    exps_loc.clone(),
                ))))];

                result.push(Mutant::new(MutationOp::new(Box::new(BinarySwap::new(
                    op.clone(),
                    function.module_env.env.get_node_loc(*node_id),
                    exps_loc,
                )))));

                result
            },
            Operation::Not => {
                let exps_loc = convert_exps_to_explocs(exps);
                vec![Mutant::new(MutationOp::new(Box::new(Unary::new(
                    op.clone(),
                    function.module_env.env.get_node_loc(*node_id),
                    exps_loc,
                ))))]
            },
            _ => vec![],
        },
        ExpData::IfElse(_, cond, if_exp, else_exp) => {
            let cond_loc = ExpLoc {
                exp: cond.clone(),
                loc: function.module_env.env.get_node_loc(cond.node_id()),
            };
            let if_exp_loc = ExpLoc {
                exp: if_exp.clone(),
                loc: function.module_env.env.get_node_loc(if_exp.node_id()),
            };
            let else_exp_loc = ExpLoc {
                exp: else_exp.clone(),
                loc: function.module_env.env.get_node_loc(else_exp.node_id()),
            };
            vec![Mutant::new(MutationOp::new(Box::new(IfElse::new(
                cond_loc,
                if_exp_loc,
                else_exp_loc,
            ))))]
        },
        ExpData::Value(node_id, value) => {
            let mutants = vec![Mutant::new(MutationOp::new(Box::new(Literal::new(
                value.clone(),
                function.module_env.env.get_node_type(*node_id),
                function.module_env.env.get_node_loc(*node_id),
            ))))];
            mutants
        },
        ExpData::LoopCont(node_id, _) => vec![Mutant::new(MutationOp::new(Box::new(
            BreakContinue::new(function.module_env.env.get_node_loc(*node_id)),
        )))],

        ExpData::Return(_, _)
        | ExpData::Mutate(_, _, _)
        | ExpData::Assign(_, _, _)
        | ExpData::Block(_, _, _, _)
        | ExpData::Invoke(_, _, _)
        | ExpData::Lambda(_, _, _)
        | ExpData::LocalVar(_, _)
        | ExpData::Loop(_, _)
        | ExpData::Temporary(_, _)
        | ExpData::SpecBlock(_, _)
        | ExpData::Sequence(_, _)
        | ExpData::Quant(_, _, _, _, _, _)
        | ExpData::Match(_, _, _)
        | ExpData::Invalid(_) => vec![],
    }
}
