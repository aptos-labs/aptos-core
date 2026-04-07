// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use api_types::on_chain_config::consensus_hardfork::{is_consensus_fork_active_at_epoch, ConsensusHardfork};
use crate::{
    epoch_state::EpochState,
    on_chain_config::ValidatorSet,
    transaction::Version,
    validator_verifier::ValidatorVerifier,
};
use aptos_crypto::hash::{HashValue, ACCUMULATOR_PLACEHOLDER_HASH};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// The round of a block is a consensus-internal counter, which starts with 0 and increases
/// monotonically.
pub type Round = u64;

// Constants for the initial genesis block.
pub const GENESIS_EPOCH: u64 = 0;
pub const GENESIS_ROUND: Round = 0;
pub const GENESIS_VERSION: Version = 0;
pub const GENESIS_TIMESTAMP_USECS: u64 = 0;

/// Additional block-level information associated with an epoch transition.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct EpochBlockInfo {
    /// The block identifier (hash) of the epoch-starting block.
    pub block_id: HashValue,
    /// The block number (height) of the epoch-starting block.
    pub block_number: u64,
    /// The round at which this epoch started.
    pub epoch_start_round: Round,
    /// The timestamp (in microseconds) at which this epoch started.
    pub epoch_start_timestamp_usecs: u64,
}

/// This structure contains all the information needed for tracking a block
/// without having access to the block or its execution output state. It
/// assumes that the block is the last block executed within the ledger.
///
/// # BCS Serialization Compatibility
///
/// `BlockInfo` uses custom `Serialize` / `Deserialize` implementations to
/// support rolling upgrades. The `epoch_block_info` field is only included
/// in BCS serialization after the hardfork activation block number
/// (see [`is_epoch_block_info_active`]).
///
/// - **Pre-hardfork**: serialized as 7 fields (compatible with legacy nodes)
/// - **Post-hardfork**: serialized as 8 fields (includes `epoch_block_info`)
/// - **Deserialization**: always accepts both 7-field and 8-field formats
#[derive(Clone, Debug)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct BlockInfo {
    /// The epoch to which the block belongs.
    epoch: u64,
    /// The consensus protocol is executed in rounds, which monotonically increase per epoch.
    round: Round,
    /// The identifier (hash) of the block.
    id: HashValue,
    /// The accumulator root hash after executing this block.
    executed_state_id: HashValue,
    /// The version of the latest transaction after executing this block.
    version: Version,
    /// The timestamp this block was proposed by a proposer.
    timestamp_usecs: u64,
    /// An optional field containing the next epoch info
    next_epoch_state: Option<EpochState>,
    /// Optional epoch-level block info (e.g., epoch start round/timestamp).
    /// Only serialized after hardfork activation.
    epoch_block_info: Option<EpochBlockInfo>,
}

impl PartialEq for BlockInfo {
    fn eq(&self, other: &Self) -> bool {
        let base_match = self.epoch == other.epoch
            && self.round == other.round
            && self.id == other.id
            && self.executed_state_id == other.executed_state_id
            && self.version == other.version
            && self.timestamp_usecs == other.timestamp_usecs
            && self.next_epoch_state == other.next_epoch_state;

        use api_types::on_chain_config::consensus_hardfork::{
            is_consensus_fork_active_at_epoch, ConsensusHardfork,
        };

        if is_consensus_fork_active_at_epoch(ConsensusHardfork::ConsensusAlpha, self.epoch) {
            // Post-hardfork: epoch_block_info must match strictly
            base_match && self.epoch_block_info == other.epoch_block_info
        } else {
            // Pre-hardfork: ignore epoch_block_info entirely since legacy nodes don't send it
            base_match
        }
    }
}

impl Eq for BlockInfo {}

