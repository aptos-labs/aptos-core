// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module contains the official gas meter implementation, along with some top-level gas
//! parameters and traits to help manipulate them.

use aptos_gas_algebra::{Fee, FeePerGasUnit, Gas, GasExpression, GasScalingFactor, Octa};
use aptos_gas_schedule::{
    instr_gas_params::*, txn_gas_params::*, FromOnChainGasSchedule, InitialGasSchedule,
    InstructionGasParameters, MiscGasParameters, StorageGasParameters, ToOnChainGasSchedule,
    TransactionGasParameters,
};
use aptos_logger::error;
use aptos_types::{
    contract_event::ContractEvent, state_store::state_key::StateKey, write_set::WriteOp,
};
use move_binary_format::{
    errors::{Location, PartialVMError, PartialVMResult, VMResult},
    file_format::CodeOffset,
};
use move_core_types::{
    account_address::AccountAddress,
    gas_algebra::{InternalGas, InternalGasUnit, NumArgs, NumBytes},
    language_storage::ModuleId,
    vm_status::StatusCode,
};
use move_vm_types::{
    gas::{GasMeter as MoveGasMeter, SimpleInstruction},
    views::{TypeView, ValueView},
};
use std::collections::BTreeMap;

/// Gas parameters for all native functions.
#[derive(Debug, Clone)]
pub struct NativeGasParameters {
    pub move_stdlib: aptos_move_stdlib::natives::GasParameters,
    pub aptos_framework: aptos_framework::natives::GasParameters,
    pub table: move_table_extension::GasParameters,
}

impl FromOnChainGasSchedule for NativeGasParameters {
    fn from_on_chain_gas_schedule(
        gas_schedule: &BTreeMap<String, u64>,
        feature_version: u64,
    ) -> Result<Self, String> {
        Ok(Self {
            move_stdlib: FromOnChainGasSchedule::from_on_chain_gas_schedule(
                gas_schedule,
                feature_version,
            )?,
            aptos_framework: FromOnChainGasSchedule::from_on_chain_gas_schedule(
                gas_schedule,
                feature_version,
            )?,
            table: FromOnChainGasSchedule::from_on_chain_gas_schedule(
                gas_schedule,
                feature_version,
            )?,
        })
    }
}

impl ToOnChainGasSchedule for NativeGasParameters {
    fn to_on_chain_gas_schedule(&self, feature_version: u64) -> Vec<(String, u64)> {
        let mut entries = self.move_stdlib.to_on_chain_gas_schedule(feature_version);
        entries.extend(
            self.aptos_framework
                .to_on_chain_gas_schedule(feature_version),
        );
        entries.extend(self.table.to_on_chain_gas_schedule(feature_version));
        entries
    }
}

impl NativeGasParameters {
    pub fn zeros() -> Self {
        Self {
            move_stdlib: aptos_move_stdlib::natives::GasParameters::zeros(),
            aptos_framework: aptos_framework::natives::GasParameters::zeros(),
            table: move_table_extension::GasParameters::zeros(),
        }
    }
}

impl InitialGasSchedule for NativeGasParameters {
    fn initial() -> Self {
        Self {
            move_stdlib: InitialGasSchedule::initial(),
            aptos_framework: InitialGasSchedule::initial(),
            table: InitialGasSchedule::initial(),
        }
    }
}

/// Gas parameters for everything that is needed to run the Aptos blockchain, including
/// instructions, transactions and native functions from various packages.
#[derive(Debug, Clone)]
pub struct AptosGasParameters {
    pub misc: MiscGasParameters,
    pub instr: InstructionGasParameters,
    pub txn: TransactionGasParameters,
    pub natives: NativeGasParameters,
}

impl FromOnChainGasSchedule for AptosGasParameters {
    fn from_on_chain_gas_schedule(
        gas_schedule: &BTreeMap<String, u64>,
        feature_version: u64,
    ) -> Result<Self, String> {
        Ok(Self {
            misc: FromOnChainGasSchedule::from_on_chain_gas_schedule(
                gas_schedule,
                feature_version,
            )?,
            instr: FromOnChainGasSchedule::from_on_chain_gas_schedule(
                gas_schedule,
                feature_version,
            )?,
            txn: FromOnChainGasSchedule::from_on_chain_gas_schedule(gas_schedule, feature_version)?,
            natives: FromOnChainGasSchedule::from_on_chain_gas_schedule(
                gas_schedule,
                feature_version,
            )?,
        })
    }
}

