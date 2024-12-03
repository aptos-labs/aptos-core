// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::{fmt::Display, io::Write};

use crate::FrameName;
use aptos_gas_meter::{AptosGasMeter, GasAlgebra};
use move_binary_format::{
    errors::PartialVMResult,
    file_format::{Bytecode, CodeOffset},
    file_format_common::Opcodes,
};
use move_bytecode_source_map::source_map::SourceMap;
use move_core_types::{identifier::Identifier, language_storage::{ModuleId, TypeTag}};
use move_ir_types::location::Loc;
use move_vm_types::{
    gas::{GasMeter, InterpreterView, SimpleInstruction},
    values::Value,
    views::{TypeView, ValueView, ValueVisitor},
};

#[derive(Debug)]
pub struct ExecutionTrace<Value>(CallFrame<Value>);

impl<Value: Display> ExecutionTrace<Value> {
    pub fn simple_debug(&self, w: &mut impl Write) -> Result<(), std::io::Error> {
        self.0.simple_debug(w, 0)
    }
}

#[derive(Debug)]
pub struct ExecutionTracer<G, Value> {
    base: G,
    frames: Vec<CallFrame<Value>>,
}

impl<G, Value> ExecutionTracer<G, Value> {
    pub fn from_entry_fun(base: G, module_id: ModuleId, function: Identifier, ty_args: Vec<TypeTag>) -> Self {
        Self {
            base,
            frames: vec![CallFrame::new_function(module_id, function, ty_args)],
        }
    }

    pub fn from_script(base: G) -> Self {
        Self {
            base,
            frames: vec![CallFrame::new_script()],
        }
    }

    fn get_top_frame(&self) -> &CallFrame<Value> {
        self.frames.last().expect("non-empty stack of frames")
    }

    fn get_top_frame_mut(&mut self) -> &mut CallFrame<Value> {
        self.frames.last_mut().expect("non-empty stack of frames")
    }

    fn get_top_events_mut(&mut self) -> &mut Vec<Event<Value>> {
        &mut self.get_top_frame_mut().events
    }

    fn gen_new_frame(&mut self, name: FrameName) {
        self.frames.push(CallFrame {
            name,
            events: Vec::new(),
            pc: 0,
        });
    }

    pub fn dump_trace(mut self) -> ExecutionTrace<Value> {
        debug_assert!(self.frames.len() == 1);
        ExecutionTrace(self.frames.pop().expect("non-empty stack of frames"))
    }
}

impl<G> ExecutionTracer<G, String> {
    fn emit_instr(&mut self, op: Opcodes, args: impl ExactSizeIterator<Item = impl ValueView + Display> + Clone) {
        self.emit_generic_instr(op, vec![], args);
    }

    fn emit_generic_instr(&mut self, op: Opcodes, ty_args: Vec<TypeTag>, args: impl ExactSizeIterator<Item = impl ValueView + Display> + Clone) {
        let pc = self.get_pc();
        self.get_top_events_mut().push(Event::Instruction(Instruction {
            op,
            ty_args,
            args: args.map(|v| v.to_string()).collect(),
            offset: pc
        }));
    }

    fn emit_generic_instr_and_inc_pc(
        &mut self,
        op: Opcodes,
        ty_args: Vec<TypeTag>,
        args: impl ExactSizeIterator<Item = impl ValueView + Display> + Clone,
    ) {
        self.emit_generic_instr(op, ty_args, args);
        self.inc_pc();
    }

    fn emit_instr_and_inc_pc(
        &mut self,
        op: Opcodes,
        args: impl ExactSizeIterator<Item = impl ValueView + Display> + Clone,
    ) {
        self.emit_generic_instr_and_inc_pc(op, vec![], args)
    }

    fn get_pc(&self) -> CodeOffset {
        self.get_top_frame().pc
    }

    fn get_pc_mut(&mut self) -> &mut CodeOffset {
        &mut self.get_top_frame_mut().pc
    }

    fn inc_pc(&mut self) {
        *self.get_pc_mut() += 1;
    }
}

/// Records the execution of an instruction
#[derive(Debug)]
pub struct Instruction<Value> {
    pub op: Opcodes,
    pub ty_args: Vec<TypeTag>,
    pub args: Vec<Value>,
    pub offset: CodeOffset,
}