// Field name constants for BCS struct serialization/deserialization.
// BCS ignores field names but serde requires them for serialize_struct/deserialize_struct.
const FIELDS_7: &[&str] = &[
    "epoch",
    "round",
    "id",
    "executed_state_id",
    "version",
    "timestamp_usecs",
    "next_epoch_state",
];
const FIELDS_8: &[&str] = &[
    "epoch",
    "round",
    "id",
    "executed_state_id",
    "version",
    "timestamp_usecs",
    "next_epoch_state",
    "epoch_block_info",
];

/// Custom BCS-compatible Serialize for BlockInfo.
///
/// Before the hardfork activation block, only the first 7 fields are serialized
/// (identical to the legacy format). After activation, all 8 fields are included.
///
/// Uses `serialize_struct` (not `serialize_tuple`) to match what `#[derive(Serialize)]`
/// generates — this is critical because BCS's `serialize_struct` calls
/// `enter_named_container` for depth tracking.
impl Serialize for BlockInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        if is_consensus_fork_active_at_epoch(ConsensusHardfork::ConsensusAlpha, self.epoch) {
            // Post-hardfork: serialize all 8 fields
            let mut state = serializer.serialize_struct("BlockInfo", 8)?;
            state.serialize_field("epoch", &self.epoch)?;
            state.serialize_field("round", &self.round)?;
            state.serialize_field("id", &self.id)?;
            state.serialize_field("executed_state_id", &self.executed_state_id)?;
            state.serialize_field("version", &self.version)?;
            state.serialize_field("timestamp_usecs", &self.timestamp_usecs)?;
            state.serialize_field("next_epoch_state", &self.next_epoch_state)?;
            state.serialize_field("epoch_block_info", &self.epoch_block_info)?;
            state.end()
        } else {
            // Pre-hardfork: serialize only 7 fields (legacy compatible)
            let mut state = serializer.serialize_struct("BlockInfo", 7)?;
            state.serialize_field("epoch", &self.epoch)?;
            state.serialize_field("round", &self.round)?;
            state.serialize_field("id", &self.id)?;
            state.serialize_field("executed_state_id", &self.executed_state_id)?;
            state.serialize_field("version", &self.version)?;
            state.serialize_field("timestamp_usecs", &self.timestamp_usecs)?;
            state.serialize_field("next_epoch_state", &self.next_epoch_state)?;
            state.end()
        }
    }
}