impl ToOnChainGasSchedule for AptosGasParameters {
    fn to_on_chain_gas_schedule(&self, feature_version: u64) -> Vec<(String, u64)> {
        let mut entries = self.instr.to_on_chain_gas_schedule(feature_version);
        entries.extend(self.txn.to_on_chain_gas_schedule(feature_version));
        entries.extend(self.natives.to_on_chain_gas_schedule(feature_version));
        entries.extend(self.misc.to_on_chain_gas_schedule(feature_version));
        entries
    }
}

impl AptosGasParameters {
    pub fn zeros() -> Self {
        Self {
            misc: MiscGasParameters::zeros(),
            instr: InstructionGasParameters::zeros(),
            txn: TransactionGasParameters::zeros(),
            natives: NativeGasParameters::zeros(),
        }
    }
}

impl InitialGasSchedule for AptosGasParameters {
    fn initial() -> Self {
        Self {
            misc: InitialGasSchedule::initial(),
            instr: InitialGasSchedule::initial(),
            txn: InitialGasSchedule::initial(),
            natives: InitialGasSchedule::initial(),
        }
    }
}

/// Trait representing a gas meter used inside the Aptos VM.
///
/// It extends Move VM's `GasMeter` trait with a few Aptos-specific callbacks, along with
/// some primitives that would allow gas meters to be composable.
pub trait AptosGasMeter: MoveGasMeter {
    type Algebra: GasAlgebra;

    /// Returns a reference to the underlying algebra object.
    fn algebra(&self) -> &Self::Algebra;

    /// Returns a mutable reference to the underlying algebra object.
    fn algebra_mut(&mut self) -> &mut Self::Algebra;

    /// Charges fee for utilizing short-term or long-term storage.
    ///
    /// Since the fee is measured in APT/Octa, it needs to be converted into gas units
    /// according to the given unit price.
    fn charge_storage_fee(
        &mut self,
        amount: Fee,
        gas_unit_price: FeePerGasUnit,
    ) -> PartialVMResult<()>;

    /// Charges an intrinsic cost for executing the transaction.
    ///
    /// The cost stays constant for transactions below a certain size, but will grow proportionally
    /// for bigger ones.
    fn charge_intrinsic_gas_for_transaction(&mut self, txn_size: NumBytes) -> VMResult<()>;

    /// Charges IO gas for an item in the write set.
    ///
    /// This is to be differentiated from the storage fee, which is meant to cover the long-term
    /// storage costs.
    fn charge_io_gas_for_write(&mut self, key: &StateKey, op: &WriteOp) -> VMResult<()>;

    /// Calculates the storage fee for a write operation.
    fn storage_fee_per_write(&self, key: &StateKey, op: &WriteOp) -> Fee;

    /// Calculates the storage fee for an event.
    fn storage_fee_per_event(&self, event: &ContractEvent) -> Fee;

    /// Calculates the discount applied to the event storage fees, based on a free quota.
    fn storage_discount_for_events(&self, total_cost: Fee) -> Fee;

    /// Calculates the storage fee for the transaction.
    fn storage_fee_for_transaction_storage(&self, txn_size: NumBytes) -> Fee;

