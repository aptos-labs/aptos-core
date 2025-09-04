// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{VelorMoveResolver, SessionId},
    VelorVM,
};
use velor_types::validator_txn::ValidatorTransaction;
use velor_vm_logging::log_schema::AdapterLogSchema;
use velor_vm_types::{
    module_and_script_storage::module_storage::VelorModuleStorage, output::VMOutput,
};
use move_core_types::vm_status::VMStatus;

impl VelorVM {
    pub(crate) fn process_validator_transaction(
        &self,
        resolver: &impl VelorMoveResolver,
        module_storage: &impl VelorModuleStorage,
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
        }
    }
}

mod dkg;
mod jwk;
