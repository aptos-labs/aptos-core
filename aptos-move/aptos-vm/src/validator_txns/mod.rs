// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    move_vm_ext::{AptosMoveResolver, SessionId},
    AptosVM,
};
use aptos_types::validator_txn::ValidatorTransaction;
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::{
    module_and_script_storage::module_storage::AptosModuleStorage, output::VMOutput,
};
use move_core_types::vm_status::VMStatus;

impl AptosVM {
    pub(crate) fn process_validator_transaction(
        &self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        txn: ValidatorTransaction,
        log_context: &AdapterLogSchema,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        let session_id = SessionId::validator_txn(&txn);
        match txn {
            ValidatorTransaction::DKGResult(dkg_node) => {
                self.process_dkg_result(resolver, module_storage, log_context, session_id, dkg_node)
            },
            ValidatorTransaction::ObservedJWKUpdate(jwk_update) => self.process_jwk_update(
                resolver,
                module_storage,
                log_context,
                session_id,
                jwk_update,
            ),
            ValidatorTransaction::ChunkyDKGResult(subtranscript) => self.process_chunky_dkg_result(
                resolver,
                module_storage,
                log_context,
                session_id,
                subtranscript,
            ),
        }
    }
}

mod chunky_dkg;
mod dkg;
mod jwk;
