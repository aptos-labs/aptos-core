// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    adapter_common::PreprocessedTransaction,
    move_vm_ext::MoveResolverExt,
    system_module_names::{BLOCK_MODULE, BLOCK_PROLOGUE, SCRIPT_PROLOGUE_NAME, USER_EPILOGUE_NAME},
};
use anyhow::{anyhow, bail, Result};
use aptos_types::{
    account_config,
    transaction::{SignedTransaction, TransactionPayload},
};
use move_deps::{
    move_bytecode_utils::module_cache::SyncModuleCache,
    move_core_types::{
        ident_str,
        identifier::{IdentStr, Identifier},
        language_storage::{ModuleId, ResourceKey, StructTag, TypeTag},
        resolver::ModuleResolver,
        value::{serialize_values, MoveValue},
    },
    read_write_set_dynamic::{ConcretizedFormals, NormalizedReadWriteSetAnalysis},
};
use once_cell::sync::Lazy;
use std::ops::Deref;

pub struct ReadWriteSetAnalysis<'a, R: ModuleResolver> {
    normalized_analysis_result: &'a NormalizedReadWriteSetAnalysis,
    module_cache: SyncModuleCache<&'a R>,
    blockchain_view: &'a R,
}

const TRANSACTION_VALIDATION_MODULE_NAME: &IdentStr = ident_str!("transaction_validation");
static TRANSACTION_VALIDATION: Lazy<ModuleId> = Lazy::new(|| {
    ModuleId::new(
        account_config::CORE_CODE_ADDRESS,
        TRANSACTION_VALIDATION_MODULE_NAME.to_owned(),
    )
});

const TRANSACTION_FEES_MODULE_NAME: &IdentStr = ident_str!("transaction_fee");
const TRANSACTION_FEES_NAME: &IdentStr = ident_str!("TransactionFee");

pub fn add_on_functions_list() -> Vec<(ModuleId, Identifier)> {
    vec![
        (BLOCK_MODULE.clone(), BLOCK_PROLOGUE.to_owned()),
        (
            TRANSACTION_VALIDATION.clone(),
            SCRIPT_PROLOGUE_NAME.to_owned(),
        ),
        (
            TRANSACTION_VALIDATION.clone(),
            USER_EPILOGUE_NAME.to_owned(),
        ),
    ]
}

impl<'a, R: MoveResolverExt> ReadWriteSetAnalysis<'a, R> {
    /// Create a Aptos transaction read/write set analysis from a generic Move module read/write set
    /// analysis and a view of the current blockchain for module fetching and access concretization.
    pub fn new(rw: &'a NormalizedReadWriteSetAnalysis, blockchain_view: &'a R) -> Self {
        ReadWriteSetAnalysis {
            normalized_analysis_result: rw,
            module_cache: SyncModuleCache::new(blockchain_view),
            blockchain_view,
        }
    }

    /// Returns an overapproximation of the `ResourceKey`'s in global storage that will be written
    /// by `tx`
    pub fn get_keys_written(&self, tx: &SignedTransaction) -> Result<Vec<ResourceKey>> {
        Ok(self.get_keys_user_transaction(tx)?.1)
    }

    /// Returns an overapproximation of the `ResourceKey`'s in global storage that will be read
    /// by `tx`
    pub fn get_keys_read(&self, tx: &SignedTransaction) -> Result<Vec<ResourceKey>> {
        Ok(self.get_keys_user_transaction(tx)?.0)
    }

    /// Returns an overapproximation of the `ResourceKey`'s in global storage that will be read
    /// by `tx`. Secondary indexes will be fully resolved by using the global state at
    /// `self.blockchain_view`.
    ///
    /// Return value will be a tuple where the first item is the read set and the second
    /// item is the write set of this transaction.
    ///
    /// Note: this will return both writes performed by the transaction prologue/epilogue and by its
    /// embedded payload.
    pub fn get_keys_user_transaction(
        &self,
        tx: &SignedTransaction,
    ) -> Result<(Vec<ResourceKey>, Vec<ResourceKey>)> {
        self.get_keys_user_transaction_impl(tx, true)
    }

    /// Returns an overapproximation of the `ResourceKey`'s in global storage that will be read
    /// by `tx`. Only formals and type arguments will be binded and secondary indexes will remain to
    /// be unresolved for better speed performance.
    ///
    /// Note: this will return both writes performed by the transaction prologue/epilogue and by its
    /// embedded payload.
    pub fn get_partial_keys_user_transaction(
        &self,
        tx: &SignedTransaction,
    ) -> Result<(Vec<ResourceKey>, Vec<ResourceKey>)> {
        self.get_keys_user_transaction_impl(tx, false)
    }

