// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::log::{
    CallFrame, Dependency, EventStorage, EventTransient, ExecutionAndIOCosts, ExecutionGasEvent,
    FrameName, StorageFees, TransactionGasLog, WriteOpType, WriteStorage, WriteTransient,
};
use aptos_gas_algebra::{Fee, FeePerGasUnit, InternalGas, NumArgs, NumBytes, NumTypeNodes};
use aptos_gas_meter::{AptosGasMeter, GasAlgebra};
use aptos_gas_schedule::gas_feature_versions::RELEASE_V1_30;
use aptos_types::{
    contract_event::ContractEvent, state_store::state_key::StateKey, write_set::WriteOpSize,
};
use aptos_vm_types::{
    change_set::ChangeSetInterface, module_and_script_storage::module_storage::AptosModuleStorage,
    resolver::ExecutorView, storage::space_pricing::ChargeAndRefund,
};
use move_binary_format::{
    errors::{Location, PartialVMResult, VMResult},
    file_format::CodeOffset,
    file_format_common::Opcodes,
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, TypeTag},
};
use move_vm_types::{
    gas::{DependencyGasMeter, DependencyKind, GasMeter, NativeGasMeter, SimpleInstruction},
    views::{TypeView, ValueView},
};

/// A special gas meter adapter that records all gas-related events, along with the associated costs
/// assessed by the underlying gas meter.
pub struct GasProfiler<G> {
    base: G,

    intrinsic_cost: Option<InternalGas>,
    keyless_cost: Option<InternalGas>,
    dependencies: Vec<Dependency>,
    frames: Vec<CallFrame>,
    transaction_transient: Option<InternalGas>,
    events_transient: Vec<EventTransient>,
    write_set_transient: Vec<WriteTransient>,
    storage_fees: Option<StorageFees>,
}

// TODO: consider switching to a library like https://docs.rs/delegate/latest/delegate/.
macro_rules! delegate {
    ($(
        fn $fn: ident $(<$($lt: lifetime),*>)? (&self $(, $arg: ident : $ty: ty)* $(,)?) -> $ret_ty: ty;
    )*) => {
        $(fn $fn $(<$($lt)*>)? (&self, $($arg: $ty),*) -> $ret_ty {
            self.base.$fn($($arg),*)
        })*
    };
}

macro_rules! delegate_mut {
    ($(
        fn $fn: ident $(<$($lt: lifetime),*>)? (&mut self $(, $arg: ident : $ty: ty)* $(,)?) -> $ret_ty: ty;
    )*) => {
        $(fn $fn $(<$($lt)*>)? (&mut self, $($arg: $ty),*) -> $ret_ty {
            self.base.$fn($($arg),*)
        })*
    };
}

macro_rules! record_bytecode {
    ($(
        $([$op: expr])?
        fn $fn: ident $(<$($lt: lifetime),*>)? (&mut self $(, $arg: ident : $ty: ty)* $(,)?) -> PartialVMResult<()>;
    )*) => {
        $(fn $fn $(<$($lt)*>)? (&mut self, $($arg: $ty),*) -> PartialVMResult<()> {
            #[allow(unused)]
            use Opcodes::*;

            #[allow(unused)]
            let (cost, res) = self.delegate_charge(|base| base.$fn($($arg),*));

            $(
                self.record_bytecode($op, cost);
            )?

            res
        })*
    };
}

impl<G> GasProfiler<G> {
    pub fn new_script(base: G) -> Self {
        Self {
            base,

            intrinsic_cost: None,
            keyless_cost: None,
            dependencies: vec![],
            frames: vec![CallFrame::new_script()],
            transaction_transient: None,
            events_transient: vec![],
            write_set_transient: vec![],
            storage_fees: None,
        }
    }

    pub fn new_function(
        base: G,
        module_id: ModuleId,
        func_name: Identifier,
        ty_args: Vec<TypeTag>,
    ) -> Self {
        Self {
            base,

            intrinsic_cost: None,
            keyless_cost: None,
            dependencies: vec![],
            frames: vec![CallFrame::new_function(module_id, func_name, ty_args)],
            transaction_transient: None,
            events_transient: vec![],
            write_set_transient: vec![],
            storage_fees: None,
        }
    }
}