    /// Charges the storage fees for writes, events & txn storage in a lump sum, minimizing the
    /// loss of precision.
    ///
    /// The contract requires that this function behaviors in a way that is consistent to
    /// the ones defining the costs.
    /// Due to this reason, you should normally not override the default implementation,
    /// unless you are doing something special, such as injecting additional logging logic.
    fn charge_storage_fee_for_all<'a>(
        &mut self,
        write_ops: impl IntoIterator<Item = (&'a StateKey, &'a WriteOp)>,
        events: impl IntoIterator<Item = &'a ContractEvent>,
        txn_size: NumBytes,
        gas_unit_price: FeePerGasUnit,
    ) -> VMResult<()> {
        // The new storage fee are only active since version 7.
        if self.feature_version() < 7 {
            return Ok(());
        }

        // TODO(Gas): right now, some of our tests use a unit price of 0 and this is a hack
        // to avoid causing them issues. We should revisit the problem and figure out a
        // better way to handle this.
        if gas_unit_price.is_zero() {
            return Ok(());
        }

        // Calculate the storage fees.
        let write_fee = write_ops.into_iter().fold(Fee::new(0), |acc, (key, op)| {
            acc + self.storage_fee_per_write(key, op)
        });
        let event_fee = events.into_iter().fold(Fee::new(0), |acc, event| {
            acc + self.storage_fee_per_event(event)
        });
        let event_discount = self.storage_discount_for_events(event_fee);
        let event_net_fee = event_fee
            .checked_sub(event_discount)
            .expect("discount should always be less than or equal to total amount");
        let txn_fee = self.storage_fee_for_transaction_storage(txn_size);
        let fee = write_fee + event_net_fee + txn_fee;

        self.charge_storage_fee(fee, gas_unit_price)
            .map_err(|err| err.finish(Location::Undefined))?;

        Ok(())
    }

    // Below are getters reexported from the gas algebra.
    // Gas meter instances should not reimplement these themselves.

    /// Returns the gas feature version.
    fn feature_version(&self) -> u64 {
        self.algebra().feature_version()
    }

    /// Returns the struct that contains all (non-storage) gas parameters.
    fn gas_params(&self) -> &AptosGasParameters {
        self.algebra().gas_params()
    }

    // Returns a reference to the struct containing all storage gas parameters.
    fn storage_gas_params(&self) -> &StorageGasParameters {
        self.algebra().storage_gas_params()
    }

    /// Returns the remaining balance, measured in (external) gas units.
    ///
    /// The number should be rounded down when converting from internal to external gas units.
    fn balance(&self) -> Gas {
        self.algebra()
            .balance_internal()
            .to_unit_round_down_with_params(&self.gas_params().txn)
    }

    /// Returns the scaling factor between internal and external gas units.
    fn gas_unit_scaling_factor(&self) -> GasScalingFactor {
        self.algebra().gas_params().txn.scaling_factor()
    }

    /// Return the total gas used for execution.
    fn execution_gas_used(&self) -> Gas {
        self.algebra()
            .execution_gas_used()
            .to_unit_round_up_with_params(&self.gas_params().txn)
    }

    /// Return the total gas used for io.
    fn io_gas_used(&self) -> Gas {
        self.algebra()
            .io_gas_used()
            .to_unit_round_up_with_params(&self.gas_params().txn)
    }

    /// Return the total gas used for storage.
    fn storage_fee_used_in_gas_units(&self) -> Gas {
        self.algebra()
            .storage_fee_used_in_gas_units()
            .to_unit_round_up_with_params(&self.gas_params().txn)
    }

    /// Return the total fee used for storage.
    fn storage_fee_used(&self) -> Fee {
        self.algebra().storage_fee_used()
    }
}

pub trait GasAlgebra {
    fn feature_version(&self) -> u64;

    fn gas_params(&self) -> &AptosGasParameters;

    fn storage_gas_params(&self) -> &StorageGasParameters;

    fn balance_internal(&self) -> InternalGas;

    fn charge_execution(
        &mut self,
        abstract_amount: impl GasExpression<AptosGasParameters, Unit = InternalGasUnit>,
    ) -> PartialVMResult<()>;

    fn charge_io(
        &mut self,
        abstract_amount: impl GasExpression<AptosGasParameters, Unit = InternalGasUnit>,
    ) -> PartialVMResult<()>;

    fn charge_storage_fee(
        &mut self,
        abstract_amount: impl GasExpression<AptosGasParameters, Unit = Octa>,
        gas_unit_price: FeePerGasUnit,
    ) -> PartialVMResult<()>;

    fn execution_gas_used(&self) -> InternalGas;

    fn io_gas_used(&self) -> InternalGas;

    fn storage_fee_used_in_gas_units(&self) -> InternalGas;

    fn storage_fee_used(&self) -> Fee;
}

pub struct StandardGasAlgebra {
    feature_version: u64,
    gas_params: AptosGasParameters,
    storage_gas_params: StorageGasParameters,

