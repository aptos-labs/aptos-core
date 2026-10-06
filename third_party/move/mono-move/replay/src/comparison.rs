// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Compares MonoMove (V2) against the legacy AptosVM (V1) on the records of a corpus.
//!
//! Per record: the overrides patch the recorded state, V1 runs on it, and V1's status must equal
//! the on-chain one — otherwise the patch changed what the transaction does, and the record is
//! not a candidate. MonoMove then runs on the same patched state and its output is compared with
//! V1's. V1 on the patched state is the reference; the chain only filters candidates, since gas
//! zeroing and metadata stripping make the replayed write set differ from the committed one.

use crate::{
    compare::{classify, diff_outputs, DiffField, Verdict},
    corpus::{Corpus, Shard, TxnRecord},
    isolated::{self, ReplayInput, V2Limits, V2Outcome},
    overrides::{OverrideConfig, OverrideCounts, PatchedState},
    panic_message,
    txn::{is_replayed, label, txn_kind},
    v1::{self, hit_metering_limit_on_chain},
    v2::Unsupported,
};
use aptos_types::{
    state_store::{state_key::StateKey, TStateView},
    transaction::{TransactionStatus, Version},
};
use mono_move_output::gap::GapKind;
use move_core_types::vm_status::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// How many of the unobserved keys a crashed MonoMove read [`Outcome::MonoCrash`] lists.
pub const CRASH_UNOBSERVED_SAMPLE: usize = 20;

/// What comparing one record found.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Outcome {
    /// MonoMove's output equals V1's, byte for byte.
    Match,
    /// MonoMove's output differs from V1's only in ways MonoMove is known to: see
    /// [`crate::compare::DiffField::is_known`]. Not a test failure, and not a match.
    KnownDifference { diffs: Vec<String> },
    /// MonoMove's output differs from V1's, or MonoMove failed where it should not: a test
    /// failure. Lists every difference, known ones included.
    Mismatch { diffs: Vec<String> },
    /// V1's status on the patched state differs from the chain's, so the patch changed observable
    /// behavior. A fact about the patch, not about MonoMove.
    NotCandidate { onchain: String, v1: String },
    /// The transaction stopped at a metering limit (gas, memory, dependencies), on chain or in V1:
    /// MonoMove runs unmetered and does not enforce it, so the two are not comparable on this
    /// record. A stop on chain is not replayed at all (`v1_status` then says "on chain").
    NotComparable { v1_status: String },
    /// A VM read keys the corpus holds no observation of (neither captured nor recorded absent),
    /// so its run rested on a guess and nothing about it is concluded.
    Incomplete { vm: String, unobserved: Vec<String> },
    /// MonoMove reported that it does not support something it reached. A statement of what it
    /// reported, not a verdict on the output; V1's status shows whether the transaction got there
    /// on V1 too.
    MonoUnsupported {
        kind: UnsupportedKind,
        v1_status: String,
    },
    /// MonoMove's process exceeded its memory or time limit, or died: a test failure, which in
    /// process would have ended the whole run. A crash is never discounted as incomplete, since a
    /// runaway MonoMove can read keys V1 never does; `unobserved` lists (up to
    /// [`CRASH_UNOBSERVED_SAMPLE`] of) the unobserved keys it read before crashing, for triage.
    MonoCrash {
        reason: String,
        v1_status: String,
        unobserved: Vec<String>,
        /// How many unobserved keys it read, of which `unobserved` lists the first.
        unobserved_total: usize,
    },
    /// V1 itself could not run the record.
    V1Error { detail: String },
    /// A transaction kind the comparison does not replay.
    Skipped { reason: String },
}

/// A MonoMove gap, as reported. Ranking these by frequency over a corpus says which gaps matter
/// most on real traffic.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UnsupportedKind {
    MissingNative { module: String, function: String },
    LoweringSkipped { reason: String },
    ResourceLayoutNotDerivable,
    RuntimeUnsupported { what: String },
    TransactionShape { what: String },
    Other { detail: String },
}

impl From<Unsupported> for UnsupportedKind {
    fn from(unsupported: Unsupported) -> Self {
        match unsupported {
            Unsupported::Transaction(what) => UnsupportedKind::TransactionShape {
                what: what.to_string(),
            },
            Unsupported::Vm(gap) | Unsupported::Output { gap, .. } => match gap.kind {
                GapKind::MissingNative {
                    address,
                    module,
                    function,
                } => UnsupportedKind::MissingNative {
                    module: format!("{}::{}", address.short_str_lossless(), module),
                    function,
                },
                GapKind::LoweringSkipped { reason } => UnsupportedKind::LoweringSkipped {
                    reason: reason.to_string(),
                },
                GapKind::ResourceLayoutNotDerivable => UnsupportedKind::ResourceLayoutNotDerivable,
                GapKind::RuntimeUnsupported { what } => UnsupportedKind::RuntimeUnsupported {
                    what: what.to_string(),
                },
                GapKind::Other => UnsupportedKind::Other {
                    detail: gap.message,
                },
            },
        }
    }
}

