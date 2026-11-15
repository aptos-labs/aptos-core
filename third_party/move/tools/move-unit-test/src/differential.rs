// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Compares two backends' raw outcomes for one test. Verdicts are not
//! compared: a broad `#[expected_failure]` accepts two different failures.

use crate::{
    backend::{TestEffects, TestEvent, TestExecution, TestRun},
    test_runner::inexact_status_reason,
};
use bytes::Bytes;
use move_binary_format::errors::VMError;
use move_core_types::{
    account_address::AccountAddress,
    effects::{ChangeSet, Op},
    language_storage::StructTag,
    vm_status::StatusCode,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
};

pub(crate) enum Comparison {
    Agree,
    /// Agreement cannot be checked, for the given reason. For example:
    /// - a backend ran out of gas, as the backends' gas budgets differ;
    /// - a backend could not record its effects;
    /// - both failed, and a failure only approximates V1's status or location;
    /// - the reference gave different results on two runs.
    Incomparable(String),
    /// Renders each difference.
    Mismatch(String),
}

/// Compares return values and effects on success, or failure details on error.
/// Messages, gas, elapsed time, and failed runs' effects are not compared.
///
/// On a mismatch, reruns the reference. If its results differ between runs,
/// compares only success versus failure and failure details. Matching outcomes
/// remain incomparable because return values and effects could not be checked.
pub(crate) fn compare(
    (reference_name, reference): (&str, &TestRun),
    (candidate_name, candidate): (&str, &TestRun),
    genesis: &ChangeSet,
    rerun_reference: impl FnOnce() -> TestExecution,
) -> Comparison {
    let comparison = compare_runs(
        (reference_name, reference),
        (candidate_name, candidate),
        genesis,
        Scope::Everything,
    );
    if !matches!(comparison, Comparison::Mismatch(_)) {
        return comparison;
    }
    let TestExecution::Ran(rerun) = rerun_reference() else {
        return comparison;
    };
    let reproduced = compare_runs(
        (reference_name, reference),
        (reference_name, &rerun),
        genesis,
        Scope::Everything,
    );
    // Only a difference between the two runs shows nondeterminism.
    if !matches!(reproduced, Comparison::Mismatch(_)) {
        return comparison;
    }
    match compare_runs(
        (reference_name, reference),
        (candidate_name, candidate),
        genesis,
        Scope::Outcome,
    ) {
        Comparison::Agree => {
            Comparison::Incomparable(format!("{reference_name} is nondeterministic on this test"))
        },
        comparison => comparison,
    }
}

/// What [`compare_runs`] compares.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    Everything,
    /// Success versus failure, and the failures.
    Outcome,
}

fn compare_runs(
    (reference_name, reference): (&str, &TestRun),
    (candidate_name, candidate): (&str, &TestRun),
    genesis: &ChangeSet,
    scope: Scope,
) -> Comparison {
    // Gas exhaustion is not comparable across backends with different budgets
    // and gas units.
    for (name, run) in [(reference_name, reference), (candidate_name, candidate)] {
        if matches!(&run.result, Err(err) if err.major_status() == StatusCode::OUT_OF_GAS) {
            return Comparison::Incomparable(format!(
                "{name} ran out of gas, and the backends' gas budgets differ"
            ));
        }
    }
    let mut diff = Diff {
        reference_name,
        candidate_name,
        lines: vec![],
    };
    match (&reference.result, &candidate.result) {
        (Ok(_), Ok(_)) if scope == Scope::Outcome => {},
        (Ok(reference_values), Ok(candidate_values)) => {
            diff.check("return values", reference_values, candidate_values);
            match (&reference.effects, &candidate.effects) {
                (Some(reference_effects), Some(candidate_effects)) => {
                    diff.check_maps(
                        "resource",
                        &live_resources(&reference_effects.resources, genesis),
                        &live_resources(&candidate_effects.resources, genesis),
                    );
                    diff.check_maps(
                        "table item",
                        &table_items(reference_effects),
                        &table_items(candidate_effects),
                    );
                    diff.check_maps(
                        "event",
                        &events(reference_effects),
                        &events(candidate_effects),
                    );
                },
                // Differential runs request effects from both backends, so
                // `None` means the backend could not record them.
                (None, _) | (_, None) => {
                    if diff.lines.is_empty() {
                        let name = if reference.effects.is_none() {
                            reference_name
                        } else {
                            candidate_name
                        };
                        return Comparison::Incomparable(format!(
                            "{name} could not record the test's effects"
                        ));
                    }
                },
            }
        },
        (Ok(_), Err(err)) => diff.push("outcome", "success".to_string(), render_error(err)),
        (Err(err), Ok(_)) => diff.push("outcome", render_error(err), "success".to_string()),
        (Err(reference_err), Err(candidate_err)) => {
            for (name, run, err) in [
                (reference_name, reference, reference_err),
                (candidate_name, candidate, candidate_err),
            ] {
                if !run.v1_exact_error {
                    return Comparison::Incomparable(format!(
                        "{name}: {}",
                        inexact_status_reason(err)
                    ));
                }
            }
            if !same_failure(reference_err, candidate_err) {
                diff.push(
                    "failure",
                    render_error(reference_err),
                    render_error(candidate_err),
                );
            }
        },
    }
    if diff.lines.is_empty() {
        Comparison::Agree
    } else {
        Comparison::Mismatch(diff.lines.join("\n"))
    }
}