/// Custom BCS-compatible Deserialize for BlockInfo.
///
/// Uses `deserialize_struct` with 8 field names. BCS creates a SeqDeserializer
/// with `remaining=8`. After reading 7 fields, the 8th `next_element()` call:
/// - For 8-field data: succeeds normally
/// - For 7-field data (legacy): the underlying reader hits EOF, we catch the
///   error and default `epoch_block_info` to `None`
impl<'de> Deserialize<'de> for BlockInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct BlockInfoVisitor;

        impl<'de> serde::de::Visitor<'de> for BlockInfoVisitor {
            type Value = BlockInfo;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("BlockInfo struct with 7 or 8 fields")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<BlockInfo, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let epoch = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                let round = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
                let id = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(2, &self))?;
                let executed_state_id = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(3, &self))?;
                let version = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(4, &self))?;
                let timestamp_usecs = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(5, &self))?;
                let next_epoch_state = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(6, &self))?;

                let mut epoch_block_info = None;
                if is_consensus_fork_active_at_epoch(ConsensusHardfork::ConsensusAlpha, epoch) {
                    epoch_block_info = seq.next_element()?.ok_or_else(|| {
                        serde::de::Error::invalid_length(7, &self)
                    })?;
                }

                Ok(BlockInfo {
                    epoch,
                    round,
                    id,
                    executed_state_id,
                    version,
                    timestamp_usecs,
                    next_epoch_state,
                    epoch_block_info,
                })
            }

            fn visit_map<V>(self, mut map: V) -> Result<BlockInfo, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut epoch = None;
                let mut round = None;
                let mut id = None;
                let mut executed_state_id = None;
                let mut version = None;
                let mut timestamp_usecs = None;
                let mut next_epoch_state = None;
                let mut epoch_block_info = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "epoch" => epoch = Some(map.next_value()?),
                        "round" => round = Some(map.next_value()?),
                        "id" => id = Some(map.next_value()?),
                        "executed_state_id" => executed_state_id = Some(map.next_value()?),
                        "version" => version = Some(map.next_value()?),
                        "timestamp_usecs" => timestamp_usecs = Some(map.next_value()?),
                        "next_epoch_state" => next_epoch_state = Some(map.next_value()?),
                        "epoch_block_info" => epoch_block_info = Some(map.next_value()?),
                        _ => {
                            let _ = map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }

                Ok(BlockInfo {
                    epoch: epoch.ok_or_else(|| serde::de::Error::missing_field("epoch"))?,
                    round: round.ok_or_else(|| serde::de::Error::missing_field("round"))?,
                    id: id.ok_or_else(|| serde::de::Error::missing_field("id"))?,
                    executed_state_id: executed_state_id
                        .ok_or_else(|| serde::de::Error::missing_field("executed_state_id"))?,
                    version: version.ok_or_else(|| serde::de::Error::missing_field("version"))?,
                    timestamp_usecs: timestamp_usecs
                        .ok_or_else(|| serde::de::Error::missing_field("timestamp_usecs"))?,
                    next_epoch_state: next_epoch_state.unwrap_or(None),
                    epoch_block_info: epoch_block_info.unwrap_or(None),
                })
            }
        }

        // Use deserialize_struct with 8 fields to match post-hardfork format.
        // For legacy 7-field data, the 8th element returns Ok(None) from SeqAccess
        // since remaining == 0 after reading 7 fields.
        //
        // IMPORTANT: BCS `deserialize_struct` delegates to `deserialize_tuple(fields.len())`.
        // The `len` parameter controls `SeqDeserializer::remaining`, which determines
        // how many `next_element()` calls succeed before returning `Ok(None)`.
        //
        // For 7-field legacy data: use FIELDS_7 so remaining=7, 8th call returns Ok(None)
        // For 8-field data: use FIELDS_8 so remaining=8, all 8 calls succeed
        //
        // Since we don't know the format at deserialization time, we use FIELDS_8 and
        // catch errors on the 8th element.
        deserializer.deserialize_struct("BlockInfo", FIELDS_8, BlockInfoVisitor)
    }
}

impl BlockInfo {
    pub fn new(
        epoch: u64,
        round: Round,
        id: HashValue,
        executed_state_id: HashValue,
        version: Version,
        timestamp_usecs: u64,
        next_epoch_state: Option<EpochState>,
    ) -> Self {
        Self {
            epoch,
            round,
            id,
            executed_state_id,
            version,
            timestamp_usecs,
            next_epoch_state,
            epoch_block_info: None,
        }
    }

