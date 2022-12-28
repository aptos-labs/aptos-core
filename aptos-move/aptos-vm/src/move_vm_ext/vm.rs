// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{MoveResolverExt, SessionExt, SessionId},
    natives::aptos_natives,
};
use aptos_framework::natives::{
    aggregator_natives::NativeAggregatorContext, code::NativeCodeContext,
    cryptography::ristretto255_point::NativeRistrettoPointContext,
    state_storage::NativeStateStorageContext, transaction_context::NativeTransactionContext,
};
use aptos_gas::{AbstractValueSizeGasParameters, NativeGasParameters};
use move_binary_format::errors::VMResult;
use move_bytecode_verifier::VerifierConfig;
use move_table_extension::NativeTableContext;
use move_vm_runtime::{
    config::VMConfig, move_vm::MoveVM, native_extensions::NativeContextExtensions,
};
use std::ops::Deref;

pub struct MoveVmExt {
    inner: MoveVM,
    chain_id: u8,
}

impl MoveVmExt {
    pub fn new(
        native_gas_params: NativeGasParameters,
        abs_val_size_gas_params: AbstractValueSizeGasParameters,
        gas_feature_version: u64,
        treat_friend_as_private: bool,
        allow_binary_format_v6: bool,
        chain_id: u8,
    ) -> VMResult<Self> {
        // Note: binary format v6 adds a few new integer types and their corresponding instructions.
        //       Therefore it depends on a new version of the gas schedule and cannot be allowed if
        //       the gas schedule hasn't been updated yet.
        let max_binary_format_version = if allow_binary_format_v6 && gas_feature_version >= 5 {
            6
        } else {
            5
        };

        Ok(Self {
            inner: MoveVM::new_with_config(
                aptos_natives(
                    native_gas_params,
                    abs_val_size_gas_params,
                    gas_feature_version,
                ),
                VMConfig {
                    verifier: verifier_config(treat_friend_as_private),
                    max_binary_format_version,
                    paranoid_type_checks: crate::AptosVM::get_paranoid_checks(),
                },
            )?,
            chain_id,
        })
    }

    pub fn new_session<'r, S: MoveResolverExt>(
        &self,
        remote: &'r S,
        session_id: SessionId,
    ) -> SessionExt<'r, '_, S> {
        let mut extensions = NativeContextExtensions::default();
        let txn_hash: [u8; 32] = session_id
            .as_uuid()
            .to_vec()
            .try_into()
            .expect("HashValue should convert to [u8; 32]");

        extensions.add(NativeTableContext::new(txn_hash, remote));
        extensions.add(NativeRistrettoPointContext::new());
        extensions.add(NativeAggregatorContext::new(txn_hash, remote));

        let script_hash = match session_id {
            SessionId::Txn {
                sender: _,
                sequence_number: _,
                script_hash,
            } => script_hash,
            _ => vec![],
        };

        extensions.add(NativeTransactionContext::new(script_hash, self.chain_id));
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

pub fn verifier_config(treat_friend_as_private: bool) -> VerifierConfig {
    VerifierConfig {
        max_loop_depth: Some(5),
        treat_friend_as_private,
        max_generic_instantiation_length: Some(32),
        max_function_parameters: Some(128),
        max_basic_blocks: Some(1024),
        max_value_stack_size: 1024,
        max_type_nodes: Some(256),
        max_dependency_depth: 256,
        max_push_size: Some(10000),
    }
}
