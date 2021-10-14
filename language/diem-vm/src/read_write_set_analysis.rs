// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    adapter_common::PreprocessedTransaction,
    script_to_script_function::remapping,
    system_module_names::{
        BLOCK_PROLOGUE, DIEM_BLOCK_MODULE, SCRIPT_PROLOGUE_NAME, USER_EPILOGUE_NAME,
    },
};
use anyhow::{anyhow, bail, Result};
use diem_types::{
    account_config,
    transaction::{SignedTransaction, TransactionPayload},
};
use move_binary_format::layout::ModuleCache;
use move_core_types::{
    ident_str,
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, ResourceKey, StructTag, TypeTag},
    resolver::{ModuleResolver, MoveResolver},
    transaction_argument::convert_txn_args,
    value::{serialize_values, MoveValue},
};
use read_write_set_dynamic::{ConcretizedFormals, NormalizedReadWriteSetAnalysis};
use std::ops::Deref;

pub struct ReadWriteSetAnalysis<'a, R: ModuleResolver> {
    normalized_analysis_result: &'a NormalizedReadWriteSetAnalysis,
    module_cache: ModuleCache<&'a R>,
    blockchain_view: &'a R,
}

const TRANSACTION_FEES_NAME: &IdentStr = ident_str!("TransactionFee");

pub fn add_on_functions_list() -> Vec<(ModuleId, Identifier)> {
    vec![
        (DIEM_BLOCK_MODULE.clone(), BLOCK_PROLOGUE.to_owned()),
        (
            account_config::constants::ACCOUNT_MODULE.clone(),
            SCRIPT_PROLOGUE_NAME.to_owned(),
        ),
        (
            account_config::constants::ACCOUNT_MODULE.clone(),
            USER_EPILOGUE_NAME.to_owned(),
        ),
    ]
}

impl<'a, R: MoveResolver> ReadWriteSetAnalysis<'a, R> {
    /// Create a Diem transaction read/write set analysis from a generic Move module read/write set
    /// analysis and a view of the current blockchain for module fetching and access concretization.
    pub fn new(rw: &'a NormalizedReadWriteSetAnalysis, blockchain_view: &'a R) -> Self {
        ReadWriteSetAnalysis {
            normalized_analysis_result: rw,
            module_cache: ModuleCache::new(blockchain_view),
            blockchain_view,
        }
    }

    /// Returns an overapproximation of the `ResourceKey`'s in global storage that will be written
    /// by `tx`
    /// Note: this will return both writes performed by the transaction prologue/epilogue and by its
    /// embedded payload.
    ///
    /// If `concretize` is true the analyzer will concretize the secondary indices in the analysis
    /// with the state state `self.blockchain_view`.
    pub fn get_keys_written(
        &self,
        tx: &SignedTransaction,
        concretize: bool,
    ) -> Result<Vec<ResourceKey>> {
        Ok(self.get_keys_user_transaction(tx, concretize)?.1)
    }

    /// Returns an overapproximation of the `ResourceKey`'s in global storage that will be read
    /// by `tx`
    /// Note: this will return both writes performed by the transaction prologue/epilogue and by its
    /// embedded payload.
    ///
    /// If `concretize` is true the analyzer will concretize the secondary indices in the analysis
    /// with the state state `self.blockchain_view`.
    pub fn get_keys_read(
        &self,
        tx: &SignedTransaction,
        concretize: bool,
    ) -> Result<Vec<ResourceKey>> {
        Ok(self.get_keys_user_transaction(tx, concretize)?.0)
    }

    pub fn get_keys_user_transaction(
        &self,
        tx: &SignedTransaction,
        concretize: bool,
    ) -> Result<(Vec<ResourceKey>, Vec<ResourceKey>)> {
        match tx.payload() {
            TransactionPayload::ScriptFunction(s) => self.get_concretized_keys_script_function(
                tx,
                s.module(),
                s.function(),
                s.args(),
                s.ty_args(),
                concretize,
            ),
            TransactionPayload::Script(s) => {
                if let Some((module, func_name)) = remapping(s.code()) {
                    self.get_concretized_keys_script_function(
                        tx,
                        module,
                        func_name,
                        convert_txn_args(s.args()).as_slice(),
                        s.ty_args(),
                        concretize,
                    )
                } else {
                    bail!("Unsupported transaction script type {:?}", s)
                }
            }
            payload => {
                // TODO: support tx scripts here. Slightly tricky since we will need to run
                // analyzer on the fly
                bail!("Unsupported transaction payload type {:?}", payload)
            }
        }
    }

    fn concretize_binded_accesses(
        &self,
        binded_result: ConcretizedFormals,
        concretize: bool,
    ) -> Result<(Vec<ResourceKey>, Vec<ResourceKey>)> {
        if concretize {
            let concretized_accesses = binded_result
                .concretize_secondary_indexes(&self.blockchain_view)
                .ok_or_else(|| anyhow!("Failed to concretize read/write set"))?;
            Ok((
                concretized_accesses
                    .get_keys_read()
                    .ok_or_else(|| anyhow!("Failed to get read set"))?,
                concretized_accesses
                    .get_keys_written()
                    .ok_or_else(|| anyhow!("Failed to get write set"))?,
            ))
        } else {
            Ok((
                binded_result
                    .get_keys_read()
                    .ok_or_else(|| anyhow!("Failed to get read set"))?,
                binded_result
                    .get_keys_written()
                    .ok_or_else(|| anyhow!("Failed to get write set"))?,
            ))
        }
    }

