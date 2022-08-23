// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::pruner::pruner_metadata::PrunerTag;
use crate::stale_node_index_cross_epoch::StaleNodeIndexCrossEpochSchema;
use crate::StaleNodeIndexSchema;
use aptos_jellyfish_merkle::StaleNodeIndex;
use schemadb::schema::{KeyCodec, Schema};

pub trait StaleNodeIndexSchemaTrait: Schema<Key = StaleNodeIndex>
where
    StaleNodeIndex: KeyCodec<Self>,
{
    fn tag() -> PrunerTag;
    fn name() -> &'static str;
}

impl StaleNodeIndexSchemaTrait for StaleNodeIndexSchema {
    fn tag() -> PrunerTag {
        PrunerTag::StateMerklePruner
    }

    fn name() -> &'static str {
        "state_merkle_pruner"
    }
}

impl StaleNodeIndexSchemaTrait for StaleNodeIndexCrossEpochSchema {
    fn tag() -> PrunerTag {
        PrunerTag::EpochEndingStateMerklePruner
    }

    fn name() -> &'static str {
        "epoch_snapshot_pruner"
    }
}
