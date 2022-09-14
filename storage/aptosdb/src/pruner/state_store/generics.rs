// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::schema::db_metadata::DbMetadataKey;
use crate::stale_node_index_cross_epoch::StaleNodeIndexCrossEpochSchema;
use crate::StaleNodeIndexSchema;
use aptos_jellyfish_merkle::StaleNodeIndex;
use schemadb::schema::{KeyCodec, Schema};

pub trait StaleNodeIndexSchemaTrait: Schema<Key = StaleNodeIndex>
where
    StaleNodeIndex: KeyCodec<Self>,
{
    fn tag() -> DbMetadataKey;
    fn name() -> &'static str;
}

impl StaleNodeIndexSchemaTrait for StaleNodeIndexSchema {
    fn tag() -> DbMetadataKey {
        DbMetadataKey::StateMerklePrunerProgress
    }

    fn name() -> &'static str {
        "state_merkle_pruner"
    }
}

impl StaleNodeIndexSchemaTrait for StaleNodeIndexCrossEpochSchema {
    fn tag() -> DbMetadataKey {
        DbMetadataKey::EpochEndingStateMerklePrunerProgress
    }

    fn name() -> &'static str {
        "epoch_snapshot_pruner"
    }
}