    balance: InternalGas,

    execution_gas_used: InternalGas,
    io_gas_used: InternalGas,
    // The gas consumed by the storage operations.
    storage_fee_in_internal_units: InternalGas,
    // The storage fee consumed by the storage operations.
    storage_fee_used: Fee,
}

impl StandardGasAlgebra {
    pub fn new(
        gas_feature_version: u64,
        gas_params: AptosGasParameters,
        storage_gas_params: StorageGasParameters,
        balance: impl Into<Gas>,
    ) -> Self {
        let balance = balance.into().to_unit_with_params(&gas_params.txn);

        Self {
            feature_version: gas_feature_version,
            gas_params,
            storage_gas_params,
            balance,
            execution_gas_used: 0.into(),
            io_gas_used: 0.into(),
            storage_fee_in_internal_units: 0.into(),
            storage_fee_used: 0.into(),
        }
    }
}

impl StandardGasAlgebra {
    fn charge(&mut self, amount: InternalGas) -> PartialVMResult<()> {
        match self.balance.checked_sub(amount) {
            Some(new_balance) => {
                self.balance = new_balance;
                Ok(())
            },
            None => {
                self.balance = 0.into();
                Err(PartialVMError::new(StatusCode::OUT_OF_GAS))
            },
        }
    }
}

impl GasAlgebra for StandardGasAlgebra {
    fn feature_version(&self) -> u64 {
        self.feature_version
    }

    fn gas_params(&self) -> &AptosGasParameters {
        &self.gas_params
    }

    fn storage_gas_params(&self) -> &StorageGasParameters {
        &self.storage_gas_params
    }

    fn balance_internal(&self) -> InternalGas {
        self.balance
    }

    fn charge_execution(
        &mut self,
        abstract_amount: impl GasExpression<AptosGasParameters, Unit = InternalGasUnit>,
    ) -> PartialVMResult<()> {
        let amount = abstract_amount.materialize(self.feature_version, &self.gas_params);

        self.charge(amount)?;

        self.execution_gas_used += amount;
        if self.feature_version >= 7
            && self.execution_gas_used > self.gas_params.txn.max_execution_gas
        {
            Err(PartialVMError::new(StatusCode::EXECUTION_LIMIT_REACHED))
        } else {
            Ok(())
        }
    }

    fn charge_io(
        &mut self,
        abstract_amount: impl GasExpression<AptosGasParameters, Unit = InternalGasUnit>,
    ) -> PartialVMResult<()> {
        let amount = abstract_amount.materialize(self.feature_version, &self.gas_params);

        self.charge(amount)?;

        self.io_gas_used += amount;
        if self.feature_version >= 7 && self.io_gas_used > self.gas_params.txn.max_io_gas {
            Err(PartialVMError::new(StatusCode::IO_LIMIT_REACHED))
        } else {
            Ok(())
        }
    }

    fn charge_storage_fee(
        &mut self,
        abstract_amount: impl GasExpression<AptosGasParameters, Unit = Octa>,
        gas_unit_price: FeePerGasUnit,
    ) -> PartialVMResult<()> {
        let amount = abstract_amount.materialize(self.feature_version, &self.gas_params);

        let txn_params = &self.gas_params.txn;

        // Because the storage fees are defined in terms of fixed APT costs, we need
        // to convert them into gas units.
        //
        // u128 is used to protect against overflow and preserve as much precision as
        // possible in the extreme cases.
        fn div_ceil(n: u128, d: u128) -> u128 {
            if n % d == 0 {
                n / d
            } else {
                n / d + 1
            }
        }
        let gas_consumed_internal = div_ceil(
            (u64::from(amount) as u128) * (u64::from(txn_params.gas_unit_scaling_factor) as u128),
            u64::from(gas_unit_price) as u128,
        );
        let gas_consumed_internal = InternalGas::new(if gas_consumed_internal > u64::MAX as u128 {
            error!(
                "Something's wrong in the gas schedule: gas_consumed_internal ({}) > u64::MAX",
                gas_consumed_internal
            );
            u64::MAX
        } else {
            gas_consumed_internal as u64
        });

        self.charge(gas_consumed_internal)?;

        self.storage_fee_in_internal_units += gas_consumed_internal;
        self.storage_fee_used += amount;
        if self.feature_version >= 7 && self.storage_fee_used > self.gas_params.txn.max_storage_fee
        {
            return Err(PartialVMError::new(StatusCode::STORAGE_LIMIT_REACHED));
        }

        Ok(())
    }