/// One line of the comparison output.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecordResult {
    /// The corpus the record is from; versions are unique only within one.
    pub corpus_id: String,
    pub version: Version,
    /// What the transaction runs, e.g. its entry function.
    pub label: String,
    /// The overrides that changed this record's state.
    pub overrides: Vec<String>,
    /// Whether the on-chain status was available to filter candidates.
    pub onchain_checked: bool,
    #[serde(flatten)]
    pub outcome: Outcome,
}

/// What every replay of the record runs on (its state, with the corpus's overrides), and the
/// patched state it makes.
pub(crate) fn prepare(
    corpus: &Corpus,
    shard: &Shard,
    record: &TxnRecord,
) -> anyhow::Result<(ReplayInput, PatchedState)> {
    let input = ReplayInput {
        values: corpus.record_state(shard, record)?.into_iter().collect(),
        absent: record.absent.iter().cloned().collect(),
        config: OverrideConfig::for_origin(&corpus.manifest().origin),
        txn: record.txn.clone(),
        aux_info: record.aux_info,
    };
    let state = input.patched()?;
    Ok((input, state))
}

pub fn compare_record(
    corpus: &Corpus,
    shard: &Shard,
    record: &TxnRecord,
    limits: &V2Limits,
) -> anyhow::Result<RecordResult> {
    let mut result = RecordResult {
        corpus_id: corpus.manifest().corpus_id.clone(),
        version: record.version,
        label: label(&record.txn),
        overrides: vec![],
        onchain_checked: record.onchain.is_some(),
        outcome: Outcome::Match,
    };
    let (input, state) = match prepare(corpus, shard, record) {
        Ok(prepared) => prepared,
        Err(err) => {
            result.outcome = Outcome::V1Error {
                detail: format!("failed to prepare the state: {:#}", err),
            };
            return Ok(result);
        },
    };
    result.overrides = state
        .report()
        .fired
        .iter()
        .map(|name| name.to_string())
        .collect();
    result.outcome = judge(&input, &state, record, limits)?;
    Ok(result)
}