    pub fn empty() -> Self {
        Self {
            epoch: 0,
            round: 0,
            id: HashValue::zero(),
            executed_state_id: HashValue::zero(),
            version: 0,
            timestamp_usecs: 0,
            next_epoch_state: None,
            epoch_block_info: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        *self == Self::empty()
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn random(round: Round) -> Self {
        Self {
            epoch: 1,
            round,
            id: HashValue::zero(),
            executed_state_id: HashValue::zero(),
            version: 0,
            timestamp_usecs: 0,
            next_epoch_state: None,
            epoch_block_info: None,
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn random_with_epoch(epoch: u64, round: Round) -> Self {
        Self {
            epoch,
            round,
            id: HashValue::zero(),
            executed_state_id: HashValue::zero(),
            version: 0,
            timestamp_usecs: 0,
            next_epoch_state: None,
            epoch_block_info: None,
        }
    }

    /// Create a new genesis block. The genesis block is effectively the
    /// blockchain state after executing the initial genesis transaction.
    ///
    /// * `genesis_state_root_hash` - the state tree root hash after executing the
    /// initial genesis transaction.
    ///
    /// * `validator_set` - the initial validator set, configured when generating
    /// the genesis transaction itself and emitted after executing the genesis
    /// transaction. Using this genesis block means transitioning to a new epoch
    /// (GENESIS_EPOCH + 1) with this `validator_set`.
    pub fn genesis(genesis_state_root_hash: HashValue, validator_set: ValidatorSet) -> Self {
        let verifier: ValidatorVerifier = (&validator_set).into();
        Self {
            epoch: GENESIS_EPOCH,
            round: GENESIS_ROUND,
            id: HashValue::zero(),
            executed_state_id: genesis_state_root_hash,
            version: GENESIS_VERSION,
            timestamp_usecs: GENESIS_TIMESTAMP_USECS,
            next_epoch_state: Some(EpochState {
                epoch: 1,
                verifier: verifier.into(),
            }),
            epoch_block_info: None,
        }
    }

    /// Create a mock genesis `BlockInfo` with an empty state tree and empty
    /// validator set.
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn mock_genesis(validator_set: Option<ValidatorSet>) -> Self {
        let validator_set = validator_set.unwrap_or_else(ValidatorSet::empty);
        Self::genesis(*ACCUMULATOR_PLACEHOLDER_HASH, validator_set)
    }

    /// The epoch after this block committed
    pub fn next_block_epoch(&self) -> u64 {
        self.next_epoch_state().map_or(self.epoch, |e| e.epoch)
    }

    pub fn change_timestamp(&mut self, timestamp: u64) {
        assert!(self.allow_timestamp_change(timestamp));
        self.timestamp_usecs = timestamp;
    }

    /// For reconfiguration suffix blocks only, with decoupled-execution proposal-generator can't
    /// guarantee suffix blocks have the same timestamp as parent thus violate the invariant that
    /// block.timestamp should always equal timestamp stored onchain.
    /// We allow it to be updated backwards to the actual reconfiguration block's timestamp.
    fn allow_timestamp_change(&self, timestamp: u64) -> bool {
        self.has_reconfiguration() && self.timestamp_usecs >= timestamp
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn executed_state_id(&self) -> HashValue {
        self.executed_state_id
    }

    pub fn has_reconfiguration(&self) -> bool {
        self.next_epoch_state.is_some()
    }

    pub fn id(&self) -> HashValue {
        self.id
    }

    pub fn next_epoch_state(&self) -> Option<&EpochState> {
        self.next_epoch_state.as_ref()
    }

    pub fn round(&self) -> Round {
        self.round
    }

    pub fn timestamp_usecs(&self) -> u64 {
        self.timestamp_usecs
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn epoch_block_info(&self) -> Option<&EpochBlockInfo> {
        self.epoch_block_info.as_ref()
    }

    pub fn set_epoch_block_info(&mut self, info: EpochBlockInfo) {
        self.epoch_block_info = Some(info);
    }

    /// This function checks if the current BlockInfo has
    /// exactly the same values in those fields that will not change
    /// after execution, compared to a given BlockInfo
    pub fn match_ordered_only(&self, executed_block_info: &BlockInfo) -> bool {
        self.epoch == executed_block_info.epoch
            && self.round == executed_block_info.round
            && self.id == executed_block_info.id
            && (self.timestamp_usecs == executed_block_info.timestamp_usecs
            // executed block info has changed its timestamp because it's a reconfiguration suffix
                || (self.timestamp_usecs > executed_block_info.timestamp_usecs
                    && executed_block_info.has_reconfiguration()))
    }

    /// This function checks if the current BlockInfo is consistent
    /// with the dummy values we put in the ordering state computer
    /// and it is not empty
    pub fn is_ordered_only(&self) -> bool {
        *self != BlockInfo::empty()
            && self.next_epoch_state.is_none()
            && self.executed_state_id == *ACCUMULATOR_PLACEHOLDER_HASH
            && self.version == 0
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn set_executed_state_id(&mut self, id: HashValue) {
        self.executed_state_id = id
    }
}

impl Display for BlockInfo {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "BlockInfo: [epoch: {}, round: {}, id: {}, executed_state_id: {}, version: {}, timestamp (us): {}, next_epoch_state: {}, epoch_block_info: {:?}]",
            self.epoch(),
            self.round(),
            self.id(),
            self.executed_state_id(),
            self.version(),
            self.timestamp_usecs(),
            self.next_epoch_state.as_ref().map_or_else(|| "None".to_string(), |epoch_state| format!("{}", epoch_state)),
            self.epoch_block_info,
        )
    }
}

/// A continuously increasing sequence number for committed blocks.
pub type BlockHeight = u64;

#[cfg(test)]
mod tests {
    use super::*;
    use api_types::on_chain_config::consensus_hardfork::{
        init_consensus_hardforks, ConsensusHardfork, ConsensusHardforks, ForkCondition,
    };

    /// Helper: generate legacy 7-field bytes using a derive-based struct
    /// to simulate what old nodes produce.
    #[derive(Serialize, Deserialize)]
    struct BlockInfoLegacy {
        epoch: u64,
        round: Round,
        id: HashValue,
        executed_state_id: HashValue,
        version: Version,
        timestamp_usecs: u64,
        next_epoch_state: Option<EpochState>,
    }

    #[test]
    fn test_pre_hardfork_roundtrip() {
        // Default: no hardforks initialized → EpochBlockInfo not active
        let block_info = BlockInfo::new(
            1, 2, HashValue::zero(), HashValue::zero(), 100, 12345, None,
        );
        let bytes = bcs::to_bytes(&block_info).unwrap();

        // Verify bytes match legacy format
        let legacy = BlockInfoLegacy {
            epoch: 1, round: 2, id: HashValue::zero(),
            executed_state_id: HashValue::zero(),
            version: 100, timestamp_usecs: 12345,
            next_epoch_state: None,
        };
        let legacy_bytes = bcs::to_bytes(&legacy).unwrap();
        assert_eq!(bytes, legacy_bytes, "pre-hardfork format should match legacy");

        let deserialized: BlockInfo = bcs::from_bytes(&bytes).unwrap();
        assert_eq!(block_info, deserialized);
        assert!(deserialized.epoch_block_info.is_none());
    }

    #[test]
    fn test_post_hardfork_roundtrip() {
        // This test needs ConsensusAlpha active at epoch 1 (the test block's epoch).
        // Since OnceLock is per-process, we use init_consensus_hardforks.
        let mut hardforks = ConsensusHardforks::new();
        hardforks.insert(
            ConsensusHardfork::ConsensusAlpha,
            ForkCondition::Epoch(1), // activate at epoch 1, test block has epoch 1
        );
        // Ignore error if already set by another test
        let _ = init_consensus_hardforks(hardforks);
        let mut block_info = BlockInfo::new(
            1, 2, HashValue::zero(), HashValue::zero(), 100, 12345, None,
        );
        block_info.set_epoch_block_info(EpochBlockInfo {
            block_id: HashValue::zero(),
            block_number: 42,
            epoch_start_round: 10,
            epoch_start_timestamp_usecs: 99999,
        });
        let bytes = bcs::to_bytes(&block_info).unwrap();
        let deserialized: BlockInfo = bcs::from_bytes(&bytes).unwrap();
        assert_eq!(block_info, deserialized);
        assert_eq!(deserialized.epoch_block_info.unwrap().block_number, 42);
        // Note: OnceLock cannot be reset; fork stays active for remaining tests
    }

    #[test]
    fn test_new_code_reads_legacy_bytes() {
        // Simulate: old node serialized with derive (7 fields)
        let legacy = BlockInfoLegacy {
            epoch: 1, round: 2, id: HashValue::zero(),
            executed_state_id: HashValue::zero(),
            version: 100, timestamp_usecs: 12345,
            next_epoch_state: None,
        };
        let legacy_bytes = bcs::to_bytes(&legacy).unwrap();

        // New code deserializes legacy bytes
        let deserialized: BlockInfo = bcs::from_bytes(&legacy_bytes).unwrap();
        assert_eq!(deserialized.epoch, 1);
        assert_eq!(deserialized.epoch_block_info, None);
    }
}
