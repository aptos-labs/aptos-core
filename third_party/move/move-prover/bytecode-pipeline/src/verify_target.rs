// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `VerifyTargetSelector`: identity of the verify-target set a single `.bpl`
//! emits. Shared by the Boogie translator and the per-VC codegen slicing.

use move_model::model::{FunId, ModuleId, QualifiedId};
use move_stackless_bytecode::function_target_pipeline::FunctionVariant;
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Debug, Clone)]
pub enum VerifyTargetSelector {
    /// Emit every verify target. Used when sharding is disabled (`shards == 1`)
    /// and granularity is `Shard`.
    All,
    /// Hash partition: include a target iff `hash(full_name) % total == idx`.
    /// Used by sharded mode with `shards > 1`.
    Shard { idx: usize, total: usize },
    /// Per-module partition: include targets in the named module.
    Module { module_id: ModuleId },
    /// Per-VC partition: include only the named verify target.
    Single {
        qid: QualifiedId<FunId>,
        variant: FunctionVariant,
    },
}

impl VerifyTargetSelector {
    /// True iff the verify target identified by `(module_id, qid, variant, full_name)`
    /// belongs to this selector. `full_name` is the same hash key the
    /// Boogie translator uses for shard assignment.
    pub fn includes(
        &self,
        module_id: ModuleId,
        qid: QualifiedId<FunId>,
        variant: &FunctionVariant,
        full_name: &str,
    ) -> bool {
        match self {
            VerifyTargetSelector::All => true,
            VerifyTargetSelector::Shard { idx, total } => {
                function_in_shard(full_name, *idx, *total)
            },
            VerifyTargetSelector::Module {
                module_id: selected,
            } => module_id == *selected,
            VerifyTargetSelector::Single {
                qid: selected_qid,
                variant: selected_variant,
            } => qid == *selected_qid && variant == selected_variant,
        }
    }
}

/// Hash-based shard predicate.
pub fn function_in_shard(fun_full_name: &str, shard_idx: usize, shards_total: usize) -> bool {
    if shards_total <= 1 {
        return true;
    }
    let mut hasher = DefaultHasher::new();
    fun_full_name.hash(&mut hasher);
    (hasher.finish() as usize) % shards_total == shard_idx
}