/// The verdict on `record`, running MonoMove isolated. `Err` only when the MonoMove harness fails,
/// which says nothing about MonoMove and so is not a verdict.
fn judge(
    input: &ReplayInput,
    state: &PatchedState,
    record: &TxnRecord,
    limits: &V2Limits,
) -> anyhow::Result<Outcome> {
    if !is_replayed(&record.txn) {
        return Ok(Outcome::Skipped {
            reason: format!("{} transactions are not replayed", txn_kind(&record.txn)),
        });
    }
    // A transaction that hit a metering limit on chain is not run: with gas zeroed, a loop that gas
    // bounded on chain would never end, and V1 cannot reproduce the chain's status anyway.
    if let Some(onchain) = &record.onchain
        && hit_metering_limit_on_chain(&onchain.status)
    {
        return Ok(Outcome::NotComparable {
            v1_status: format!(
                "{:?} (on chain)",
                TransactionStatus::Keep(onchain.status.clone())
            ),
        });
    }
    let read_failed = |err: anyhow::Error| Outcome::V1Error {
        detail: format!("failed to read the patched state: {:#}", err),
    };
    // `Err` with the verdict if `vm` read keys the corpus holds no observation of.
    let all_observed = |vm: &str, reads: BTreeSet<StateKey>| -> Result<(), Outcome> {
        let keys = state.unobserved(&reads).map_err(read_failed)?;
        if keys.is_empty() {
            return Ok(());
        }
        Err(Outcome::Incomplete {
            vm: vm.to_string(),
            unobserved: keys.iter().map(|key| format!("{:?}", key)).collect(),
        })
    };

    let view = state.view();
    let v1 = v1::run_caught(&view, &record.txn, record.aux_info);
    // V1 on unobserved state is not V1 on chain state: neither its errors, the candidate filter
    // nor the comparison can trust it.
    if let Err(outcome) = all_observed("v1", view.into_reads()) {
        return Ok(outcome);
    }
    let v1 = match v1 {
        Ok(Ok(run)) => run,
        Ok(Err(err)) => {
            return Ok(Outcome::V1Error {
                detail: format!("{:#}", err),
            })
        },
        Err(panic) => {
            return Ok(Outcome::V1Error {
                detail: format!("panicked: {}", panic_message(&panic)),
            })
        },
    };
    let (v1, v1_hit_metering_limit, v1_vm_status) =
        (v1.output, v1.hit_metering_limit, v1.vm_status);

    if let Some(onchain) = &record.onchain {
        let onchain_status = TransactionStatus::Keep(onchain.status.clone());
        if v1.status() != &onchain_status {
            return Ok(Outcome::NotCandidate {
                onchain: format!("{:?}", onchain_status),
                v1: format!("{:?}", v1.status()),
            });
        }
    }

    if v1_hit_metering_limit {
        return Ok(Outcome::NotComparable {
            v1_status: format!("{:?}", v1.status()),
        });
    }

    // In a child process, so that MonoMove exhausting memory or never finishing is a verdict
    // rather than the end of the run.
    let v2 = isolated::run_v2(input, limits)?;
    // A crash is judged first: it is never discounted as incomplete. Anything else V2 did is judged
    // only once its reads are known observed.
    let run = match v2.outcome {
        V2Outcome::Crashed(reason) => {
            // A runaway crash can have read a million keys: only a sample is collected and formatted.
            let (sample, total) = match state.unobserved_sample(&v2.reads, CRASH_UNOBSERVED_SAMPLE)
            {
                Ok(found) => found,
                Err(err) => return Ok(read_failed(err)),
            };
            return Ok(Outcome::MonoCrash {
                reason,
                v1_status: format!("{:?}", v1.status()),
                unobserved: sample.iter().map(|key| format!("{:?}", key)).collect(),
                unobserved_total: total,
            });
        },
        V2Outcome::Ran(run) => Ok(run),
        V2Outcome::SetupFailed(err) => Err(Outcome::Mismatch {
            diffs: vec![format!("V2 could not be set up: {}", err)],
        }),
        V2Outcome::Panicked(message) => Err(Outcome::Mismatch {
            diffs: vec![format!("V2 panicked: {}", message)],
        }),
    };
    // Checked before anything else V2 did is judged, a panic or a gap included: on unobserved
    // state, V2 may have failed or diverged only because the corpus guessed a key absent.
    if let Err(outcome) = all_observed("v2", v2.reads) {
        return Ok(outcome);
    }
    let v2 = match run {
        Ok(run) => run,
        Err(outcome) => return Ok(outcome),
    };
    if let Some(kind) = v2.unsupported {
        // A gap hit while writing the output comes after execution, whose status is final: one
        // that differs from V1's is a divergence the gap must not hide.
        if let Some(v2_status) = &v2.output_gap_status
            && v2_status != v1.status()
        {
            return Ok(Outcome::Mismatch {
                diffs: vec![DiffField::Status {
                    v1: v1.status().clone(),
                    v2: v2_status.clone(),
                }
                .describe()],
            });
        }
        // A native neither VM has: V1 fails at the call with `MISSING_DEPENDENCY` naming it (a
        // framework declaring a native not yet bound), so the native is no gap of MonoMove's alone.
        // `MISSING_DEPENDENCY` is a verification status, so V1's status carries no module: the
        // function's name is matched. Either verdict is neither a pass nor a failure.
        if let UnsupportedKind::MissingNative { function, .. } = &kind
            && v1_vm_status.status_code() == StatusCode::MISSING_DEPENDENCY
            && v1_vm_status.message() == Some(&format!("Missing Native Function `{}`", function))
        {
            return Ok(Outcome::NotComparable {
                v1_status: format!("{:?} (a native neither VM has)", v1.status()),
            });
        }
        return Ok(Outcome::MonoUnsupported {
            kind,
            v1_status: format!("{:?}", v1.status()),
        });
    }
    let v2_output = match v2.output {
        Ok(output) => output,
        Err(err) => {
            let mut diffs = vec![err];
            if let Some(vm_error) = v2.vm_error {
                diffs.push(format!("V2 reported: {}", vm_error));
            }
            return Ok(Outcome::Mismatch { diffs });
        },
    };

    let diffs = diff_outputs(&v1, &v2_output, |key| {
        state
            .get_state_value(key)
            .ok()
            .flatten()
            .map(|value| value.bytes().to_vec())
    });
    let verdict = classify(&diffs);
    let mut diffs: Vec<String> = diffs.iter().map(|diff| diff.describe()).collect();
    Ok(match verdict {
        Verdict::Match => Outcome::Match,
        Verdict::KnownDifference => Outcome::KnownDifference { diffs },
        Verdict::Mismatch => {
            if let Some(err) = v2.vm_error {
                diffs.push(format!("V2 reported: {}", err));
            }
            Outcome::Mismatch { diffs }
        },
    })
}

