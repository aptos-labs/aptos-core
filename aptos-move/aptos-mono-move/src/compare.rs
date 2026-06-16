// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Compares the two VMs' global-storage writes, driven off the legacy VM.
//!
//! For each resource the legacy VM wrote, it looks up MonoMove's write for the
//! same key and byte-compares. One-directional: it does not flag resources that
//! only MonoMove wrote. Sufficient for a first version.

use crate::{v1::V1Outcome, v2::V2Outcome};
use mono_move_runtime::ResourceWrite;
use move_core_types::{account_address::AccountAddress, effects::Op, language_storage::StructTag};
use std::collections::BTreeSet;

/// Per-resource comparison outcome.
#[derive(Debug)]
pub enum ResourceVerdict {
    /// Same op and identical bytes.
    Match,
    /// Both VMs wrote, but the result differs.
    Mismatch {
        v1_len: usize,
        v2_len: Option<usize>,
    },
    /// The legacy VM wrote this resource; MonoMove did not.
    MissingInV2,
}

/// Aggregated comparison over all of the legacy VM's writes.
pub struct ResourceDiff {
    pub per_key: Vec<((AccountAddress, StructTag), ResourceVerdict)>,
    pub matched: usize,
    pub mismatched: usize,
    pub missing: usize,
}

/// Compares MonoMove's writes against the legacy VM's, keyed by `(address,
/// StructTag)`.
pub fn compare(v1: &V1Outcome, v2: &V2Outcome) -> ResourceDiff {
    let mut per_key = Vec::new();
    let (mut matched, mut mismatched, mut missing) = (0, 0, 0);

    for ((addr, tag), op) in &v1.writes {
        let v2_write = v2
            .writes
            .get(&(*addr, tag.clone()))
            .and_then(|w| w.as_ref());
        // The op kind and write kind must agree — creation with creation,
        // modification with modification, deletion with deletion — and the bytes
        // must match. A create/modify disagreement is itself a mismatch.
        let verdict = match (op, v2_write) {
            (_, None) => {
                missing += 1;
                ResourceVerdict::MissingInV2
            },
            (Op::New(a), Some(ResourceWrite::Created(b)))
            | (Op::Modify(a), Some(ResourceWrite::Modified(b)))
                if a.as_ref() == b.as_slice() =>
            {
                matched += 1;
                ResourceVerdict::Match
            },
            (Op::Delete, Some(ResourceWrite::Deleted)) => {
                matched += 1;
                ResourceVerdict::Match
            },
            (op, Some(write)) => {
                mismatched += 1;
                ResourceVerdict::Mismatch {
                    v1_len: op_len(op),
                    v2_len: write_len(write),
                }
            },
        };
        per_key.push(((*addr, tag.clone()), verdict));
    }

    ResourceDiff {
        per_key,
        matched,
        mismatched,
        missing,
    }
}

/// Aggregated table-item write comparison over the union of both VMs' keys.
pub struct TableDiff {
    pub matched: usize,
    pub mismatched: usize,
    /// Written by the legacy VM but not MonoMove.
    pub only_in_v1: usize,
    /// Written by MonoMove but not the legacy VM.
    pub only_in_v2: usize,
}

/// Compares table-item writes by `(table handle address, serialized key)`.
/// Bidirectional: table writes are enumerated on both sides (neither drives the
/// other), so it reports keys only one VM wrote.
pub fn compare_table_writes(v1: &V1Outcome, v2: &V2Outcome) -> TableDiff {
    let mut diff = TableDiff {
        matched: 0,
        mismatched: 0,
        only_in_v1: 0,
        only_in_v2: 0,
    };
    let keys: BTreeSet<_> = v1
        .table_writes
        .keys()
        .chain(v2.table_writes.keys())
        .collect();
    for key in keys {
        match (v1.table_writes.get(key), v2.table_writes.get(key)) {
            (Some(op), Some(write)) => {
                if table_write_matches(op, write) {
                    diff.matched += 1;
                } else {
                    diff.mismatched += 1;
                }
            },
            (Some(_), None) => diff.only_in_v1 += 1,
            (None, Some(_)) => diff.only_in_v2 += 1,
            (None, None) => unreachable!("key came from one of the two maps"),
        }
    }
    diff
}

/// Whether the legacy VM's table op and MonoMove's table write agree on both
/// kind (create / modify / delete) and bytes.
fn table_write_matches(op: &Op<bytes::Bytes>, write: &ResourceWrite) -> bool {
    match (op, write) {
        (Op::New(v1), ResourceWrite::Created(v2))
        | (Op::Modify(v1), ResourceWrite::Modified(v2)) => v1.as_ref() == v2.as_slice(),
        (Op::Delete, ResourceWrite::Deleted) => true,
        _ => false,
    }
}

/// Byte length of a legacy VM write op (0 for a deletion).
fn op_len(op: &Op<bytes::Bytes>) -> usize {
    match op {
        Op::New(bytes) | Op::Modify(bytes) => bytes.len(),
        Op::Delete => 0,
    }
}

/// Byte length of a MonoMove write (`None` for a deletion).
fn write_len(write: &ResourceWrite) -> Option<usize> {
    match write {
        ResourceWrite::Created(bytes) | ResourceWrite::Modified(bytes) => Some(bytes.len()),
        ResourceWrite::Deleted => None,
    }
}