struct Diff<'a> {
    reference_name: &'a str,
    candidate_name: &'a str,
    lines: Vec<String>,
}

impl Diff<'_> {
    fn check<V: PartialEq + Debug>(&mut self, what: &str, reference: &V, candidate: &V) {
        if reference != candidate {
            self.push(what, format!("{reference:?}"), format!("{candidate:?}"));
        }
    }

    fn check_maps<K: Ord + Debug, V: PartialEq + Debug>(
        &mut self,
        what: &str,
        reference: &BTreeMap<K, V>,
        candidate: &BTreeMap<K, V>,
    ) {
        let keys: BTreeSet<&K> = reference.keys().chain(candidate.keys()).collect();
        for key in keys {
            let (reference_value, candidate_value) = (reference.get(key), candidate.get(key));
            if reference_value != candidate_value {
                let render = |value: Option<&V>| {
                    value.map_or("<none>".to_string(), |value| format!("{value:?}"))
                };
                self.push(
                    &format!("{what} {key:?}"),
                    render(reference_value),
                    render(candidate_value),
                );
            }
        }
    }

    fn push(&mut self, what: &str, reference: String, candidate: String) {
        let width = self.reference_name.len().max(self.candidate_name.len()) + 1;
        self.lines.push(format!(
            "{what} differs\n  {:width$} {reference}\n  {:width$} {candidate}",
            format!("{}:", self.reference_name),
            format!("{}:", self.candidate_name),
        ));
    }
}

