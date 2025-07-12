// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implements transaction epilogue, which is called when
//!   1. User payload is executed successfully (success epilogue).
//!   2. User payload or success epilogue fail (failure epilogue).

use crate::{
    aptos_vm::SerializedSigners,
    move_vm_ext::{AptosMoveResolver, SessionId},
    system_module_names::{EMIT_FEE_STATEMENT, TRANSACTION_FEE_MODULE},
    transaction_validation::{epilogue_serialized_args, APTOS_TRANSACTION_VALIDATION},
    v2::session::{gas_used, UserTransactionSession},
};
use aptos_gas_meter::AptosGasMeter;
use aptos_logger::error;
use aptos_types::{
    fee_statement::FeeStatement,
    transaction::{ExecutionStatus, TransactionStatus},
};
use aptos_vm_types::{module_and_script_storage::code_storage::AptosCodeStorage, output::VMOutput};
use move_core_types::vm_status::VMStatus;

impl<'a, DataView, CodeView> UserTransactionSession<'a, DataView, CodeView>
where
    DataView: AptosMoveResolver,
    CodeView: AptosCodeStorage,
{
    /// Called after the user transaction payload is successfully executed.
    pub(crate) fn execute_user_transaction_success_epilogue(
        &mut self,
        gas_meter: &mut impl AptosGasMeter,
        serialized_signers: &SerializedSigners,
    ) -> Result<VMOutput, VMStatus> {
        // Check if the gas meter's internal counters are consistent.
        //
        // It is better to fail the transaction here early rather than to allow potentially wrong
        // states to be committed.
        gas_meter.check_consistency().inspect_err(|err| {
            error!(
                "[aptos-vm] Inconsistency found in gas meter (success epilogue): {}",
                err
            );
        })?;

        self.run_epilogue_and_emit_fee_statement(gas_meter, serialized_signers)?;
        self.session.materialize_output(ExecutionStatus::Success)
    }

    /// Called when user transaction payload failed execution or the success epilogue failed.
    pub(crate) fn execute_user_transaction_failure_epilogue(
        &mut self,
        gas_meter: &mut impl AptosGasMeter,
        serialized_signers: &SerializedSigners,
        status: VMStatus,
    ) -> Result<VMOutput, VMStatus> {
        if let Err(err) = gas_meter.check_consistency() {
            error!(
                "[aptos-vm] Inconsistency found in gas meter (failure epilogue): {}",
                err
            );
        };

        let txn_status = TransactionStatus::from_vm_status(status, self.features());
        let execution_status = match txn_status {
            TransactionStatus::Keep(execution_status) => {
                self.inject_abort_info_if_available(execution_status)
            },
            TransactionStatus::Discard(status_code) => {
                return Ok(VMOutput::discarded(status_code));
            },
            TransactionStatus::Retry => {
                unreachable!("Transaction status constructed from VM status cannot be retry")
            },
        };

        self.update_extensions(SessionId::epilogue_meta(&self.txn_metadata));
        self.run_epilogue_and_emit_fee_statement(gas_meter, serialized_signers)?;
        self.session.materialize_output(execution_status)
    }
}

// Private interfaces.
impl<'a, DataView, CodeView> UserTransactionSession<'a, DataView, CodeView>
where
    DataView: AptosMoveResolver,
    CodeView: AptosCodeStorage,
{
    /// Runs epilogue for (un)successfully executed transaction:
    ///   1. Extracts fee statement.
    ///   2. Executes epilogue function that tries to charge gas and update the state.
    ///   3. Emits fee statement event.
    fn run_epilogue_and_emit_fee_statement(
        &mut self,
        gas_meter: &impl AptosGasMeter,
        serialized_signers: &SerializedSigners,
    ) -> Result<FeeStatement, VMStatus> {
        let fee_statement = self.fee_statement_from_gas_meter(gas_meter);

        let (function_name, args) = epilogue_serialized_args(
            &self.txn_metadata,
            self.features(),
            serialized_signers,
            &fee_statement,
            gas_meter.balance(),
            self.is_simulation,
        );

        self.session.execute_unmetered_system_function(
            &APTOS_TRANSACTION_VALIDATION.module_id(),
            function_name,
            args,
        )?;

        self.emit_fee_statement(fee_statement)?;
        Ok(fee_statement)
    }

    /// Emits fee statement event on-chain.
    fn emit_fee_statement(&mut self, fee_statement: FeeStatement) -> Result<(), VMStatus> {
        self.session.execute_unmetered_system_function(
            &TRANSACTION_FEE_MODULE,
            EMIT_FEE_STATEMENT,
            vec![bcs::to_bytes(&fee_statement).expect("Failed to serialize fee statement")],
        )?;
        Ok(())
    }

    /// Returns the fee statement based on the gas used by the meter and the currently accumulated
    /// storage refund.
    fn fee_statement_from_gas_meter(&self, gas_meter: &impl AptosGasMeter) -> FeeStatement {
        let gas_used = gas_used(self.txn_metadata.max_gas_amount(), gas_meter);
        FeeStatement::new(
            gas_used,
            u64::from(gas_meter.execution_gas_used()),
            u64::from(gas_meter.io_gas_used()),
            u64::from(gas_meter.storage_fee_used()),
            u64::from(self.storage_refund),
        )
    }
}
