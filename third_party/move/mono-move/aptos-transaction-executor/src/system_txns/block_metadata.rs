// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::common::{call_block_function, serialize, system_txn_outcome, SystemTxnMetadata};
use crate::{
    calls::invariant_violation,
    errors::{MoveExecutionFailure, SystemTxnFailure},
    executor::AptosTransactionExecutor,
    outcome::TxnOutcome,
};
use anyhow::anyhow;
use aptos_types::{
    block_metadata::BlockMetadata, block_metadata_ext::BlockMetadataExt, randomness::Randomness,
    transaction::AuxiliaryInfo,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::InterpreterContext;
use move_core_types::{account_address::AccountAddress, ident_str, identifier::IdentStr};

const BLOCK_PROLOGUE: &IdentStr = ident_str!("block_prologue");
const BLOCK_PROLOGUE_EXT: &IdentStr = ident_str!("block_prologue_ext");
const BLOCK_PROLOGUE_EXT_V2: &IdentStr = ident_str!("block_prologue_ext_v2");
const BLOCK_PROLOGUE_EXT_V3: &IdentStr = ident_str!("block_prologue_ext_v3");

impl<'guard> AptosTransactionExecutor<'guard> {
    /// Executes a block-metadata (system) transaction. The auxiliary info is
    /// unused: system sessions carry no user transaction context.
    pub fn execute_block_metadata_transaction(
        &self,
        block_metadata: &BlockMetadata,
        _aux_info: &AuxiliaryInfo,
    ) -> TxnOutcome<'guard> {
        let txn_data = SystemTxnMetadata::for_block_metadata(block_metadata);
        let mut interp = self.system_session(&txn_data);
        match run_block_prologue(&mut interp, self.guard, block_metadata) {
            Ok(()) => system_txn_outcome(interp),
            Err(failure) => TxnOutcome::UnexpectedSystemTransactionFailure(SystemTxnFailure {
                call: "block_prologue",
                failure,
            }),
        }
    }

    /// Executes an extended block-metadata transaction, which additionally
    /// carries the block's randomness seed and encrypted-transaction
    /// decryption metadata.
    pub fn execute_block_metadata_ext_transaction(
        &self,
        block_metadata_ext: &BlockMetadataExt,
        aux_info: &AuxiliaryInfo,
    ) -> TxnOutcome<'guard> {
        if let BlockMetadataExt::V0(block_metadata) = block_metadata_ext {
            return self.execute_block_metadata_transaction(block_metadata, aux_info);
        }
        let txn_data = SystemTxnMetadata::for_block_metadata_ext(block_metadata_ext);
        let mut interp = self.system_session(&txn_data);
        match run_block_prologue_ext(&mut interp, self.guard, block_metadata_ext) {
            Ok(()) => system_txn_outcome(interp),
            Err(failure) => TxnOutcome::UnexpectedSystemTransactionFailure(SystemTxnFailure {
                call: "block_prologue_ext",
                failure,
            }),
        }
    }
}

/// Arguments shared by every block-prologue variant.
//
// TODO(perf): place the arguments directly into the interpreter's heap,
// avoiding the BCS round trip.
#[allow(clippy::too_many_arguments)]
fn block_prologue_common_args(
    id: Vec<u8>,
    epoch: u64,
    round: u64,
    proposer: AccountAddress,
    failed_proposer_indices: &[u32],
    previous_block_votes_bitvec: &[u8],
    timestamp_usecs: u64,
) -> Result<Vec<Vec<u8>>, MoveExecutionFailure> {
    let hash = AccountAddress::from_bytes(&id)
        .map_err(|e| invariant_violation(anyhow!("block id is not 32 bytes: {e}")))
        .map_err(MoveExecutionFailure::RuntimeError)?;
    let failed_proposer_indices: Vec<u64> = failed_proposer_indices
        .iter()
        .map(|index| u64::from(*index))
        .collect();
    Ok(vec![
        serialize(&hash)?,
        serialize(&epoch)?,
        serialize(&round)?,
        serialize(&proposer)?,
        serialize(&failed_proposer_indices)?,
        serialize(&previous_block_votes_bitvec)?,
        serialize(&timestamp_usecs)?,
    ])
}

/// Calls `0x1::block::block_prologue` with the block's consensus metadata.
fn run_block_prologue(
    interp: &mut InterpreterContext<'_>,
    guard: &ExecutionGuard<'_>,
    block_metadata: &BlockMetadata,
) -> Result<(), MoveExecutionFailure> {
    let args = block_prologue_common_args(
        block_metadata.id().to_vec(),
        block_metadata.epoch(),
        block_metadata.round(),
        block_metadata.proposer(),
        block_metadata.failed_proposer_indices(),
        block_metadata.previous_block_votes_bitvec(),
        block_metadata.timestamp_usecs(),
    )?;
    call_block_function(interp, guard, BLOCK_PROLOGUE, &args)
}

/// Calls the `0x1::block::block_prologue_ext*` variant matching the extended
/// metadata.
fn run_block_prologue_ext(
    interp: &mut InterpreterContext<'_>,
    guard: &ExecutionGuard<'_>,
    block_metadata_ext: &BlockMetadataExt,
) -> Result<(), MoveExecutionFailure> {
    let mut args = block_prologue_common_args(
        block_metadata_ext.id().to_vec(),
        block_metadata_ext.epoch(),
        block_metadata_ext.round(),
        block_metadata_ext.proposer(),
        block_metadata_ext.failed_proposer_indices(),
        block_metadata_ext.previous_block_votes_bitvec(),
        block_metadata_ext.timestamp_usecs(),
    )?;
    let seed = |randomness: &Option<Randomness>| {
        serialize(&randomness.as_ref().map(Randomness::randomness_cloned))
    };
    let function = match block_metadata_ext {
        BlockMetadataExt::V0(_) => {
            return Err(MoveExecutionFailure::RuntimeError(invariant_violation(
                anyhow!("V0 metadata must run as a plain block-metadata transaction"),
            )))
        },
        BlockMetadataExt::V1(v1) => {
            args.push(seed(&v1.randomness)?);
            BLOCK_PROLOGUE_EXT
        },
        BlockMetadataExt::V2(v2) => {
            args.push(seed(&v2.randomness)?);
            args.push(serialize(
                &v2.decryption_key
                    .as_ref()
                    .map(|key| key.decryption_key_cloned()),
            )?);
            BLOCK_PROLOGUE_EXT_V2
        },
        BlockMetadataExt::V3(v3) => {
            args.push(seed(&v3.randomness)?);
            let payload = v3.decryption_payload.as_ref();
            args.push(serialize(
                &payload.map(|payload| payload.key.decryption_key_cloned()),
            )?);
            args.push(serialize(&payload.map(|payload| payload.decryption_round))?);
            BLOCK_PROLOGUE_EXT_V3
        },
    };
    call_block_function(interp, guard, function, &args)
}
