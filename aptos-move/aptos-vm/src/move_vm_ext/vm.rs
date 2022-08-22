// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{MoveResolverExt, SessionExt, SessionId},
    natives::aptos_natives,
};
use aptos_gas::{AbstractValueSizeGasParameters, NativeGasParameters};
use framework::natives::{
    aggregator_natives::NativeAggregatorContext, code::NativeCodeContext,
    cryptography::ristretto255_point::NativeRistrettoPointContext,
    state_storage::NativeStateStorageContext, transaction_context::NativeTransactionContext,
};
use move_deps::{
    move_binary_format::errors::VMResult,
    move_bytecode_verifier::VerifierConfig,
    move_table_extension::NativeTableContext,
    move_vm_runtime::{move_vm::MoveVM, native_extensions::NativeContextExtensions},
};
use std::ops::Deref;

pub struct MoveVmExt {
    inner: MoveVM,
}

impl MoveVmExt {
    pub fn new(
        native_gas_params: NativeGasParameters,
        abs_val_size_gas_params: AbstractValueSizeGasParameters,
    ) -> VMResult<Self> {
        Ok(Self {
            inner: MoveVM::new_with_verifier_config(
                aptos_natives(native_gas_params, abs_val_size_gas_params),
                VerifierConfig {
                    max_loop_depth: Some(5),
                },
            )?,
        })
    }

    pub fn new_session<'r, S: MoveResolverExt>(
        &self,
        remote: &'r S,
        session_id: SessionId,
    ) -> SessionExt<'r, '_, S> {
        let mut extensions = NativeContextExtensions::default();
        extensions.add(NativeTableContext::new(session_id.as_uuid(), remote));
        extensions.add(NativeRistrettoPointContext::new());
        extensions.add(NativeAggregatorContext::new(session_id.as_uuid(), remote));

        let script_hash = match session_id {
            SessionId::Txn {
                sender: _,
                sequence_number: _,
                script_hash,
            } => script_hash,
            _ => vec![],
        };
        extensions.add(NativeTransactionContext::new(script_hash));
        extensions.add(NativeCodeContext::default());
        extensions.add(NativeStateStorageContext::new(remote));

        // The VM code loader has bugs around module upgrade. After a module upgrade, the internal
        // cache needs to be flushed to work around those bugs.
        self.inner.flush_loader_cache_if_invalidated();

        SessionExt::new(self.inner.new_session_with_extensions(remote, extensions))
    }
}

impl Deref for MoveVmExt {
    type Target = MoveVM;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
