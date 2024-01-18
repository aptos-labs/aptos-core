// Copyright © Aptos Foundation

use crate::{
    move_vm_ext::{AptosMoveResolver, SessionId},
    AptosVM,
};
use aptos_types::{
    transaction::{ExecutionStatus, TransactionStatus},
    validator_txn::DummyValidatorTransaction,
};
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::output::VMOutput;
use move_core_types::vm_status::{AbortLocation, StatusCode, VMStatus};

impl AptosVM {
    pub(crate) fn process_dummy_validator_txn(
        &self,
        _resolver: &impl AptosMoveResolver,
        _log_context: &AdapterLogSchema,
        _session_id: SessionId,
        dummy_vtxn: DummyValidatorTransaction,
    ) -> anyhow::Result<(VMStatus, VMOutput), VMStatus> {
        let DummyValidatorTransaction { valid, .. } = dummy_vtxn;
        if valid {
            Ok((
                VMStatus::Executed,
                VMOutput::empty_with_status(TransactionStatus::Keep(ExecutionStatus::Success)),
            ))
        } else {
            Ok((
                VMStatus::MoveAbort(AbortLocation::Script, 0),
                VMOutput::empty_with_status(TransactionStatus::Discard(
                    StatusCode::INVALID_SIGNATURE,
                )),
            ))
        }
    }
}