impl<G> GasProfiler<G>
where
    G: AptosGasMeter,
{
    fn active_event_stream(&mut self) -> &mut Vec<ExecutionGasEvent> {
        &mut self.frames.last_mut().unwrap().events
    }

    fn record_gas_event(&mut self, event: ExecutionGasEvent) {
        self.active_event_stream().push(event);
    }

    fn record_bytecode(&mut self, op: Opcodes, cost: InternalGas) {
        self.record_gas_event(ExecutionGasEvent::Bytecode { op, cost })
    }

    fn record_offset(&mut self, offset: CodeOffset) {
        self.record_gas_event(ExecutionGasEvent::Loc(offset))
    }

    /// Delegate the charging call to the base gas meter and measure variation in balance.
    fn delegate_charge<F, R>(&mut self, charge: F) -> (InternalGas, R)
    where
        F: FnOnce(&mut G) -> R,
    {
        let old = self.base.balance_internal();
        let res = charge(&mut self.base);
        let new = self.base.balance_internal();
        let cost = old.checked_sub(new).expect("gas cost must be non-negative");

        (cost, res)
    }
}

impl<G> DependencyGasMeter for GasProfiler<G>
where
    G: AptosGasMeter,
{
    fn charge_dependency(
        &mut self,
        kind: DependencyKind,
        addr: &AccountAddress,
        name: &IdentStr,
        size: NumBytes,
    ) -> PartialVMResult<()> {
        let (cost, res) =
            self.delegate_charge(|base| base.charge_dependency(kind, addr, name, size));

        if !cost.is_zero() {
            self.dependencies.push(Dependency {
                kind,
                id: ModuleId::new(*addr, name.to_owned()),
                size,
                cost,
            });
        }

        res
    }
}

impl<G> NativeGasMeter for GasProfiler<G>
where
    G: AptosGasMeter,
{
    delegate! {
        fn legacy_gas_budget_in_native_context(&self) -> InternalGas;
    }

    fn use_heap_memory_in_native_context(&mut self, amount: u64) -> PartialVMResult<()> {
        let (cost, res) =
            self.delegate_charge(|base| base.use_heap_memory_in_native_context(amount));
        assert_eq!(
            cost,
            0.into(),
            "Using heap memory does not incur any gas costs"
        );
        res
    }

    fn charge_native_execution(&mut self, amount: InternalGas) -> PartialVMResult<()> {
        self.frames
            .last_mut()
            .expect("Native function must have recorded the frame")
            .native_gas += amount;

        self.base.charge_native_execution(amount)
    }
}