    pub fn get_keys_transaction(
        &self,
        tx: &PreprocessedTransaction,
        concretize: bool,
    ) -> Result<(Vec<ResourceKey>, Vec<ResourceKey>)> {
        match tx {
            PreprocessedTransaction::UserTransaction(tx) => {
                self.get_keys_user_transaction(tx, concretize)
            }
            PreprocessedTransaction::BlockMetadata(block_metadata) => {
                let (round, timestamp, previous_vote, proposer) =
                    block_metadata.clone().into_inner();
                let args = serialize_values(&vec![
                    MoveValue::Signer(account_config::reserved_vm_address()),
                    MoveValue::U64(round),
                    MoveValue::U64(timestamp),
                    MoveValue::Vector(previous_vote.into_iter().map(MoveValue::Address).collect()),
                    MoveValue::Address(proposer),
                ]);
                let metadata_access = self.get_binded_summary(
                    &DIEM_BLOCK_MODULE,
                    BLOCK_PROLOGUE,
                    &[],
                    &args,
                    &[],
                    &self.module_cache,
                )?;
                self.concretize_binded_accesses(metadata_access, concretize)
            }
            PreprocessedTransaction::InvalidSignature => Ok((vec![], vec![])),
            PreprocessedTransaction::WriteSet(_) | PreprocessedTransaction::WaypointWriteSet(_) => {
                bail!("Unsupported writeset transaction")
            }
        }
    }

    fn get_concretized_keys_script_function(
        &self,
        tx: &SignedTransaction,
        module_name: &ModuleId,
        script_name: &IdentStr,
        actuals: &[Vec<u8>],
        type_actuals: &[TypeTag],
        concretize: bool,
    ) -> Result<(Vec<ResourceKey>, Vec<ResourceKey>)> {
        let signers = vec![tx.sender()];
        let gas_currency = account_config::type_tag_for_currency_code(
            account_config::from_currency_code_string(tx.gas_currency_code())?,
        );
        let prologue_accesses = self.get_binded_summary(
            &account_config::constants::ACCOUNT_MODULE,
            SCRIPT_PROLOGUE_NAME,
            &signers,
            &serialize_values(&vec![
                MoveValue::U64(tx.sequence_number()),
                MoveValue::vector_u8(tx.authenticator().sender().public_key_bytes()),
                MoveValue::U64(tx.gas_unit_price()),
                MoveValue::U64(tx.max_gas_amount()),
                MoveValue::U64(tx.expiration_timestamp_secs()),
                MoveValue::U8(tx.chain_id().id()),
                MoveValue::vector_u8(vec![]), // script_hash; it's ignored
            ]),
            &[gas_currency.clone()],
            &self.module_cache,
        )?;

        let epilogue_accesses = self.get_binded_summary(
            &account_config::constants::ACCOUNT_MODULE,
            USER_EPILOGUE_NAME,
            &signers,
            &serialize_values(&vec![
                MoveValue::U64(tx.sequence_number()),
                MoveValue::U64(tx.gas_unit_price()),
                MoveValue::U64(tx.max_gas_amount()),
                MoveValue::U64(0), // gas_units_remaining
            ]),
            &[gas_currency.clone()],
            &self.module_cache,
        )?;
        let script_accesses = self.get_binded_summary(
            module_name,
            script_name,
            &signers,
            actuals,
            type_actuals,
            &self.module_cache,
        )?;

        let mut keys_read = vec![];
        let mut keys_written = vec![];

        for accesses in vec![prologue_accesses, script_accesses, epilogue_accesses] {
            let (reads, writes) = self.concretize_binded_accesses(accesses, concretize)?;
            keys_read.extend(reads);
            keys_written.extend(writes);
        }

        keys_read.sort();
        keys_read.dedup();

        keys_written.sort();
        keys_written.dedup();
        // Hack: remove GasFees accesses from epilogue if gas_price is zero. This is sound
        // to do as of Diem 1.4, but should be re-evaluated if the epilogue changes
        if tx.gas_unit_price() == 0 {
            let tx_fees_tag = StructTag {
                address: account_config::CORE_CODE_ADDRESS,
                module: TRANSACTION_FEES_NAME.to_owned(),
                name: TRANSACTION_FEES_NAME.to_owned(),
                type_params: vec![gas_currency],
            };
            keys_written.retain(|r| r.type_() != &tx_fees_tag);
        }

        Ok((keys_read, keys_written))
    }
}

impl<'a, R: MoveResolver> Deref for ReadWriteSetAnalysis<'a, R> {
    type Target = NormalizedReadWriteSetAnalysis;

    fn deref(&self) -> &Self::Target {
        self.normalized_analysis_result
    }
}