    fn get_keys_user_transaction_impl(
        &self,
        tx: &SignedTransaction,
        concretize: bool,
    ) -> Result<(Vec<ResourceKey>, Vec<ResourceKey>)> {
        match tx.payload() {
            TransactionPayload::EntryFunction(s) => self.get_concretized_keys_entry_function(
                tx,
                s.module(),
                s.function(),
                s.args(),
                s.ty_args(),
                concretize,
            ),
            TransactionPayload::Script(s) => {
                bail!("Unsupported transaction script type {:?}", s)
            }
            payload => {
                // TODO: support tx scripts here. Slightly tricky since we will need to run
                // analyzer on the fly
                bail!("Unsupported transaction payload type {:?}", payload)
            }
        }
    }

    fn concretize_secondary_indexes(
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

    #[allow(dead_code)]
    /// Internal API to get the read/write set of `PreprocessedTransaction`.
    pub(crate) fn get_keys_transaction(
        &self,
        tx: &PreprocessedTransaction,
        concretize: bool,
    ) -> Result<(Vec<ResourceKey>, Vec<ResourceKey>)> {
        match tx {
            PreprocessedTransaction::UserTransaction(tx) => {
                self.get_keys_user_transaction_impl(tx, concretize)
            }
            PreprocessedTransaction::BlockMetadata(block_metadata) => {
                let args = serialize_values(
                    &block_metadata
                        .clone()
                        .get_prologue_move_args(account_config::reserved_vm_address()),
                );
                let metadata_access = self.get_partially_concretized_summary(
                    &BLOCK_MODULE,
                    BLOCK_PROLOGUE,
                    &[],
                    &args,
                    &[],
                    &self.module_cache,
                )?;
                self.concretize_secondary_indexes(metadata_access, concretize)
            }
            PreprocessedTransaction::InvalidSignature => Ok((vec![], vec![])),
            PreprocessedTransaction::StateCheckpoint => Ok((vec![], vec![])),
            PreprocessedTransaction::WaypointWriteSet(_) => {
                bail!("Unsupported writeset transaction")
            }
        }
    }

    fn get_concretized_keys_entry_function(
        &self,
        tx: &SignedTransaction,
        module_name: &ModuleId,
        script_name: &IdentStr,
        actuals: &[Vec<u8>],
        type_actuals: &[TypeTag],
        concretize: bool,
    ) -> Result<(Vec<ResourceKey>, Vec<ResourceKey>)> {
        let signers = vec![tx.sender()];
        let prologue_accesses = self.get_partially_concretized_summary(
            &TRANSACTION_VALIDATION,
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
            &[],
            &self.module_cache,
        )?;

        let epilogue_accesses = self.get_partially_concretized_summary(
            &TRANSACTION_VALIDATION,
            USER_EPILOGUE_NAME,
            &signers,
            &serialize_values(&vec![
                MoveValue::U64(tx.sequence_number()),
                MoveValue::U64(tx.gas_unit_price()),
                MoveValue::U64(tx.max_gas_amount()),
                MoveValue::U64(0), // gas_units_remaining
            ]),
            &[],
            &self.module_cache,
        )?;

        // If any error occurs while analyzing the body, skip the read/writeset result as only the
        // prologue and epilogue will be run by this transaction.
        let script_accesses = self
            .get_partially_concretized_summary(
                module_name,
                script_name,
                &signers,
                actuals,
                type_actuals,
                &self.module_cache,
            )
            .unwrap_or_else(|_| ConcretizedFormals::empty());

        let mut keys_read = vec![];
        let mut keys_written = vec![];

        for accesses in vec![prologue_accesses, script_accesses, epilogue_accesses] {
            let (reads, writes) = self.concretize_secondary_indexes(accesses, concretize)?;
            keys_read.extend(reads);
            keys_written.extend(writes);
        }

        keys_read.sort();
        keys_read.dedup();

        keys_written.sort();
        keys_written.dedup();
        // Hack: remove GasFees accesses from epilogue if gas_price is zero. This is sound
        // to do as of Aptos 1.4, but should be re-evaluated if the epilogue changes
        if tx.gas_unit_price() == 0 {
            let tx_fees_tag = StructTag {
                address: account_config::CORE_CODE_ADDRESS,
                module: TRANSACTION_FEES_MODULE_NAME.to_owned(),
                name: TRANSACTION_FEES_NAME.to_owned(),
                type_params: vec![],
            };
            keys_written.retain(|r| r.type_() != &tx_fees_tag);
        }

        Ok((keys_read, keys_written))
    }
}

impl<'a, R: MoveResolverExt> Deref for ReadWriteSetAnalysis<'a, R> {
    type Target = NormalizedReadWriteSetAnalysis;

    fn deref(&self) -> &Self::Target {
        self.normalized_analysis_result
    }
}