impl<G> GasMeter for GasProfiler<G>
where
    G: AptosGasMeter,
{
    delegate_mut! {
        // Note: we only use this callback for memory tracking, not for charging gas.
        fn charge_ld_const_after_deserialization(&mut self, val: impl ValueView)
            -> PartialVMResult<()>;

        // Note: we don't use this to charge gas so no need to record anything.
        fn charge_native_function_before_execution(
            &mut self,
            ty_args: impl ExactSizeIterator<Item = impl TypeView> + Clone,
            args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
        ) -> PartialVMResult<()>;

        // Note: we don't use this to charge gas so no need to record anything.
        fn charge_drop_frame(
            &mut self,
            locals: impl Iterator<Item = impl ValueView> + Clone,
        ) -> PartialVMResult<()>;
    }

    record_bytecode! {
        [POP]
        fn charge_pop(&mut self, popped_val: impl ValueView) -> PartialVMResult<()>;

        [LD_CONST]
        fn charge_ld_const(&mut self, size: NumBytes) -> PartialVMResult<()>;

        [COPY_LOC]
        fn charge_copy_loc(&mut self, val: impl ValueView) -> PartialVMResult<()>;

        [MOVE_LOC]
        fn charge_move_loc(&mut self, val: impl ValueView) -> PartialVMResult<()>;

        [ST_LOC]
        fn charge_store_loc(&mut self, val: impl ValueView) -> PartialVMResult<()>;

        [PACK]
        fn charge_pack(
            &mut self,
            is_generic: bool,
            args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
        ) -> PartialVMResult<()>;

        [UNPACK]
        fn charge_unpack(
            &mut self,
            is_generic: bool,
            args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
        ) -> PartialVMResult<()>;

        [PACK_CLOSURE]
        fn charge_pack_closure(
            &mut self,
            is_generic: bool,
            args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
        ) -> PartialVMResult<()>;

        [READ_REF]
        fn charge_read_ref(&mut self, val: impl ValueView) -> PartialVMResult<()>;

        [WRITE_REF]
        fn charge_write_ref(
            &mut self,
            new_val: impl ValueView,
            old_val: impl ValueView,
        ) -> PartialVMResult<()>;

        [EQ]
        fn charge_eq(&mut self, lhs: impl ValueView, rhs: impl ValueView) -> PartialVMResult<()>;

        [NEQ]
        fn charge_neq(&mut self, lhs: impl ValueView, rhs: impl ValueView) -> PartialVMResult<()>;

        [
            match (is_mut, is_generic) {
                (false, false) => IMM_BORROW_GLOBAL,
                (false, true) => IMM_BORROW_GLOBAL_GENERIC,
                (true, false) => MUT_BORROW_GLOBAL,
                (true, true) => MUT_BORROW_GLOBAL_GENERIC
            }
        ]
        fn charge_borrow_global(
            &mut self,
            is_mut: bool,
            is_generic: bool,
            ty: impl TypeView,
            is_success: bool,
        ) -> PartialVMResult<()>;

        [if is_generic { EXISTS } else { EXISTS_GENERIC }]
        fn charge_exists(
            &mut self,
            is_generic: bool,
            ty: impl TypeView,
            exists: bool,
        ) -> PartialVMResult<()>;

        [if is_generic { MOVE_FROM } else { MOVE_FROM_GENERIC }]
        fn charge_move_from(
            &mut self,
            is_generic: bool,
            ty: impl TypeView,
            val: Option<impl ValueView>,
        ) -> PartialVMResult<()>;

        [if is_generic { MOVE_TO } else { MOVE_TO_GENERIC }]
        fn charge_move_to(
            &mut self,
            is_generic: bool,
            ty: impl TypeView,
            val: impl ValueView,
            is_success: bool,
        ) -> PartialVMResult<()>;

        [VEC_PACK]
        fn charge_vec_pack<'a>(
            &mut self,
            ty: impl TypeView + 'a,
            args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
        ) -> PartialVMResult<()>;

        [VEC_LEN]
        fn charge_vec_len(&mut self, ty: impl TypeView) -> PartialVMResult<()>;

        [VEC_IMM_BORROW]
        fn charge_vec_borrow(
            &mut self,
            is_mut: bool,
            ty: impl TypeView,
            is_success: bool,
        ) -> PartialVMResult<()>;

        [VEC_PUSH_BACK]
        fn charge_vec_push_back(
            &mut self,
            ty: impl TypeView,
            val: impl ValueView,
        ) -> PartialVMResult<()>;

        [VEC_POP_BACK]
        fn charge_vec_pop_back(
            &mut self,
            ty: impl TypeView,
            val: Option<impl ValueView>,
        ) -> PartialVMResult<()>;

        [VEC_UNPACK]
        fn charge_vec_unpack(
            &mut self,
            ty: impl TypeView,
            expect_num_elements: NumArgs,
            elems: impl ExactSizeIterator<Item = impl ValueView> + Clone,
        ) -> PartialVMResult<()>;

        [VEC_SWAP]
        fn charge_vec_swap(&mut self, ty: impl TypeView) -> PartialVMResult<()>;
    }

    fn balance_internal(&self) -> InternalGas {
        self.base.balance_internal()
    }

    fn charge_native_function(
        &mut self,
        amount: InternalGas,
        ret_vals: Option<impl ExactSizeIterator<Item = impl ValueView> + Clone>,
    ) -> PartialVMResult<()> {
        // Whenever a function gets called, the VM will notify the gas profiler
        // via `charge_call/charge_call_generic`.
        //
        // At this point of time, the gas profiler does not yet have an efficient way to determine
        // whether the function is a native or not, so it will blindly create a new frame.
        //
        // Later when it realizes the function is native, it will transform the original frame
        // into a native-specific event that does not contain recursive structures.
        let frame = self.frames.pop().expect("frame must exist");

        // Add native gas accumulated per frame.
        let (mut cost, res) =
            self.delegate_charge(|base| base.charge_native_function(amount, ret_vals));
        cost += frame.native_gas;

        let (module_id, name, ty_args) = match frame.name {
            FrameName::Function {
                module_id,
                name,
                ty_args,
            } => (module_id, name, ty_args),
            FrameName::Script => unreachable!(),
        };

        // The following line of code is needed for correctness.
        //
        // This is because additional gas events may be produced after the frame has been
        // created and these events need to be preserved.
        self.active_event_stream().extend(frame.events);

        self.record_gas_event(ExecutionGasEvent::CallNative {
            module_id,
            fn_name: name,
            ty_args,
            cost,
        });

        res
    }

    fn charge_br_false(&mut self, target_offset: Option<CodeOffset>) -> PartialVMResult<()> {
        let (cost, res) = self.delegate_charge(|base| base.charge_br_false(target_offset));

        self.record_bytecode(Opcodes::BR_FALSE, cost);
        if let Some(offset) = target_offset {
            self.record_offset(offset);
        }

        res
    }

    fn charge_br_true(&mut self, target_offset: Option<CodeOffset>) -> PartialVMResult<()> {
        let (cost, res) = self.delegate_charge(|base| base.charge_br_true(target_offset));

        self.record_bytecode(Opcodes::BR_TRUE, cost);
        if let Some(offset) = target_offset {
            self.record_offset(offset);
        }

        res
    }

    fn charge_branch(&mut self, target_offset: CodeOffset) -> PartialVMResult<()> {
        let (cost, res) = self.delegate_charge(|base| base.charge_branch(target_offset));

        self.record_bytecode(Opcodes::BRANCH, cost);
        self.record_offset(target_offset);

        res
    }

    fn charge_simple_instr(&mut self, instr: SimpleInstruction) -> PartialVMResult<()> {
        let (cost, res) = self.delegate_charge(|base| base.charge_simple_instr(instr));

        self.record_bytecode(instr.to_opcode(), cost);

        // TODO: Right now we keep the last frame on the stack even after hitting the ret instruction,
        //       so that it can be picked up by finishing procedure.
        //       This is a bit hacky and can lead to weird behaviors if the profiler is used
        //       over multiple transactions, but again, guarding against that case is a broader
        //       problem we can deal with in the future.
        if matches!(instr, SimpleInstruction::Ret) && self.frames.len() > 1 {
            let cur_frame = self.frames.pop().expect("frame must exist");
            let last_frame = self.frames.last_mut().expect("frame must exist");
            last_frame.events.push(ExecutionGasEvent::Call(cur_frame));
        }

        res
    }

    fn charge_call(
        &mut self,
        module_id: &ModuleId,
        func_name: &str,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
        num_locals: NumArgs,
    ) -> PartialVMResult<()> {
        let (cost, res) =
            self.delegate_charge(|base| base.charge_call(module_id, func_name, args, num_locals));

        self.record_bytecode(Opcodes::CALL, cost);
        self.frames.push(CallFrame::new_function(
            module_id.clone(),
            Identifier::new(func_name).unwrap(),
            vec![],
        ));

        res
    }

    fn charge_call_generic(
        &mut self,
        module_id: &ModuleId,
        func_name: &str,
        ty_args: impl ExactSizeIterator<Item = impl TypeView> + Clone,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
        num_locals: NumArgs,
    ) -> PartialVMResult<()> {
        let ty_tags = ty_args
            .clone()
            .map(|ty| ty.to_type_tag())
            .collect::<Vec<_>>();

        let (cost, res) = self.delegate_charge(|base| {
            base.charge_call_generic(module_id, func_name, ty_args, args, num_locals)
        });

        self.record_bytecode(Opcodes::CALL_GENERIC, cost);
        self.frames.push(CallFrame::new_function(
            module_id.clone(),
            Identifier::new(func_name).unwrap(),
            ty_tags,
        ));

        res
    }

    fn charge_load_resource(
        &mut self,
        addr: AccountAddress,
        ty: impl TypeView,
        val: Option<impl ValueView>,
        bytes_loaded: NumBytes,
    ) -> PartialVMResult<()> {
        let ty_tag = ty.to_type_tag();

        let (cost, res) =
            self.delegate_charge(|base| base.charge_load_resource(addr, ty, val, bytes_loaded));

        self.record_gas_event(ExecutionGasEvent::LoadResource {
            addr,
            ty: ty_tag,
            cost,
        });

        res
    }

    fn charge_create_ty(&mut self, num_nodes: NumTypeNodes) -> PartialVMResult<()> {
        let (cost, res) = self.delegate_charge(|base| base.charge_create_ty(num_nodes));

        self.record_gas_event(ExecutionGasEvent::CreateTy { cost });

        res
    }
}

