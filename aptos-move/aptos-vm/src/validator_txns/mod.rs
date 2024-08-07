// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{AptosMoveResolver, SessionId},
    AptosVM,
};
use aptos_types::validator_txn::ValidatorTransaction;
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::output::VMOutput;
use move_core_types::vm_status::VMStatus;

impl AptosVM {
    pub(crate) fn process_validator_transaction(
        &self,
        resolver: &impl AptosMoveResolver,
        txn: ValidatorTransaction,
        log_context: &AdapterLogSchema,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        let session_id = SessionId::validator_txn(&txn);
        match txn {
            ValidatorTransaction::DKGResult(dkg_node) => {
                self.process_dkg_result(resolver, log_context, session_id, dkg_node)
            },
            ValidatorTransaction::ObservedJWKUpdate(jwk_update) => {
                self.process_jwk_update(resolver, log_context, session_id, jwk_update)
            },
            ValidatorTransaction::MPCReconfigWorkDone(reconfig_work_result) => {
                self.process_mpc_reconfig_work_done(resolver, log_context, session_id, reconfig_work_result)
            }
            ValidatorTransaction::MPCUserRequestDone(task_result) => {
                self.process_mpc_user_request_done(resolver, log_context, session_id, task_result)
            }
        }
    }
}

mod dkg;
mod jwk;
mod mpc;