/// Totals over many records.
#[derive(Debug, Default, Serialize)]
pub struct Summary {
    pub records: usize,
    pub matches: usize,
    pub known_differences: usize,
    pub mismatches: usize,
    pub not_candidates: usize,
    pub not_comparable: usize,
    /// Records a VM read unobserved state for, by VM.
    pub incomplete: BTreeMap<String, usize>,
    pub unsupported: usize,
    /// Records whose MonoMove process exceeded a limit or died.
    pub mono_crashes: usize,
    pub v1_errors: usize,
    pub skipped: usize,
    /// Records whose on-chain status was unavailable, so not filtered as candidates.
    pub unchecked: usize,
    pub overrides: OverrideCounts,
    /// MonoMove gaps, most frequent first.
    pub unsupported_kinds: Vec<(UnsupportedKind, usize)>,
    /// Versions recorded by more than one corpus, so counted once per corpus: the same
    /// transaction, unless the corpora are of different networks.
    pub versions_in_several_corpora: usize,
}

impl Summary {
    pub fn new<'a>(results: impl IntoIterator<Item = &'a RecordResult>) -> Self {
        let mut builder = SummaryBuilder::default();
        for result in results {
            builder.add(result);
        }
        builder.finish()
    }
}

/// A [`Summary`] built one result at a time, so that the results need not be kept.
#[derive(Default)]
pub struct SummaryBuilder {
    summary: Summary,
    kinds: BTreeMap<UnsupportedKind, usize>,
    corpora: BTreeMap<Version, BTreeSet<String>>,
}

impl SummaryBuilder {
    pub fn add(&mut self, result: &RecordResult) {
        let summary = &mut self.summary;
        summary.records += 1;
        let corpora = self.corpora.entry(result.version).or_default();
        if !corpora.contains(&result.corpus_id) {
            corpora.insert(result.corpus_id.clone());
        }
        if !result.onchain_checked {
            summary.unchecked += 1;
        }
        summary.overrides.add(&result.overrides);
        match &result.outcome {
            Outcome::Match => summary.matches += 1,
            Outcome::KnownDifference { .. } => summary.known_differences += 1,
            Outcome::Mismatch { .. } => summary.mismatches += 1,
            Outcome::Incomplete { vm, .. } => {
                *summary.incomplete.entry(vm.clone()).or_default() += 1
            },
            Outcome::NotCandidate { .. } => summary.not_candidates += 1,
            Outcome::NotComparable { .. } => summary.not_comparable += 1,
            Outcome::MonoUnsupported { kind, .. } => {
                summary.unsupported += 1;
                *self.kinds.entry(kind.clone()).or_default() += 1;
            },
            Outcome::MonoCrash { .. } => summary.mono_crashes += 1,
            Outcome::V1Error { .. } => summary.v1_errors += 1,
            Outcome::Skipped { .. } => summary.skipped += 1,
        }
    }

    pub fn finish(self) -> Summary {
        let mut summary = self.summary;
        summary.versions_in_several_corpora = self
            .corpora
            .values()
            .filter(|corpora| corpora.len() > 1)
            .count();
        summary.unsupported_kinds = self.kinds.into_iter().collect();
        summary
            .unsupported_kinds
            .sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v1::hit_metering_limit;
    use aptos_types::transaction::ExecutionStatus;
    use move_core_types::vm_status::{AbortLocation, StatusCode};

    #[test]
    fn only_metering_limits_are_not_comparable() {
        let keep_misc =
            |code| TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(code)));
        assert!(hit_metering_limit(&TransactionStatus::Keep(
            ExecutionStatus::OutOfGas
        )));
        assert!(hit_metering_limit(&keep_misc(
            StatusCode::DEPENDENCY_LIMIT_REACHED
        )));
        assert!(hit_metering_limit(&keep_misc(
            StatusCode::MEMORY_LIMIT_EXCEEDED
        )));
        // Limits MonoMove must enforce like V1 stay comparable: the write-set size limits are a
        // check on the change set, not the gas meter. V1 reports them without a location, which
        // commits as an execution failure in the script.
        assert!(!hit_metering_limit(&TransactionStatus::Keep(
            ExecutionStatus::ExecutionFailure {
                location: AbortLocation::Script,
                function: 0,
                code_offset: 0,
            }
        )));
        assert!(!hit_metering_limit(&keep_misc(
            StatusCode::TYPE_TAG_LIMIT_EXCEEDED
        )));
        assert!(!hit_metering_limit(&TransactionStatus::Keep(
            ExecutionStatus::Success
        )));
    }
}