fn write_op_type(op: &WriteOpSize) -> WriteOpType {
    use WriteOpSize as O;
    use WriteOpType as T;

    match op {
        O::Creation { .. } => T::Creation,
        O::Modification { .. } => T::Modification,
        O::Deletion => T::Deletion,
    }
}

impl<G> AptosGasMeter for GasProfiler<G>
where
    G: AptosGasMeter,
{
    type Algebra = G::Algebra;

    delegate! {
        fn algebra(&self) -> &Self::Algebra;
    }

    delegate_mut! {
        fn algebra_mut(&mut self) -> &mut Self::Algebra;

        fn charge_storage_fee(
            &mut self,
            amount: Fee,
            gas_unit_price: FeePerGasUnit,
        ) -> PartialVMResult<()>;
    }

    fn charge_io_gas_for_transaction(&mut self, txn_size: NumBytes) -> VMResult<()> {
        let (cost, res) = self.delegate_charge(|base| base.charge_io_gas_for_transaction(txn_size));

        self.transaction_transient = Some(cost);

        res
    }

    fn charge_io_gas_for_event(&mut self, event: &ContractEvent) -> VMResult<()> {
        let (cost, res) = self.delegate_charge(|base| base.charge_io_gas_for_event(event));

        self.events_transient.push(EventTransient {
            ty: event.type_tag().clone(),
            cost,
        });

        res
    }

    fn charge_io_gas_for_write(&mut self, key: &StateKey, op: &WriteOpSize) -> VMResult<()> {
        let (cost, res) = self.delegate_charge(|base| base.charge_io_gas_for_write(key, op));

        self.write_set_transient.push(WriteTransient {
            key: key.clone(),
            cost,
            op_type: write_op_type(op),
        });

        res
    }

    fn process_storage_fee_for_all(
        &mut self,
        change_set: &mut impl ChangeSetInterface,
        txn_size: NumBytes,
        gas_unit_price: FeePerGasUnit,
        executor_view: &dyn ExecutorView,
        module_storage: &impl AptosModuleStorage,
    ) -> VMResult<Fee> {
        // The new storage fee are only active since version 7.
        if self.feature_version() < 7 {
            return Ok(0.into());
        }

        // TODO(Gas): right now, some of our tests use a unit price of 0 and this is a hack
        // to avoid causing them issues. We should revisit the problem and figure out a
        // better way to handle this.
        if gas_unit_price.is_zero() {
            return Ok(0.into());
        }

        let pricing = self.disk_space_pricing();
        let params = &self.vm_gas_params().txn;

        // Write set
        let mut write_fee = Fee::new(0);
        let mut write_set_storage = vec![];
        let mut total_refund = Fee::new(0);
        let fix_prev_materialized_size = self.feature_version() > RELEASE_V1_30;
        for res in change_set.write_op_info_iter_mut(
            executor_view,
            module_storage,
            fix_prev_materialized_size,
        ) {
            let write_op_info = res.map_err(|err| err.finish(Location::Undefined))?;
            let key = write_op_info.key.clone();
            let op_type = write_op_type(&write_op_info.op_size);
            let ChargeAndRefund { charge, refund } =
                pricing.charge_refund_write_op(params, write_op_info);
            write_fee += charge;
            total_refund += refund;

            write_set_storage.push(WriteStorage {
                key,
                op_type,
                cost: charge,
                refund,
            });
        }

        // Events (no event fee in v2)
        let mut event_fee = Fee::new(0);
        let mut event_fees = vec![];
        for event in change_set.events_iter() {
            let fee = pricing.legacy_storage_fee_per_event(params, event);
            event_fees.push(EventStorage {
                ty: event.type_tag().clone(),
                cost: fee,
            });
            event_fee += fee;
        }
        let event_discount = pricing.legacy_storage_discount_for_events(params, event_fee);
        let event_net_fee = event_fee
            .checked_sub(event_discount)
            .expect("discount should always be less than or equal to total amount");

        // Txn (no txn fee in v2)
        let txn_fee = pricing.legacy_storage_fee_for_transaction_storage(params, txn_size);

        self.storage_fees = Some(StorageFees {
            total: write_fee + event_fee + txn_fee,
            total_refund,

            write_set_storage,
            events: event_fees,
            event_discount,
            txn_storage: txn_fee,
        });

        let fee = write_fee + event_net_fee + txn_fee;
        self.charge_storage_fee(fee, gas_unit_price)
            .map_err(|err| err.finish(Location::Undefined))?;

        Ok(total_refund)
    }

    fn charge_intrinsic_gas_for_transaction(&mut self, txn_size: NumBytes) -> VMResult<()> {
        let (cost, res) =
            self.delegate_charge(|base| base.charge_intrinsic_gas_for_transaction(txn_size));

        self.intrinsic_cost = Some(cost);

        res
    }

    fn charge_keyless(&mut self) -> VMResult<()> {
        let (_cost, res) = self.delegate_charge(|base| base.charge_keyless());

        // TODO: add keyless

        res
    }
}

