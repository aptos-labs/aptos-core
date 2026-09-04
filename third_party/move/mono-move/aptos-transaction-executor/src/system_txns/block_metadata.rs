// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::common::{call_block_function, system_txn_outcome, SystemTxnMetadata};
use crate::{
    errors::{invariant_violation, MoveExecutionFailure, SystemTxnFailure},
    executor::AptosTransactionExecutor,
    outcome::TxnOutcome,
};
use aptos_types::{
    block_metadata::BlockMetadata, block_metadata_ext::BlockMetadataExt, randomness::Randomness,
    transaction::AuxiliaryInfo,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{CallBuilder, InterpreterContext};
use move_core_types::{account_address::AccountAddress, ident_str, identifier::IdentStr};
use move_value_view::IterAsMoveVector;

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

/// Places the arguments every block-prologue variant takes, after the VM
/// signer `call_block_function` fills. A macro because the two metadata types
/// share these accessors but no trait.
macro_rules! place_prologue_common_args {
    ($call:expr, $metadata:expr) => {{
        let call: &mut CallBuilder<'_, '_> = $call;
        let metadata = $metadata;
        let hash = AccountAddress::from_bytes(metadata.id().as_slice())
            .map_err(|e| invariant_violation(format!("block id is not 32 bytes: {e}")))?;
        call.arg(&hash)?;
        call.arg(&metadata.epoch())?;
        call.arg(&metadata.round())?;
        call.arg(&metadata.proposer())?;
        call.arg(&IterAsMoveVector(
            metadata
                .failed_proposer_indices()
                .iter()
                .map(|index| u64::from(*index)),
        ))?;
        call.arg(&metadata.previous_block_votes_bitvec())?;
        call.arg(&metadata.timestamp_usecs())
    }};
}

/// Calls `0x1::block::block_prologue` with the block's consensus metadata.
fn run_block_prologue<'a>(
    interp: &mut InterpreterContext<'a>,
    guard: &ExecutionGuard<'a>,
    block_metadata: &BlockMetadata,
) -> Result<(), MoveExecutionFailure> {
    call_block_function(interp, guard, BLOCK_PROLOGUE, |call| {
        place_prologue_common_args!(call, block_metadata)
    })
}

/// Calls the `0x1::block::block_prologue_ext*` variant matching the extended
/// metadata.
fn run_block_prologue_ext<'a>(
    interp: &mut InterpreterContext<'a>,
    guard: &ExecutionGuard<'a>,
    block_metadata_ext: &BlockMetadataExt,
) -> Result<(), MoveExecutionFailure> {
    let seed =
        |randomness: &Option<Randomness>| randomness.as_ref().map(Randomness::randomness_cloned);
    match block_metadata_ext {
        BlockMetadataExt::V0(_) => Err(MoveExecutionFailure::RuntimeError(invariant_violation(
            "V0 metadata must run as a plain block-metadata transaction",
        ))),
        BlockMetadataExt::V1(v1) => {
            call_block_function(interp, guard, BLOCK_PROLOGUE_EXT, |call| {
                place_prologue_common_args!(call, block_metadata_ext)?;
                call.arg(&seed(&v1.randomness))
            })
        },
        BlockMetadataExt::V2(v2) => {
            call_block_function(interp, guard, BLOCK_PROLOGUE_EXT_V2, |call| {
                place_prologue_common_args!(call, block_metadata_ext)?;
                call.arg(&seed(&v2.randomness))?;
                call.arg(
                    &v2.decryption_key
                        .as_ref()
                        .map(|key| key.decryption_key_cloned()),
                )
            })
        },
        BlockMetadataExt::V3(v3) => {
            call_block_function(interp, guard, BLOCK_PROLOGUE_EXT_V3, |call| {
                place_prologue_common_args!(call, block_metadata_ext)?;
                call.arg(&seed(&v3.randomness))?;
                let payload = v3.decryption_payload.as_ref();
                call.arg(&payload.map(|payload| payload.key.decryption_key_cloned()))?;
                call.arg(&payload.map(|payload| payload.decryption_round))
            })
        },
    }
}
