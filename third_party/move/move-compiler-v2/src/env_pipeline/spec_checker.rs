// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! The spec checker runs over the specifications of the target modules:
//!
//! - It checks whether the constructs they use are pure. If a specification
//!   expression calls a Move function it checks that for pureness as well.
//! - It checks whether struct invariants do not depend on global state.

use crate::env_pipeline::rewrite_target::{
    RewriteState, RewriteTarget, RewriteTargets, RewritingScope,
};
use codespan_reporting::diagnostic::Severity;
use log::debug;
use move_model::{
    ast::{Exp, Spec},
    model::{FunId, GlobalEnv, NodeId, QualifiedId},
    pureness_checker::{FunctionPurenessChecker, FunctionPurenessCheckerMode},
};

pub fn run_spec_checker(env: &GlobalEnv) {
    debug!("checking specifications");

    // Targets are all spec functions and spec blocks, as well as functions to
    // process inline specs.
    let mut targets = RewriteTargets::create(env, RewritingScope::CompilationTarget);
    targets.filter(|target, _| {
        matches!(
            target,
            RewriteTarget::SpecFun(_) | RewriteTarget::SpecBlock(_) | RewriteTarget::MoveFun(_)
        )
    });

    // Walk over those targets and check them for pureness.
    for target in targets.keys() {
        match (target.clone(), target.get_env_state(env)) {
            (RewriteTarget::MoveFun(_), RewriteState::Def(exp)) => {
                exp.visit_inline_specs(&mut |s| {
                    check_spec(env, s);
                    true
                })
            },
            (RewriteTarget::SpecFun(_), RewriteState::Def(exp)) => check_exp(env, &exp),
            (RewriteTarget::SpecBlock(_), RewriteState::Spec(spec)) => check_spec(env, &spec),
            _ => {},
        }
    }
}

fn check_exp(env: &GlobalEnv, exp: &Exp) {
    let mut error_reported = false;
    let mut checker = FunctionPurenessChecker::new(
        FunctionPurenessCheckerMode::Specification,
        |node_id, msg, call_chain| report_error(env, &mut error_reported, node_id, msg, call_chain),
    );
    checker.check_exp(env, exp);
}

fn check_spec(env: &GlobalEnv, spec: &Spec) {
    let mut error_reported = false;
    let mut checker = FunctionPurenessChecker::new(
        FunctionPurenessCheckerMode::Specification,
        |node_id, msg, call_chain| report_error(env, &mut error_reported, node_id, msg, call_chain),
    );
    checker.check_spec(env, spec);
}

fn report_error(
    env: &GlobalEnv,
    error_reported: &mut bool,
    id: NodeId,
    msg: &str,
    call_chain: &[(QualifiedId<FunId>, NodeId)],
) {
    // We report the first error only because otherwise the error messages can be
    // overwhelming, if the user e.g. accidentally calls a complex system function.
    if *error_reported {
        return;
    }
    // The first call in call_chain is the one from the specification function to
    // a Move function. We take this as the primary anchor for the error message
    let print_fun = |f: QualifiedId<FunId>| env.get_function(f).get_name_str();
    if call_chain.is_empty() {
        // Direct report
        env.diag_with_primary_and_labels(
            Severity::Error,
            &env.get_node_loc(id),
            "specification expression cannot use impure construct",
            msg,
            vec![],
        );
    } else {
        let (first_fun, first_id) = call_chain[0];
        let mut call_chain_info = vec![];
        // First print the sequence of calls leading us to the issue
        for i in 1..call_chain.len() {
            let previous_fun = print_fun(call_chain[i - 1].0);
            let this_fun = print_fun(call_chain[1].0);
            let this_loc = env.get_node_loc(call_chain[1].1);
            call_chain_info.push((
                this_loc,
                format!(
                    "transitively calling `{}` from `{}` here",
                    this_fun, previous_fun
                ),
            ))
        }
        // Next print the particular issue detected
        let last_fun = call_chain.last().unwrap().0;
        call_chain_info.push((
            env.get_node_loc(id),
            format!("in `{}`: {}", print_fun(last_fun), msg),
        ));

        env.diag_with_primary_and_labels(
            Severity::Error,
            &env.get_node_loc(first_id),
            &format!(
                "specification expression cannot call impure \
            Move function `{}`",
                env.get_function(first_fun).get_name_str()
            ),
            "called here",
            call_chain_info,
        );
    }
    *error_reported = true;
}