impl<G> GasProfiler<G>
where
    G: AptosGasMeter,
{
    pub fn finish(mut self) -> TransactionGasLog {
        while self.frames.len() > 1 {
            let cur = self.frames.pop().expect("frame must exist");
            let last = self.frames.last_mut().expect("frame must exist");
            last.events.push(ExecutionGasEvent::Call(cur));
        }

        let exec_io = ExecutionAndIOCosts {
            gas_scaling_factor: self.base.gas_unit_scaling_factor(),
            total: self.algebra().execution_gas_used() + self.algebra().io_gas_used(),
            intrinsic_cost: self.intrinsic_cost.unwrap_or_else(|| 0.into()),
            keyless_cost: self.keyless_cost.unwrap_or_else(|| 0.into()),
            dependencies: self.dependencies,
            call_graph: self.frames.pop().expect("frame must exist"),
            transaction_transient: self.transaction_transient,
            events_transient: self.events_transient,
            write_set_transient: self.write_set_transient,
        };
        exec_io.assert_consistency();

        let storage = self.storage_fees.unwrap_or_else(|| StorageFees {
            total: 0.into(),
            total_refund: 0.into(),
            write_set_storage: vec![],
            events: vec![],
            event_discount: 0.into(),
            txn_storage: 0.into(),
        });
        storage.assert_consistency();

        TransactionGasLog { exec_io, storage }
    }
}
