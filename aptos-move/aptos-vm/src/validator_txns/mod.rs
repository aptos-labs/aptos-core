// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{move_vm_ext::SessionId, AptosVM};
use aptos_types::validator_txn::ValidatorTransaction;
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::{
    module_and_script_storage::code_storage::AptosCodeStorage, output::VMOutput,
    resolver::ExecutorView,
};
use move_core_types::vm_status::VMStatus;

impl AptosVM {
    pub(crate) fn process_validator_transaction(
        &self,
        executor_view: &impl ExecutorView,
        module_storage: &impl AptosCodeStorage,
        txn: ValidatorTransaction,
        log_context: &AdapterLogSchema,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        let session_id = SessionId::validator_txn(&txn);
        match txn {
            ValidatorTransaction::DKGResult(dkg_node) => self.process_dkg_result(
                executor_view,
                module_storage,
                log_context,
                session_id,
                dkg_node,
            ),
            ValidatorTransaction::ObservedJWKUpdate(jwk_update) => self.process_jwk_update(
                executor_view,
                module_storage,
                log_context,
                session_id,
                jwk_update,
            ),
        }
    }
}

mod dkg;
mod jwk;