impl<Value: Display> Instruction<Value> {
    pub fn display_qualified_instr(&self) -> impl Display + '_ {
        struct QualifiedInstr<'a> {
            op: Opcodes,
            ty_args: &'a [TypeTag],
        }

        impl Display for QualifiedInstr<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{:?}", self.op)?;
                if !self.ty_args.is_empty() {
                    write!(f, "<")?;
                }
                for (i, ty_arg) in self.ty_args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", ty_arg)?;
                }
                if !self.ty_args.is_empty() {
                    write!(f, ">")?;
                }
                Ok(())
            }
        }
        QualifiedInstr { op: self.op, ty_args: &self.ty_args }
    }
}

/// Records the execution of a function call
#[derive(Debug)]
pub struct CallFrame<Value> {
    pub name: FrameName,
    pub events: Vec<Event<Value>>,
    pub pc: CodeOffset,
}

impl<Value> CallFrame<Value> {
    fn new_function(module_id: ModuleId, name: Identifier, ty_args: Vec<TypeTag>) -> Self {
        Self {
            name: FrameName::Function { module_id, name, ty_args },
            events: Vec::new(),
            pc: 0,
        }
    }

    fn new_script() -> Self {
        Self {
            name: FrameName::Script,
            events: Vec::new(),
            pc: 0,
        }
    }
}