    fn execution_gas_used(&self) -> InternalGas {
        self.execution_gas_used
    }

    fn io_gas_used(&self) -> InternalGas {
        self.io_gas_used
    }

    fn storage_fee_used_in_gas_units(&self) -> InternalGas {
        self.storage_fee_in_internal_units
    }

    fn storage_fee_used(&self) -> Fee {
        self.storage_fee_used
    }
}

/// The official gas meter used inside the Aptos VM.
/// It maintains an internal gas counter, measured in internal gas units, and carries an environment
/// consisting all the gas parameters, which it can lookup when performing gas calculations.
pub struct StandardGasMeter<A> {
    algebra: A,
}

impl<A> StandardGasMeter<A>
where
    A: GasAlgebra,
{
    pub fn new(algebra: A) -> Self {
        Self { algebra }
    }

    pub fn feature_version(&self) -> u64 {
        self.algebra.feature_version()
    }
}

impl<A> MoveGasMeter for StandardGasMeter<A>
where
    A: GasAlgebra,
{
    #[inline]
    fn balance_internal(&self) -> InternalGas {
        self.algebra.balance_internal()
    }

    #[inline]
    fn charge_br_false(&mut self, _target_offset: Option<CodeOffset>) -> PartialVMResult<()> {
        self.algebra.charge_execution(BR_FALSE)
    }

    #[inline]
    fn charge_br_true(&mut self, _target_offset: Option<CodeOffset>) -> PartialVMResult<()> {
        self.algebra.charge_execution(BR_TRUE)
    }

    #[inline]
    fn charge_branch(&mut self, _target_offset: CodeOffset) -> PartialVMResult<()> {
        self.algebra.charge_execution(BRANCH)
    }

    #[inline]
    fn charge_simple_instr(&mut self, instr: SimpleInstruction) -> PartialVMResult<()> {
        macro_rules! dispatch {
            ($($name: ident => $cost: expr),* $(,)?) => {
                match instr {
                    $(SimpleInstruction::$name => self.algebra.charge_execution($cost)),*
                }
            };
        }

        dispatch! {
            Nop => NOP,

            Abort => ABORT,
            Ret => RET,

            LdU8 => LD_U8,
            LdU16 => LD_U16,
            LdU32 => LD_U32,
            LdU64 => LD_U64,
            LdU128 => LD_U128,
            LdU256 => LD_U256,
            LdTrue => LD_TRUE,
            LdFalse => LD_FALSE,

            ImmBorrowLoc => IMM_BORROW_LOC,
            MutBorrowLoc => MUT_BORROW_LOC,
            ImmBorrowField => IMM_BORROW_FIELD,
            MutBorrowField => MUT_BORROW_FIELD,
            ImmBorrowFieldGeneric => IMM_BORROW_FIELD_GENERIC,
            MutBorrowFieldGeneric => MUT_BORROW_FIELD_GENERIC,
            FreezeRef => FREEZE_REF,

            CastU8 => CAST_U8,
            CastU16 => CAST_U16,
            CastU32 => CAST_U32,
            CastU64 => CAST_U64,
            CastU128 => CAST_U128,
            CastU256 => CAST_U256,

            Add => ADD,
            Sub => SUB,
            Mul => MUL,
            Mod => MOD_,
            Div => DIV,

            BitOr => BIT_OR,
            BitAnd => BIT_AND,
            Xor => XOR,
            Shl => SHL,
            Shr => SHR,

            Or => OR,
            And => AND,
            Not => NOT,

            Lt => LT,
            Gt => GT,
            Le => LE,
            Ge => GE,
        }
    }

    #[inline]
    fn charge_native_function_before_execution(
        &mut self,
        _ty_args: impl ExactSizeIterator<Item = impl TypeView>,
        _args: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    #[inline]
    fn charge_native_function(
        &mut self,
        amount: InternalGas,
        _ret_vals: Option<impl ExactSizeIterator<Item = impl ValueView>>,
    ) -> PartialVMResult<()> {
        // TODO(Gas): Is this correct???
        self.algebra.charge_execution(amount)
    }

    #[inline]
    fn charge_load_resource(
        &mut self,
        _addr: AccountAddress,
        _ty: impl TypeView,
        val: Option<impl ValueView>,
        bytes_loaded: NumBytes,
    ) -> PartialVMResult<()> {
        // TODO(Gas): check if this is correct.
        if self.feature_version() <= 8 && val.is_none() && bytes_loaded != 0.into() {
            return Err(PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message("in legacy versions, number of bytes loaded must be zero when the resource does not exist ".to_string()));
        }
        let cost = self
            .storage_gas_params()
            .pricing
            .calculate_read_gas(val.is_some(), bytes_loaded);
        self.algebra.charge_io(cost)
    }

    #[inline]
    fn charge_pop(&mut self, _popped_val: impl ValueView) -> PartialVMResult<()> {
        self.algebra.charge_execution(POP)
    }

    #[inline]
    fn charge_call(
        &mut self,
        _module_id: &ModuleId,
        _func_name: &str,
        args: impl ExactSizeIterator<Item = impl ValueView>,
        num_locals: NumArgs,
    ) -> PartialVMResult<()> {
        let cost = CALL_BASE + CALL_PER_ARG * NumArgs::new(args.len() as u64);

        match self.feature_version() {
            0..=2 => self.algebra.charge_execution(cost),
            3.. => self
                .algebra
                .charge_execution(cost + CALL_PER_LOCAL * num_locals),
        }
    }

    #[inline]
    fn charge_call_generic(
        &mut self,
        _module_id: &ModuleId,
        _func_name: &str,
        ty_args: impl ExactSizeIterator<Item = impl TypeView>,
        args: impl ExactSizeIterator<Item = impl ValueView>,
        num_locals: NumArgs,
    ) -> PartialVMResult<()> {
        let cost = CALL_GENERIC_BASE
            + CALL_GENERIC_PER_TY_ARG * NumArgs::new(ty_args.len() as u64)
            + CALL_GENERIC_PER_ARG * NumArgs::new(args.len() as u64);

        match self.feature_version() {
            0..=2 => self.algebra.charge_execution(cost),
            3.. => self
                .algebra
                .charge_execution(cost + CALL_GENERIC_PER_LOCAL * num_locals),
        }
    }

    #[inline]
    fn charge_ld_const(&mut self, size: NumBytes) -> PartialVMResult<()> {
        self.algebra
            .charge_execution(LD_CONST_BASE + LD_CONST_PER_BYTE * size)
    }

    #[inline]
    fn charge_ld_const_after_deserialization(
        &mut self,
        _val: impl ValueView,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    #[inline]
    fn charge_copy_loc(&mut self, val: impl ValueView) -> PartialVMResult<()> {
        let (stack_size, heap_size) = self
            .gas_params()
            .misc
            .abs_val
            .abstract_value_size_stack_and_heap(val, self.feature_version());

        // Note(Gas): this makes a deep copy so we need to charge for the full value size
        self.algebra
            .charge_execution(COPY_LOC_BASE + COPY_LOC_PER_ABS_VAL_UNIT * (stack_size + heap_size))
    }

    #[inline]
    fn charge_move_loc(&mut self, _val: impl ValueView) -> PartialVMResult<()> {
        self.algebra.charge_execution(MOVE_LOC_BASE)
    }

    #[inline]
    fn charge_store_loc(&mut self, _val: impl ValueView) -> PartialVMResult<()> {
        self.algebra.charge_execution(ST_LOC_BASE)
    }

    #[inline]
    fn charge_pack(
        &mut self,
        is_generic: bool,
        args: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        let num_args = NumArgs::new(args.len() as u64);

        match is_generic {
            false => self
                .algebra
                .charge_execution(PACK_BASE + PACK_PER_FIELD * num_args),
            true => self
                .algebra
                .charge_execution(PACK_GENERIC_BASE + PACK_GENERIC_PER_FIELD * num_args),
        }
    }

    #[inline]
    fn charge_unpack(
        &mut self,
        is_generic: bool,
        args: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        let num_args = NumArgs::new(args.len() as u64);

        match is_generic {
            false => self
                .algebra
                .charge_execution(UNPACK_BASE + UNPACK_PER_FIELD * num_args),
            true => self
                .algebra
                .charge_execution(UNPACK_GENERIC_BASE + UNPACK_GENERIC_PER_FIELD * num_args),
        }
    }

    #[inline]
    fn charge_read_ref(&mut self, val: impl ValueView) -> PartialVMResult<()> {
        let (stack_size, heap_size) = self
            .gas_params()
            .misc
            .abs_val
            .abstract_value_size_stack_and_heap(val, self.feature_version());

        // Note(Gas): this makes a deep copy so we need to charge for the full value size
        self.algebra
            .charge_execution(READ_REF_BASE + READ_REF_PER_ABS_VAL_UNIT * (stack_size + heap_size))
    }

    #[inline]
    fn charge_write_ref(
        &mut self,
        _new_val: impl ValueView,
        _old_val: impl ValueView,
    ) -> PartialVMResult<()> {
        self.algebra.charge_execution(WRITE_REF_BASE)
    }

    #[inline]
    fn charge_eq(&mut self, lhs: impl ValueView, rhs: impl ValueView) -> PartialVMResult<()> {
        let abs_val_params = &self.gas_params().misc.abs_val;

        let cost = EQ_BASE
            + EQ_PER_ABS_VAL_UNIT
                * (abs_val_params.abstract_value_size_dereferenced(lhs, self.feature_version())
                    + abs_val_params.abstract_value_size_dereferenced(rhs, self.feature_version()));

        self.algebra.charge_execution(cost)
    }

    #[inline]
    fn charge_neq(&mut self, lhs: impl ValueView, rhs: impl ValueView) -> PartialVMResult<()> {
        let abs_val_params = &self.gas_params().misc.abs_val;

        let cost = NEQ_BASE
            + NEQ_PER_ABS_VAL_UNIT
                * (abs_val_params.abstract_value_size_dereferenced(lhs, self.feature_version())
                    + abs_val_params.abstract_value_size_dereferenced(rhs, self.feature_version()));

        self.algebra.charge_execution(cost)
    }

    #[inline]
    fn charge_borrow_global(
        &mut self,
        is_mut: bool,
        is_generic: bool,
        _ty: impl TypeView,
        _is_success: bool,
    ) -> PartialVMResult<()> {
        match (is_mut, is_generic) {
            (false, false) => self.algebra.charge_execution(IMM_BORROW_GLOBAL_BASE),
            (false, true) => self
                .algebra
                .charge_execution(IMM_BORROW_GLOBAL_GENERIC_BASE),
            (true, false) => self.algebra.charge_execution(MUT_BORROW_GLOBAL_BASE),
            (true, true) => self
                .algebra
                .charge_execution(MUT_BORROW_GLOBAL_GENERIC_BASE),
        }
    }

    #[inline]
    fn charge_exists(
        &mut self,
        is_generic: bool,
        _ty: impl TypeView,
        _exists: bool,
    ) -> PartialVMResult<()> {
        match is_generic {
            false => self.algebra.charge_execution(EXISTS_BASE),
            true => self.algebra.charge_execution(EXISTS_GENERIC_BASE),
        }
    }

    #[inline]
    fn charge_move_from(
        &mut self,
        is_generic: bool,
        _ty: impl TypeView,
        _val: Option<impl ValueView>,
    ) -> PartialVMResult<()> {
        match is_generic {
            false => self.algebra.charge_execution(MOVE_FROM_BASE),
            true => self.algebra.charge_execution(MOVE_FROM_GENERIC_BASE),
        }
    }

    #[inline]
    fn charge_move_to(
        &mut self,
        is_generic: bool,
        _ty: impl TypeView,
        _val: impl ValueView,
        _is_success: bool,
    ) -> PartialVMResult<()> {
        match is_generic {
            false => self.algebra.charge_execution(MOVE_TO_BASE),
            true => self.algebra.charge_execution(MOVE_TO_GENERIC_BASE),
        }
    }

    #[inline]
    fn charge_vec_pack<'a>(
        &mut self,
        _ty: impl TypeView + 'a,
        args: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        let num_args = NumArgs::new(args.len() as u64);

        self.algebra
            .charge_execution(VEC_PACK_BASE + VEC_PACK_PER_ELEM * num_args)
    }

    #[inline]
    fn charge_vec_unpack(
        &mut self,
        _ty: impl TypeView,
        expect_num_elements: NumArgs,
        _elems: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        self.algebra
            .charge_execution(VEC_UNPACK_BASE + VEC_UNPACK_PER_EXPECTED_ELEM * expect_num_elements)
    }

    #[inline]
    fn charge_vec_len(&mut self, _ty: impl TypeView) -> PartialVMResult<()> {
        self.algebra.charge_execution(VEC_LEN_BASE)
    }

    #[inline]
    fn charge_vec_borrow(
        &mut self,
        is_mut: bool,
        _ty: impl TypeView,
        _is_success: bool,
    ) -> PartialVMResult<()> {
        match is_mut {
            false => self.algebra.charge_execution(VEC_IMM_BORROW_BASE),
            true => self.algebra.charge_execution(VEC_MUT_BORROW_BASE),
        }
    }

    #[inline]
    fn charge_vec_push_back(
        &mut self,
        _ty: impl TypeView,
        _val: impl ValueView,
    ) -> PartialVMResult<()> {
        self.algebra.charge_execution(VEC_PUSH_BACK_BASE)
    }

    #[inline]
    fn charge_vec_pop_back(
        &mut self,
        _ty: impl TypeView,
        _val: Option<impl ValueView>,
    ) -> PartialVMResult<()> {
        self.algebra.charge_execution(VEC_POP_BACK_BASE)
    }

    #[inline]
    fn charge_vec_swap(&mut self, _ty: impl TypeView) -> PartialVMResult<()> {
        self.algebra.charge_execution(VEC_SWAP_BASE)
    }

    #[inline]
    fn charge_drop_frame(
        &mut self,
        _locals: impl Iterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        Ok(())
    }
}

impl<A> AptosGasMeter for StandardGasMeter<A>
where
    A: GasAlgebra,
{
    type Algebra = A;

    #[inline]
    fn algebra(&self) -> &Self::Algebra {
        &self.algebra
    }

    #[inline]
    fn algebra_mut(&mut self) -> &mut Self::Algebra {
        &mut self.algebra
    }

    #[inline]
    fn charge_storage_fee(
        &mut self,
        amount: Fee,
        gas_unit_price: FeePerGasUnit,
    ) -> PartialVMResult<()> {
        self.algebra.charge_storage_fee(amount, gas_unit_price)
    }

    fn charge_io_gas_for_write(&mut self, key: &StateKey, op: &WriteOp) -> VMResult<()> {
        let cost = self.storage_gas_params().pricing.io_gas_per_write(key, op);
        self.algebra
            .charge_io(cost)
            .map_err(|e| e.finish(Location::Undefined))
    }

    fn storage_fee_per_write(&self, key: &StateKey, op: &WriteOp) -> Fee {
        self.gas_params().txn.storage_fee_per_write(key, op)
    }

    fn storage_fee_per_event(&self, event: &ContractEvent) -> Fee {
        self.gas_params().txn.storage_fee_per_event(event)
    }

    fn storage_discount_for_events(&self, total_cost: Fee) -> Fee {
        self.gas_params()
            .txn
            .storage_discount_for_events(total_cost)
    }

    fn storage_fee_for_transaction_storage(&self, txn_size: NumBytes) -> Fee {
        self.gas_params()
            .txn
            .storage_fee_for_transaction_storage(txn_size)
    }

    fn charge_intrinsic_gas_for_transaction(&mut self, txn_size: NumBytes) -> VMResult<()> {
        let excess = txn_size
            .checked_sub(self.gas_params().txn.large_transaction_cutoff)
            .unwrap_or_else(|| 0.into());

        self.algebra
            .charge_execution(MIN_TRANSACTION_GAS_UNITS + INTRINSIC_GAS_PER_BYTE * excess)
            .map_err(|e| e.finish(Location::Undefined))
    }
}
