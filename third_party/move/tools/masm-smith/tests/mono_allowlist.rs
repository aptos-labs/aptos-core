// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Checks that every divergence the `run_vm_diff` fuzz target allows is backed by a witness the
//! Move VM corpus lists as a MonoMove divergence of the same category.

#[path = "../fuzz/fuzz_targets/common/mono_allowlist.rs"]
mod mono_allowlist;

use mono_allowlist::ALLOWED_DIVERGENCES;
use move_transactional_test_matrix::{DivergenceCategory, MOVE_VM};
use std::collections::BTreeSet;

#[test]
fn allowed_divergences_have_witnesses() {
    let mut signatures = BTreeSet::new();
    for allowed in ALLOWED_DIVERGENCES {
        let fields: Vec<&str> = allowed.signature.split('|').collect();
        assert_eq!(
            fields.len(),
            5,
            "malformed signature `{}`",
            allowed.signature
        );
        // The arm and the v2 status are never wildcards.
        let wildcards = fields.iter().filter(|field| **field == "*").count();
        assert!(
            fields[0] != "*" && fields[3] != "*",
            "`{}` matches any arm or v2 status",
            allowed.signature
        );
        assert!(
            wildcards == 0 || allowed.category == DivergenceCategory::Unsupported,
            "`{}` has wildcards but is not an Unsupported divergence",
            allowed.signature
        );
        let concrete = allowed.signature.replace('*', "ok");
        assert!(
            allowed.matches(&concrete),
            "`{}` rejects `{concrete}`",
            allowed.signature
        );
        let other_v2 = [fields[0], fields[1], fields[2], "NO_SUCH_STATUS", fields[4]].join("|");
        assert!(
            !allowed.matches(&other_v2),
            "`{}` accepts another v2 status",
            allowed.signature
        );
        assert!(
            signatures.insert(allowed.signature),
            "signature `{}` is allowed twice",
            allowed.signature
        );
        let witness = MOVE_VM
            .mono_move_divergences
            .iter()
            .find(|divergence| divergence.source == allowed.witness)
            .unwrap_or_else(|| {
                panic!(
                    "witness `{}` of `{}` is not a MonoMove divergence in the Move VM corpus: \
                     if MonoMove closed it, remove the allowlist entry",
                    allowed.witness, allowed.signature
                )
            });
        assert_eq!(
            witness.category, allowed.category,
            "category of `{}` differs from its witness `{}`",
            allowed.signature, allowed.witness
        );
    }
}