impl<Value: Display> CallFrame<Value> {
    pub fn simple_debug(&self, w: &mut impl Write, depth: usize) -> Result<(), std::io::Error> {
        writeln!(w, "{}{}", " ".repeat(depth * 4), self.name)?;
        for event in &self.events {
            match event {
                Event::Instruction(instr) => {
                    write!(w, "{}", " ".repeat(depth * 4 + 2))?;
                    write!(w, "{}: ", instr.offset)?;
                    write!(w, "{} ", instr.display_qualified_instr())?;
                    for arg in &instr.args {
                        write!(w, "{} ", arg)?;
                    }
                    writeln!(w)?;
                },
                Event::Call(frame) => frame.simple_debug(w, depth + 1)?,
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum Event<Value> {
    Call(CallFrame<Value>),
    Instruction(Instruction<Value>),
}

impl<G> GasMeter for ExecutionTracer<G, String>
where
    G: AptosGasMeter,
{
    fn balance_internal(&self) -> aptos_gas_algebra::InternalGas {
        self.base.balance_internal()
    }

    fn charge_simple_instr(&mut self, instr: SimpleInstruction, interpreter: impl InterpreterView) -> PartialVMResult<()> {
        match instr {
            SimpleInstruction::Nop => (),
            SimpleInstruction::Ret => {
                if self.frames.len() > 1 {
                    let cur_frame = self.frames.pop().expect("frame must exist");
                    let last_frame = self.frames.last_mut().expect("frame must exist");
                    last_frame.events.push(Event::Call(cur_frame));
                }
            },
            SimpleInstruction::LdU8 => self.emit_instr_and_inc_pc(Opcodes::LD_U8, std::iter::empty::<&Value>()),
            SimpleInstruction::LdU64 => self.emit_instr_and_inc_pc(Opcodes::LD_U64, std::iter::empty::<&Value>()),
            SimpleInstruction::LdU128 => self.emit_instr_and_inc_pc(Opcodes::LD_U128, std::iter::empty::<&Value>()),
            SimpleInstruction::LdTrue => self.emit_instr_and_inc_pc(Opcodes::LD_TRUE, std::iter::empty::<&Value>()),
            SimpleInstruction::LdFalse => self.emit_instr_and_inc_pc(Opcodes::LD_FALSE, std::iter::empty::<&Value>()),
            SimpleInstruction::FreezeRef => self.emit_instr_and_inc_pc(Opcodes::FREEZE_REF, std::iter::empty::<&Value>()),
            SimpleInstruction::MutBorrowLoc => self.emit_instr_and_inc_pc(Opcodes::MUT_BORROW_LOC, std::iter::empty::<&Value>()),
            SimpleInstruction::ImmBorrowLoc => self.emit_instr_and_inc_pc(Opcodes::IMM_BORROW_LOC, std::iter::empty::<&Value>()),
            SimpleInstruction::ImmBorrowField => self.emit_instr_and_inc_pc(Opcodes::IMM_BORROW_FIELD, interpreter.view_last_n_values(1).unwrap()),
            SimpleInstruction::MutBorrowField => self.emit_instr_and_inc_pc(Opcodes::MUT_BORROW_FIELD, interpreter.view_last_n_values(1).unwrap()),
            SimpleInstruction::ImmBorrowFieldGeneric => self.emit_instr_and_inc_pc(Opcodes::IMM_BORROW_FIELD_GENERIC, interpreter.view_last_n_values(1).unwrap()),
            SimpleInstruction::MutBorrowFieldGeneric => self.emit_instr_and_inc_pc(Opcodes::MUT_BORROW_FIELD_GENERIC, interpreter.view_last_n_values(1).unwrap()),
            SimpleInstruction::ImmBorrowVariantField => self.emit_instr_and_inc_pc(Opcodes::IMM_BORROW_VARIANT_FIELD, interpreter.view_last_n_values(1).unwrap()),
            SimpleInstruction::MutBorrowVariantField => self.emit_instr_and_inc_pc(Opcodes::MUT_BORROW_VARIANT_FIELD, interpreter.view_last_n_values(1).unwrap()),
            SimpleInstruction::ImmBorrowVariantFieldGeneric => self.emit_instr_and_inc_pc(Opcodes::IMM_BORROW_VARIANT_FIELD_GENERIC, interpreter.view_last_n_values(1).unwrap()),
            SimpleInstruction::MutBorrowVariantFieldGeneric => self.emit_instr_and_inc_pc(Opcodes::MUT_BORROW_VARIANT_FIELD_GENERIC, interpreter.view_last_n_values(1).unwrap()),
            SimpleInstruction::TestVariant => self.emit_instr_and_inc_pc(Opcodes::TEST_VARIANT, interpreter.view_last_n_values(1).unwrap()),
            SimpleInstruction::TestVariantGeneric => self.emit_instr_and_inc_pc(Opcodes::TEST_VARIANT_GENERIC, interpreter.view_last_n_values(1).unwrap()),
            SimpleInstruction::CastU8 => self.emit_instr_and_inc_pc(Opcodes::CAST_U8, interpreter.view_last_n_values(1).unwrap()),
            SimpleInstruction::CastU64 => self.emit_instr_and_inc_pc(Opcodes::CAST_U64, interpreter.view_last_n_values(1).unwrap()),
            SimpleInstruction::CastU128 => self.emit_instr_and_inc_pc(Opcodes::CAST_U128, interpreter.view_last_n_values(1).unwrap()),
            SimpleInstruction::Add => self.emit_instr_and_inc_pc(Opcodes::ADD, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Sub => self.emit_instr_and_inc_pc(Opcodes::SUB, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Mul => self.emit_instr_and_inc_pc(Opcodes::MUL, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Mod => self.emit_instr_and_inc_pc(Opcodes::MOD, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Div => self.emit_instr_and_inc_pc(Opcodes::DIV, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::BitOr => self.emit_instr_and_inc_pc(Opcodes::BIT_OR, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::BitAnd => self.emit_instr_and_inc_pc(Opcodes::BIT_AND, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Xor => self.emit_instr_and_inc_pc(Opcodes::XOR, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Shl => self.emit_instr_and_inc_pc(Opcodes::SHL, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Shr => self.emit_instr_and_inc_pc(Opcodes::SHR, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Or => self.emit_instr_and_inc_pc(Opcodes::OR, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::And => self.emit_instr_and_inc_pc(Opcodes::AND, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Not => self.emit_instr_and_inc_pc(Opcodes::NOT, interpreter.view_last_n_values(1).unwrap()),
            SimpleInstruction::Lt => self.emit_instr_and_inc_pc(Opcodes::LT, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Gt => self.emit_instr_and_inc_pc(Opcodes::GT, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Le => self.emit_instr_and_inc_pc(Opcodes::LE, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Ge => self.emit_instr_and_inc_pc(Opcodes::GE, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Abort => self.emit_instr_and_inc_pc(Opcodes::ABORT, interpreter.view_last_n_values(1).unwrap()),
            SimpleInstruction::LdU16 => (),
            SimpleInstruction::LdU32 => (),
            SimpleInstruction::LdU256 => (),
            SimpleInstruction::CastU16 => (),
            SimpleInstruction::CastU32 => (),
            SimpleInstruction::CastU256 => (),
        }
        self.base.charge_simple_instr(instr, interpreter)
    }

    fn charge_br_true(&mut self, target_offset: Option<CodeOffset>) -> PartialVMResult<()> {
        self.emit_instr(Opcodes::BR_TRUE, Vec::<&Value>::new().into_iter());
        if let Some(offset) = target_offset {
            *self.get_pc_mut() = offset;
        } else {
            *self.get_pc_mut() += 1;
        }
        self.base.charge_br_true(target_offset)
    }

    fn charge_br_false(&mut self, target_offset: Option<CodeOffset>) -> PartialVMResult<()> {
        self.emit_instr(Opcodes::BR_FALSE, Vec::<&Value>::new().into_iter());
        if let Some(offset) = target_offset {
            *self.get_pc_mut() = offset;
        } else {
            *self.get_pc_mut() += 1;
        }
        self.base.charge_br_false(target_offset)
    }

    fn charge_branch(&mut self, target_offset: CodeOffset) -> PartialVMResult<()> {
        self.emit_instr(Opcodes::BRANCH, Vec::<&Value>::new().into_iter());
        *self.get_pc_mut() = target_offset;
        self.base.charge_branch(target_offset)
    }

    fn charge_pop(&mut self, popped_val: impl ValueView + Display) -> PartialVMResult<()> {
        println!("charge_pop: {:?}", self.frames);
        self.emit_generic_instr_and_inc_pc(Opcodes::POP, vec![], [&popped_val].into_iter());
        self.base.charge_pop(popped_val)
    }

    fn charge_call(
        &mut self,
        module_id: &move_core_types::language_storage::ModuleId,
        func_name: &str,
        args: impl ExactSizeIterator<Item = impl ValueView + Display> + Clone,
        num_locals: aptos_gas_algebra::NumArgs,
    ) -> PartialVMResult<()> {
        self.inc_pc();
        self.gen_new_frame(FrameName::Function {
            module_id: module_id.clone(),
            name: Identifier::new(func_name).unwrap(),
            ty_args: vec![],
        });
        self.base.charge_call(module_id, func_name, args, num_locals)
    }

    fn charge_call_generic(
        &mut self,
        module_id: &move_core_types::language_storage::ModuleId,
        func_name: &str,
        ty_args: impl ExactSizeIterator<Item = impl TypeView> + Clone,
        args: impl ExactSizeIterator<Item = impl ValueView + Display> + Clone,
        num_locals: aptos_gas_algebra::NumArgs,
    ) -> PartialVMResult<()> {
        self.inc_pc();
        let ty_tags = ty_args
            .clone()
            .map(|ty| ty.to_type_tag())
            .collect::<Vec<_>>();

        self.gen_new_frame(FrameName::Function {
            module_id: module_id.clone(),
            name: Identifier::new(func_name).unwrap(),
            ty_args: ty_tags,
        });
        self.base.charge_call_generic(module_id, func_name, ty_args, args, num_locals)
    }

    fn charge_ld_const(&mut self, size: aptos_gas_algebra::NumBytes) -> PartialVMResult<()> {
        self.inc_pc();
        self.base.charge_ld_const(size)
    }

    fn charge_ld_const_after_deserialization(
        &mut self,
        val: impl ValueView + Display,
    ) -> PartialVMResult<()> {
        self.inc_pc();
        self.base.charge_ld_const_after_deserialization(val)
    }

    fn charge_copy_loc(&mut self, val: impl ValueView + Display) -> PartialVMResult<()> {
        self.inc_pc();
        self.base.charge_copy_loc(val)
    }

    fn charge_move_loc(&mut self, val: impl ValueView + Display) -> PartialVMResult<()> {
        self.emit_instr_and_inc_pc(Opcodes::MOVE_LOC, std::iter::empty::<&Value>());
        self.base.charge_move_loc(val)
    }

    fn charge_store_loc(&mut self, val: impl ValueView, interpreter_view: impl InterpreterView) -> PartialVMResult<()> {
        self.emit_instr_and_inc_pc(Opcodes::ST_LOC, interpreter_view.view_last_n_values(1).unwrap());
        self.base.charge_store_loc(val, interpreter_view)
    }

    fn charge_pack(
        &mut self,
        is_generic: bool,
        args: impl ExactSizeIterator<Item = impl ValueView + Display> + Clone,
    ) -> PartialVMResult<()> {
        self.inc_pc();
        self.base.charge_pack(is_generic, args)
    }

    fn charge_unpack(
        &mut self,
        is_generic: bool,
        args: impl ExactSizeIterator<Item = impl ValueView + Display> + Clone,
    ) -> PartialVMResult<()> {
        self.emit_instr_and_inc_pc(Opcodes::UNPACK, args.clone());
        self.base.charge_unpack(is_generic, args)
    }

    fn charge_read_ref(&mut self, val: impl ValueView) -> PartialVMResult<()> {
        self.inc_pc();
        self.base.charge_read_ref(val)
    }

    fn charge_write_ref(
        &mut self,
        new_val: impl ValueView + Display,
        old_val: impl ValueView,
    ) -> PartialVMResult<()> {
        self.inc_pc();
        self.base.charge_write_ref(new_val, old_val)
    }

    fn charge_eq(&mut self, lhs: impl ValueView + Display, rhs: impl ValueView + Display) -> PartialVMResult<()> {
        self.inc_pc();
        self.base.charge_eq(lhs, rhs)
    }

    fn charge_neq(&mut self, lhs: impl ValueView + Display, rhs: impl ValueView + Display) -> PartialVMResult<()> {
        self.inc_pc();
        self.base.charge_neq(lhs, rhs)
    }

    fn charge_borrow_global(
        &mut self,
        is_mut: bool,
        is_generic: bool,
        ty: impl TypeView,
        is_success: bool,
    ) -> PartialVMResult<()> {
        self.inc_pc();
        self.base.charge_borrow_global(is_mut, is_generic, ty, is_success)
    }

    fn charge_exists(
        &mut self,
        is_generic: bool,
        ty: impl TypeView,
        // TODO(Gas): see if we can get rid of this param
        exists: bool,
    ) -> PartialVMResult<()> {
        self.emit_generic_instr_and_inc_pc(Opcodes::EXISTS, vec![ty.to_type_tag()], Vec::<&Value>::new().into_iter());
        self.base.charge_exists(is_generic, ty, exists)
    }

    fn charge_move_from(
        &mut self,
        is_generic: bool,
        ty: impl TypeView,
        val: Option<impl ValueView + Display>,
    ) -> PartialVMResult<()> {
        let ty_args = vec![ty.to_type_tag()];
        if let Some(val) = &val {
            self.emit_generic_instr_and_inc_pc(Opcodes::MOVE_FROM, ty_args, [&val].into_iter());
        } else {
            self.emit_generic_instr_and_inc_pc(Opcodes::MOVE_FROM, ty_args, Vec::<&Value>::new().into_iter());
        }
        self.base.charge_move_from(is_generic, ty, val)
    }

    fn charge_move_to(
        &mut self,
        is_generic: bool,
        ty: impl TypeView,
        val: impl ValueView,
        is_success: bool,
    ) -> PartialVMResult<()> {
        self.inc_pc();
        self.base.charge_move_to(is_generic, ty, val, is_success)
    }

    fn charge_vec_pack<'a>(
        &mut self,
        ty: impl TypeView + 'a,
        args: impl ExactSizeIterator<Item = impl ValueView + Display> + Clone,
    ) -> PartialVMResult<()> {
        self.inc_pc();
        self.base.charge_vec_pack(ty, args)
    }

    fn charge_vec_len(&mut self, ty: impl TypeView) -> PartialVMResult<()> {
        self.inc_pc();
        self.base.charge_vec_len(ty)
    }

    fn charge_vec_borrow(
        &mut self,
        is_mut: bool,
        ty: impl TypeView,
        is_success: bool,
    ) -> PartialVMResult<()> {
        self.inc_pc();
        self.base.charge_vec_borrow(is_mut, ty, is_success)
    }

    fn charge_vec_push_back(
        &mut self,
        ty: impl TypeView,
        val: impl ValueView + Display,
    ) -> PartialVMResult<()> {
        self.inc_pc();
        self.base.charge_vec_push_back(ty, val)
    }

    fn charge_vec_pop_back(
        &mut self,
        ty: impl TypeView,
        val: Option<impl ValueView + Display>,
    ) -> PartialVMResult<()> {
        self.inc_pc();
        self.base.charge_vec_pop_back(ty, val)
    }

    fn charge_vec_unpack(
        &mut self,
        ty: impl TypeView,
        expect_num_elements: aptos_gas_algebra::NumArgs,
        elems: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()> {
        self.inc_pc();
        self.base.charge_vec_unpack(ty, expect_num_elements, elems)
    }

    fn charge_vec_swap(&mut self, ty: impl TypeView) -> PartialVMResult<()> {
        self.inc_pc();
        self.base.charge_vec_swap(ty)
    }

    fn charge_load_resource(
        &mut self,
        addr: aptos_types::PeerId,
        ty: impl TypeView,
        val: Option<impl ValueView>,
        bytes_loaded: aptos_gas_algebra::NumBytes,
    ) -> PartialVMResult<()> {
        self.inc_pc();
        self.base.charge_load_resource(addr, ty, val, bytes_loaded)
    }

    fn charge_native_function(
        &mut self,
        amount: aptos_gas_algebra::InternalGas,
        ret_vals: Option<impl ExactSizeIterator<Item = impl ValueView + Display> + Clone>,
    ) -> PartialVMResult<()> {
        self.inc_pc();
        self.base.charge_native_function(amount, ret_vals)
    }

    fn charge_native_function_before_execution(
        &mut self,
        ty_args: impl ExactSizeIterator<Item = impl TypeView> + Clone,
        args: impl ExactSizeIterator<Item = impl ValueView + Display> + Clone,
    ) -> PartialVMResult<()> {
        self.inc_pc();
        self.base.charge_native_function_before_execution(ty_args, args)
    }

    fn charge_drop_frame(
        &mut self,
        locals: impl Iterator<Item = impl ValueView + Display> + Clone,
    ) -> PartialVMResult<()> {
        self.inc_pc();
        self.base.charge_drop_frame(locals)
    }

    fn charge_create_ty(
        &mut self,
        num_nodes: aptos_gas_algebra::NumTypeNodes,
    ) -> PartialVMResult<()> {
        self.base.charge_create_ty(num_nodes)
    }

    fn charge_dependency(
        &mut self,
        is_new: bool,
        addr: &aptos_types::PeerId,
        name: &move_core_types::identifier::IdentStr,
        size: aptos_gas_algebra::NumBytes,
    ) -> PartialVMResult<()> {
        self.base.charge_dependency(is_new, addr, name, size)
    }
}

impl<G> AptosGasMeter for ExecutionTracer<G, String>
where
    G: AptosGasMeter,
{
    type Algebra = G::Algebra;

    fn algebra(&self) -> &Self::Algebra {
        self.base.algebra()
    }

    fn algebra_mut(&mut self) -> &mut Self::Algebra {
        self.base.algebra_mut()
    }

    fn charge_storage_fee(
        &mut self,
        amount: aptos_gas_algebra::Fee,
        gas_unit_price: aptos_gas_algebra::FeePerGasUnit,
    ) -> PartialVMResult<()> {
        self.base.charge_storage_fee(amount, gas_unit_price)
    }

    fn charge_intrinsic_gas_for_transaction(&mut self, txn_size: aptos_gas_algebra::NumBytes) -> move_binary_format::errors::VMResult<()> {
        self.base.charge_intrinsic_gas_for_transaction(txn_size)
    }

    fn charge_keyless(&mut self) -> move_binary_format::errors::VMResult<()> {
        self.base.charge_keyless()
    }

    fn charge_io_gas_for_transaction(&mut self, txn_size: aptos_gas_algebra::NumBytes) -> move_binary_format::errors::VMResult<()> {
        self.base.charge_io_gas_for_transaction(txn_size)
    }

    fn charge_io_gas_for_event(&mut self, event: &aptos_types::contract_event::ContractEvent) -> move_binary_format::errors::VMResult<()> {
        self.base.charge_io_gas_for_event(event)
    }

    fn charge_io_gas_for_write(&mut self, key: &aptos_types::state_store::state_key::StateKey, op: &aptos_types::write_set::WriteOpSize) -> move_binary_format::errors::VMResult<()> {
        self.base.charge_io_gas_for_write(key, op)
    }
}
