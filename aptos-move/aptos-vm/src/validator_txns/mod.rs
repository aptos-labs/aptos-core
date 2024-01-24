// Copyright © Aptos Foundation

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
            ValidatorTransaction::DummyTopic1(dummy) | ValidatorTransaction::DummyTopic2(dummy) => {
                self.process_dummy_validator_txn(resolver, log_context, session_id, dummy)
            },
        }
    }
}

mod dkg;
mod dummy;
