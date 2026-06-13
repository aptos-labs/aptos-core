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
        let verdict = match v2_write {
            None => {
                missing += 1;
                ResourceVerdict::MissingInV2
            },
            Some(ResourceWrite::Value(v2_bytes)) => match op {
                Op::New(v1_bytes) | Op::Modify(v1_bytes) => {
                    if v1_bytes.as_ref() == v2_bytes.as_slice() {
                        matched += 1;
                        ResourceVerdict::Match
                    } else {
                        mismatched += 1;
                        ResourceVerdict::Mismatch {
                            v1_len: v1_bytes.len(),
                            v2_len: Some(v2_bytes.len()),
                        }
                    }
                },
                Op::Delete => {
                    mismatched += 1;
                    ResourceVerdict::Mismatch {
                        v1_len: 0,
                        v2_len: Some(v2_bytes.len()),
                    }
                },
            },
            Some(ResourceWrite::Deleted) => match op {
                Op::Delete => {
                    matched += 1;
                    ResourceVerdict::Match
                },
                Op::New(v1_bytes) | Op::Modify(v1_bytes) => {
                    mismatched += 1;
                    ResourceVerdict::Mismatch {
                        v1_len: v1_bytes.len(),
                        v2_len: None,
                    }
                },
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