/// Returns resource operations, excluding modifications that rewrite an
/// unchanged genesis value.
fn live_resources<'a>(
    changes: &'a ChangeSet,
    genesis: &ChangeSet,
) -> BTreeMap<(AccountAddress, &'a StructTag), Op<&'a Bytes>> {
    changes
        .resources()
        .filter(|(address, tag, op)| {
            let Op::Modify(value) = op else {
                return true;
            };
            let base = genesis
                .accounts()
                .get(address)
                .and_then(|account| account.resources().get(*tag));
            !matches!(base, Some(Op::New(base) | Op::Modify(base)) if base == *value)
        })
        .map(|(address, tag, op)| ((address, tag), op))
        .collect()
}

/// Table items keyed by table handle and serialized key.
fn table_items(effects: &TestEffects) -> BTreeMap<(AccountAddress, &Vec<u8>), &Op<Bytes>> {
    effects
        .table_items
        .iter()
        .flat_map(|(handle, items)| items.iter().map(move |(key, op)| ((*handle, key), op)))
        .collect()
}

/// Events keyed by emission order.
fn events(effects: &TestEffects) -> BTreeMap<usize, &TestEvent> {
    effects.events.iter().enumerate().collect()
}

/// Whether the failures agree on status, sub-status, location, and offsets.
///
/// V1 attributes a native abort to the native's definition at offset 0, where
/// no Move abort can be; a candidate may record no instruction for it.
fn same_failure(reference: &VMError, candidate: &VMError) -> bool {
    let native_abort = reference.major_status() == StatusCode::ABORTED
        && matches!(reference.offsets().as_slice(), [(_, 0)])
        && candidate.offsets().is_empty();
    reference.major_status() == candidate.major_status()
        && reference.sub_status() == candidate.sub_status()
        && reference.location() == candidate.location()
        && (native_abort || reference.offsets() == candidate.offsets())
}

/// The parts of a failure that are compared.
fn render_error(err: &VMError) -> String {
    format!(
        "{:?} (sub-status {:?}) at {:?}, offsets {:?}",
        err.major_status(),
        err.sub_status(),
        err.location(),
        err.offsets()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_binary_format::{
        errors::{Location, PartialVMError, VMResult},
        file_format::FunctionDefinitionIndex,
    };
    use move_core_types::{
        identifier::Identifier,
        language_storage::{ModuleId, TypeTag},
    };
    use std::time::Duration;

    fn module() -> ModuleId {
        ModuleId::new(AccountAddress::ONE, Identifier::new("m").unwrap())
    }

    fn abort(code: u64, offset: u16) -> VMError {
        PartialVMError::new(StatusCode::ABORTED)
            .with_sub_status(code)
            .with_message(format!("message {code}"))
            .at_code_offset(FunctionDefinitionIndex(0), offset)
            .finish(Location::Module(module()))
    }

    fn run(result: VMResult<Vec<Vec<u8>>>, effects: Option<TestEffects>) -> TestRun {
        TestRun {
            result,
            v1_exact_error: true,
            effects,
            storage_dump: None,
            gas_used: 0,
            elapsed: Duration::ZERO,
        }
    }

    fn resource(name: &str) -> StructTag {
        StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new("m").unwrap(),
            name: Identifier::new(name).unwrap(),
            type_args: vec![],
        }
    }

    fn effects(resources: &[(&str, Op<Bytes>)], events: &[u8]) -> TestEffects {
        let mut changes = ChangeSet::new();
        for (name, op) in resources {
            changes
                .add_resource_op(AccountAddress::ONE, resource(name), op.clone())
                .unwrap();
        }
        let mut effects = TestEffects::new(changes);
        effects.events = events
            .iter()
            .map(|byte| TestEvent {
                handle: None,
                type_tag: TypeTag::U8,
                data: vec![*byte],
            })
            .collect();
        effects
    }

    fn compare_v1_mono(
        reference: &TestRun,
        candidate: &TestRun,
        genesis: &ChangeSet,
    ) -> Comparison {
        compare(("V1", reference), ("MonoMove", candidate), genesis, || {
            TestExecution::Ran(run(reference.result.clone(), reference.effects.clone()))
        })
    }

    /// Two different failures disagree even though a bare `#[expected_failure]`
    /// accepts each of them.
    #[test]
    fn different_failures_mismatch() {
        let comparison = compare_v1_mono(
            &run(Err(abort(1, 3)), None),
            &run(Err(abort(2, 3)), None),
            &ChangeSet::new(),
        );
        let Comparison::Mismatch(diff) = comparison else {
            panic!("expected a mismatch");
        };
        assert!(
            diff.contains("Some(1)") && diff.contains("Some(2)"),
            "{diff}"
        );
    }

    #[test]
    fn failures_differing_in_offset_mismatch() {
        assert!(matches!(
            compare_v1_mono(
                &run(Err(abort(1, 3)), None),
                &run(Err(abort(1, 4)), None),
                &ChangeSet::new()
            ),
            Comparison::Mismatch(_)
        ));
    }

    /// Messages are not compared.
    #[test]
    fn failures_differing_in_message_agree() {
        let candidate = PartialVMError::new(StatusCode::ABORTED)
            .with_sub_status(1)
            .at_code_offset(FunctionDefinitionIndex(0), 3)
            .finish(Location::Module(module()));
        assert!(matches!(
            compare_v1_mono(
                &run(Err(abort(1, 3)), None),
                &run(Err(candidate), None),
                &ChangeSet::new()
            ),
            Comparison::Agree
        ));
    }

    #[test]
    fn success_and_failure_mismatch() {
        let success = run(Ok(vec![]), Some(effects(&[], &[])));
        assert!(matches!(
            compare_v1_mono(&success, &run(Err(abort(1, 3)), None), &ChangeSet::new()),
            Comparison::Mismatch(_)
        ));
    }

    /// A fallback status cannot be compared with another failure, but still
    /// disagrees with a success.
    #[test]
    fn inexact_failure_is_incomparable_only_against_a_failure() {
        let mut inexact = run(Err(abort(1, 3)), None);
        inexact.v1_exact_error = false;
        assert!(matches!(
            compare_v1_mono(&run(Err(abort(1, 3)), None), &inexact, &ChangeSet::new()),
            Comparison::Incomparable(_)
        ));
        assert!(matches!(
            compare_v1_mono(
                &run(Ok(vec![]), Some(effects(&[], &[]))),
                &inexact,
                &ChangeSet::new()
            ),
            Comparison::Mismatch(_)
        ));
    }

    /// The backends' gas budgets differ, so running out on either side is no
    /// basis for comparison.
    #[test]
    fn out_of_gas_is_incomparable() {
        let out_of_gas = run(
            Err(PartialVMError::new(StatusCode::OUT_OF_GAS).finish(Location::Module(module()))),
            None,
        );
        let success = run(Ok(vec![]), Some(effects(&[], &[])));
        for (reference, candidate) in [(&out_of_gas, &success), (&success, &out_of_gas)] {
            assert!(matches!(
                compare_v1_mono(reference, candidate, &ChangeSet::new()),
                Comparison::Incomparable(_)
            ));
        }
    }

    /// Effects a backend could not record are no basis for comparison, but a
    /// difference found without them stands.
    #[test]
    fn missing_effects_are_incomparable() {
        let unrecorded = run(Ok(vec![vec![1]]), None);
        assert!(matches!(
            compare_v1_mono(
                &unrecorded,
                &run(Ok(vec![vec![1]]), Some(effects(&[], &[]))),
                &ChangeSet::new()
            ),
            Comparison::Incomparable(_)
        ));
        assert!(matches!(
            compare_v1_mono(
                &unrecorded,
                &run(Ok(vec![vec![2]]), Some(effects(&[], &[]))),
                &ChangeSet::new()
            ),
            Comparison::Mismatch(_)
        ));
    }

    #[test]
    fn return_values_are_compared() {
        assert!(matches!(
            compare_v1_mono(
                &run(Ok(vec![vec![1]]), Some(effects(&[], &[]))),
                &run(Ok(vec![vec![2]]), Some(effects(&[], &[]))),
                &ChangeSet::new()
            ),
            Comparison::Mismatch(_)
        ));
    }

    /// A modification that rewrites the genesis value is not a write.
    #[test]
    fn unchanged_genesis_modification_is_pruned() {
        let value = Bytes::from_static(&[7]);
        let genesis = effects(&[("R", Op::New(value.clone()))], &[]).resources;
        let reference = run(Ok(vec![]), Some(effects(&[("R", Op::Modify(value))], &[])));
        let candidate = run(Ok(vec![]), Some(effects(&[], &[])));
        assert!(matches!(
            compare_v1_mono(&reference, &candidate, &genesis),
            Comparison::Agree
        ));

        let changed = run(
            Ok(vec![]),
            Some(effects(&[("R", Op::Modify(Bytes::from_static(&[8])))], &[])),
        );
        assert!(matches!(
            compare_v1_mono(&changed, &candidate, &genesis),
            Comparison::Mismatch(_)
        ));
    }

    #[test]
    fn events_are_compared_in_order() {
        let reference = run(Ok(vec![]), Some(effects(&[], &[1, 2])));
        let candidate = run(Ok(vec![]), Some(effects(&[], &[2, 1])));
        assert!(matches!(
            compare_v1_mono(&reference, &candidate, &ChangeSet::new()),
            Comparison::Mismatch(_)
        ));
    }

    /// V1 reports a native abort at the native's offset 0; a candidate may
    /// report no offset for it.
    #[test]
    fn native_abort_without_offset_agrees() {
        let native_abort = |offset: Option<u16>| {
            let mut error = PartialVMError::new(StatusCode::ABORTED).with_sub_status(1);
            if let Some(offset) = offset {
                error = error.at_code_offset(FunctionDefinitionIndex(5), offset);
            }
            error.finish(Location::Module(module()))
        };
        assert!(matches!(
            compare_v1_mono(
                &run(Err(native_abort(Some(0))), None),
                &run(Err(native_abort(None)), None),
                &ChangeSet::new()
            ),
            Comparison::Agree
        ));
        assert!(matches!(
            compare_v1_mono(
                &run(Err(native_abort(Some(3))), None),
                &run(Err(native_abort(None)), None),
                &ChangeSet::new()
            ),
            Comparison::Mismatch(_)
        ));
    }

    /// Values from a reference that disagrees with itself are not compared,
    /// and the test is not counted as agreeing.
    #[test]
    fn nondeterministic_reference_is_incomparable() {
        let reference = run(Ok(vec![vec![1]]), Some(effects(&[], &[])));
        let candidate = run(Ok(vec![vec![2]]), Some(effects(&[], &[])));
        let rerun = || TestExecution::Ran(run(Ok(vec![vec![3]]), Some(effects(&[], &[]))));
        assert!(matches!(
            compare(
                ("V1", &reference),
                ("MonoMove", &candidate),
                &ChangeSet::new(),
                rerun
            ),
            Comparison::Incomparable(_)
        ));

        // Outcomes are still compared.
        let failed = run(Err(abort(1, 3)), None);
        assert!(matches!(
            compare(
                ("V1", &reference),
                ("MonoMove", &failed),
                &ChangeSet::new(),
                rerun
            ),
            Comparison::Mismatch(_)
        ));
    }
}
