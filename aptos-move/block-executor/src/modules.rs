// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::view::{LatestView, ViewState};
use aptos_mvhashmap::types::{MVModulesError, MVModulesOutput};
use aptos_types::{
    executable::{Executable, ModulePath},
    state_store::TStateView,
    transaction::BlockExecutableTransaction as Transaction,
    vm::modules::OnChainUnverifiedModule,
};
use move_binary_format::errors::PartialVMResult;

impl<'a, T: Transaction, S: TStateView<Key = T::Key>, X: Executable> LatestView<'a, T, S, X> {
    pub(crate) fn get_module_state_value_impl(
        &self,
        key: &T::Key,
    ) -> PartialVMResult<Option<OnChainUnverifiedModule>> {
        debug_assert!(
            key.is_module_path(),
            "Expected to read a module, but is reading {:?} instead",
            key,
        );

        match &self.latest_view {
            ViewState::Sync(state) => {
                state
                    .captured_reads
                    .borrow_mut()
                    .module_reads
                    .push(key.clone());

                use MVModulesError::*;
                use MVModulesOutput::*;

                let modules = state.versioned_map.modules();
                match modules.fetch_module(key, self.txn_idx) {
                    Ok(Executable(_)) => unreachable!("Versioned executable not implemented"),
                    Ok(Module((v, _))) => Ok(Some(v)),
                    Err(Dependency(_)) => {
                        // Return anything (e.g. module does not exist) to avoid waiting,
                        // because parallel execution will fall back to sequential anyway.
                        Ok(None)
                    },
                    Err(NotFound) => match self.get_base_on_chain_module(key)? {
                        Some(m) => {
                            state
                                .versioned_map
                                .modules()
                                .set_base(key.clone(), m.clone());
                            Ok(Some(m))
                        },
                        None => Ok(None),
                    },
                }
            },
            ViewState::Unsync(state) => {
                state.read_set.borrow_mut().module_reads.insert(key.clone());
                match state.unsync_map.fetch_module_data(key) {
                    Some(m) => Ok(Some(m)),
                    None => match self.get_base_on_chain_module(key)? {
                        Some(m) => {
                            state.unsync_map.write_module(key.clone(), m.clone());
                            Ok(Some(m))
                        },
                        None => Ok(None),
                    },
                }
            },
        }
    }

    fn get_base_on_chain_module(
        &self,
        key: &T::Key,
    ) -> PartialVMResult<Option<OnChainUnverifiedModule>> {
        Ok(match self.get_raw_base_value(key)? {
            Some(state_value) => Some(OnChainUnverifiedModule::from_state_value(state_value)?),
            None => None,
        })
    }
}
