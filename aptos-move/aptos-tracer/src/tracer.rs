// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_gas_meter::{AptosGasMeter, GasAlgebra};
use aptos_gas_profiling::FrameName;
use move_binary_format::{
    errors::PartialVMResult,
    file_format::{Bytecode, CodeOffset},
    file_format_common::Opcodes,
};
use move_bytecode_source_map::source_map::SourceMap;
use move_core_types::{
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
};
use move_model::model::{GlobalEnv, Loc};
use move_package::{BuildConfig, ModelConfig};
use move_vm_types::{
    gas::{GasMeter, InterpreterView, SimpleInstruction},
    values::Value,
    views::{TypeView, ValueView, ValueVisitor},
};
use std::{
    collections::VecDeque, env, fmt::Display, io::{BufRead, Write}, path::Path
};
#[derive(Debug, Clone)]
pub struct ExecutionTrace<Value>(CallFrame1<Value>);

impl<Value: Display> ExecutionTrace<Value> {
    pub fn debug_with_loc(
        &self,
        w: &mut impl Write,
        env: &GlobalEnv,
    ) -> Result<(), std::io::Error> {
        self.0.simple_debug1(w, 0, env)
    }

    pub fn simple_debug(&self, w: &mut impl Write) -> Result<(), std::io::Error> {
        self.0.simple_debug(w, 0)
    }
}

#[derive(Debug)]
pub struct ExecutionTracer<G, RequestReceiver, ResponseHandler> {
    base: G,
    frames: Vec<CallFrame>,
    request_receiver: RequestReceiver,
    response_handler: ResponseHandler,
    step_counter: usize,
    queue: VecDeque<Request>,
    pub env: Option<GlobalEnv>,
}

pub type Tracer<G, Value> = ExecutionTracer<G, IncrementalStepper, TraceCollector<Value>>;

pub fn new_tracer_from_entry_fun<G, Value>(
    base: G,
    module_id: ModuleId,
    function: Identifier,
    ty_args: Vec<TypeTag>,
    env: Option<GlobalEnv>,
) -> ExecutionTracer<G, IncrementalStepper, TraceCollector<Value>> {
    ExecutionTracer::from_entry_fun(
        base,
        module_id.clone(),
        function.clone(),
        ty_args.clone(),
        IncrementalStepper,
        TraceCollector::new_from_entry_fun(module_id, function, ty_args),
        env,
    )
}

pub fn debugger_from_entry_fun<G>(
    base: G,
    module_id: ModuleId,
    function: Identifier,
    ty_args: Vec<TypeTag>,
    env: Option<GlobalEnv>,
) -> ExecutionTracer<G, StandardIOCommandReader, ResponsePrinter> {
    ExecutionTracer::from_entry_fun(
        base,
        module_id,
        function,
        ty_args,
        new_standard_io_command_reader(),
        ResponsePrinter,
        env,
    )
}

impl<G, RequestReceiver, ResponseHandler> ExecutionTracer<G, RequestReceiver, ResponseHandler> {
    pub fn from_entry_fun(
        base: G,
        module_id: ModuleId,
        function: Identifier,
        ty_args: Vec<TypeTag>,
        request_receiver: RequestReceiver,
        response_handler: ResponseHandler,
        env: Option<GlobalEnv>,
    ) -> Self {
        Self {
            base,
            frames: vec![CallFrame::new_function(module_id, function, ty_args)],
            request_receiver,
            response_handler,
            queue: VecDeque::new(),
            step_counter: 1,
            env,
        }
    }

    pub fn from_script(base: G) -> Self {
        todo!()
    }

    fn get_top_frame(&self) -> &CallFrame {
        self.frames.last().expect("non-empty stack of frames")
    }

    fn get_cur_frame_name(&self) -> &FrameName {
        &self.get_top_frame().name
    }

    fn get_top_frame_mut(&mut self) -> &mut CallFrame {
        self.frames.last_mut().expect("non-empty stack of frames")
    }

    // fn get_top_events_mut(&mut self) -> &mut Vec<Event<Value>> {
    //     &mut self.get_top_frame_mut().events
    // }

    fn gen_new_frame(&mut self, name: FrameName) {
        self.frames.push(CallFrame { name, pc: 0 });
    }

    pub fn get_response_handler(&self) -> &ResponseHandler {
        &self.response_handler
    }

    pub fn get_response_handler_mut(&mut self) -> &mut ResponseHandler {
        &mut self.response_handler
    }

    // fn push_instr(&mut self, instr: Instruction<Value>) {
    //     self.get_top_events_mut().push(Event::Instruction(instr));
    // }

    // pub fn dump_trace(&mut self) -> ExecutionTrace<Value> {
    //     // debug_assert!(self.frames.len() == 1);
    //     // TODO: fix for abort
    //     ExecutionTrace(self.frames.pop().expect("non-empty stack of frames"))
    // }
}

impl<G, R: RequestReceiver, W: ResponseHandler<String>> ExecutionTracer<G, R, W> {
    /// Gets the location of the bytecode at the given offset of the current function.
    fn get_loc(&self, offset: CodeOffset) -> Option<Loc> {
        if let FrameName::Function {
            module_id, name, ..
        } = &self.get_top_frame().name
        {
            let env = self.env.as_ref()?;
            let fun_env = env.find_function_by_language_storage_id_name(module_id, name)?;
            let loc = fun_env.get_bytecode_loc(offset)?;
            return Some(loc);
        } else {
            None
        }
    }

    fn handle_instr(
        &mut self,
        op: Opcodes,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) {
        self.handle_generic_instr(op, vec![], args);
    }

    fn handle_generic_instr(
        &mut self,
        op: Opcodes,
        ty_args: Vec<TypeTag>,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) {
        let instr = self.gen_instr(op, ty_args, args);
        if self.step_counter == 1 {
            self.response_handler
                .handle_response(Response::InstructionExecuted(instr));
        }
        if self.step_counter == 0 {
            println!("impossible");
        } else {
            self.step_counter -= 1;
        }
    }

    fn handle_instr_and_inc_pc(
        &mut self,
        op: Opcodes,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) {
        self.handle_generic_instr_and_inc_pc(op, vec![], args)
    }

    fn handle_generic_instr_and_inc_pc(
        &mut self,
        op: Opcodes,
        ty_args: Vec<TypeTag>,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) {
        self.handle_generic_instr(op, ty_args, args);
        self.inc_pc();
    }

    fn gen_instr(
        &self,
        op: Opcodes,
        ty_args: Vec<TypeTag>,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) -> Instruction<String> {
        let pc = self.get_pc();
        Instruction {
            op,
            ty_args,
            args: args
                .map(|v| format!("{:?}", Into::<TValue>::into(v)))
                .collect(),
            frame_name: Some(self.get_cur_frame_name().clone()),
            offset: pc,
            loc: self.get_loc(pc),
        }
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

    fn handle_new_requests(&mut self) {
        debug_assert!(self.step_counter == 0);
        loop {
            let command = self.request_receiver.get_request();
            match command {
                Request::Step(n) => {
                    self.step_counter = n;
                    break;
                },
                Request::Continue => todo!(),
                Request::Help => {
                    println!("Usage:");
                    println!("step <n>: step n instructions");
                    println!("continue: continue execution");
                    println!("break <module_id> <function_name>: break at function");
                    println!("backtrace: print backtrace");
                },
                Request::Break(module_id, name) => todo!(),
                Request::Backtrace(_) => {
                    for (i, frame) in self.frames.iter().rev().enumerate() {
                        println!("{:2}: {}", i, frame.name);
                    }
                },
            }
        }
    }
}

/// Records the execution of an instruction
#[derive(Debug, Clone)]
pub struct Instruction<Value> {
    pub op: Opcodes,
    pub ty_args: Vec<TypeTag>,
    pub args: Vec<Value>,
    pub frame_name: Option<FrameName>,
    pub offset: CodeOffset,
    pub loc: Option<Loc>,
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
        QualifiedInstr {
            op: self.op,
            ty_args: &self.ty_args,
        }
    }
}

/// Records the execution of a function call
#[derive(Debug, Clone)]
pub struct CallFrame {
    pub name: FrameName,
    // pub events: Vec<Event<Value>>,
    pub pc: CodeOffset,
}

impl CallFrame {
    fn new_function(module_id: ModuleId, name: Identifier, ty_args: Vec<TypeTag>) -> Self {
        Self {
            name: FrameName::Function {
                module_id,
                name,
                ty_args,
            },
            // events: Vec::new(),
            pc: 0,
        }
    }

    fn new_script() -> Self {
        Self {
            name: FrameName::Script,
            // events: Vec::new(),
            pc: 0,
        }
    }
}

impl<Value: Display> CallFrame1<Value> {
    pub fn simple_debug(&self, w: &mut impl Write, depth: usize) -> Result<(), std::io::Error> {
        writeln!(w, "{}{}", " ".repeat(depth * 4), self.name)?;
        for event in &self.events {
            match event {
                Event1::Instruction(instr) => {
                    write!(w, "{}", " ".repeat(depth * 4 + 2))?;
                    write!(w, "{}: ", instr.offset)?;
                    write!(w, "{} ", instr.display_qualified_instr())?;
                    for arg in &instr.args {
                        write!(w, "{} ", arg)?;
                    }
                    writeln!(w)?;
                },
                Event1::Call(frame) => frame.simple_debug(w, depth + 1)?,
            }
        }
        Ok(())
    }

    pub fn simple_debug1(
        &self,
        w: &mut impl Write,
        depth: usize,
        env: &GlobalEnv,
    ) -> Result<(), std::io::Error> {
        writeln!(w, "{}{}", " ".repeat(depth * 4), self.name)?;
        for event in &self.events {
            match event {
                Event1::Instruction(instr) => {
                    write!(w, "{}", " ".repeat(depth * 4 + 2))?;
                    write!(w, "{}: ", instr.offset)?;
                    write!(w, "{} ", instr.display_qualified_instr())?;
                    for arg in &instr.args {
                        write!(w, "{} ", arg)?;
                    }
                    write!(w, "{}: ", instr.loc.as_ref().unwrap().display_file_name_and_line(env))?;
                    writeln!(w)?;
                },
                Event1::Call(frame) => frame.simple_debug1(w, depth + 1, env)?,
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Event<Value> {
    Call(CallFrame),
    Instruction(Instruction<Value>),
}

impl<G, R: RequestReceiver, W: ResponseHandler<String>> GasMeter for ExecutionTracer<G, R, W>
where
    G: AptosGasMeter,
{
    fn balance_internal(&self) -> aptos_gas_algebra::InternalGas {
        self.base.balance_internal()
    }

    fn charge_simple_instr(
        &mut self,
        instr: SimpleInstruction,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        match instr {
            SimpleInstruction::Nop => (),
            SimpleInstruction::Ret => {
                self.handle_instr_and_inc_pc(Opcodes::RET, std::iter::empty::<&Value>());
                self.response_handler.handle_response(Response::Ret);
                if self.frames.len() > 1 {
                    let _cur_frame = self.frames.pop().expect("frame must exist");
                }
            },
            SimpleInstruction::LdU8 => {
                self.handle_instr_and_inc_pc(Opcodes::LD_U8, std::iter::empty::<&Value>())
            },
            SimpleInstruction::LdU64 => {
                self.handle_instr_and_inc_pc(Opcodes::LD_U64, std::iter::empty::<&Value>())
            },
            SimpleInstruction::LdU128 => {
                self.handle_instr_and_inc_pc(Opcodes::LD_U128, std::iter::empty::<&Value>())
            },
            SimpleInstruction::LdTrue => {
                self.handle_instr_and_inc_pc(Opcodes::LD_TRUE, std::iter::empty::<&Value>())
            },
            SimpleInstruction::LdFalse => {
                self.handle_instr_and_inc_pc(Opcodes::LD_FALSE, std::iter::empty::<&Value>())
            },
            SimpleInstruction::FreezeRef => {
                self.handle_instr_and_inc_pc(Opcodes::FREEZE_REF, std::iter::empty::<&Value>())
            },
            SimpleInstruction::MutBorrowLoc => {
                self.handle_instr_and_inc_pc(Opcodes::MUT_BORROW_LOC, std::iter::empty::<&Value>())
            },
            SimpleInstruction::ImmBorrowLoc => {
                self.handle_instr_and_inc_pc(Opcodes::IMM_BORROW_LOC, std::iter::empty::<&Value>())
            },
            SimpleInstruction::ImmBorrowField => self.handle_instr_and_inc_pc(
                Opcodes::IMM_BORROW_FIELD,
                interpreter.view_last_n_values(1).unwrap(),
            ),
            SimpleInstruction::MutBorrowField => self.handle_instr_and_inc_pc(
                Opcodes::MUT_BORROW_FIELD,
                interpreter.view_last_n_values(1).unwrap(),
            ),
            SimpleInstruction::ImmBorrowFieldGeneric => self.handle_instr_and_inc_pc(
                Opcodes::IMM_BORROW_FIELD_GENERIC,
                interpreter.view_last_n_values(1).unwrap(),
            ),
            SimpleInstruction::MutBorrowFieldGeneric => self.handle_instr_and_inc_pc(
                Opcodes::MUT_BORROW_FIELD_GENERIC,
                interpreter.view_last_n_values(1).unwrap(),
            ),
            SimpleInstruction::ImmBorrowVariantField => self.handle_instr_and_inc_pc(
                Opcodes::IMM_BORROW_VARIANT_FIELD,
                interpreter.view_last_n_values(1).unwrap(),
            ),
            SimpleInstruction::MutBorrowVariantField => self.handle_instr_and_inc_pc(
                Opcodes::MUT_BORROW_VARIANT_FIELD,
                interpreter.view_last_n_values(1).unwrap(),
            ),
            SimpleInstruction::ImmBorrowVariantFieldGeneric => self.handle_instr_and_inc_pc(
                Opcodes::IMM_BORROW_VARIANT_FIELD_GENERIC,
                interpreter.view_last_n_values(1).unwrap(),
            ),
            SimpleInstruction::MutBorrowVariantFieldGeneric => self.handle_instr_and_inc_pc(
                Opcodes::MUT_BORROW_VARIANT_FIELD_GENERIC,
                interpreter.view_last_n_values(1).unwrap(),
            ),
            SimpleInstruction::TestVariant => self.handle_instr_and_inc_pc(
                Opcodes::TEST_VARIANT,
                interpreter.view_last_n_values(1).unwrap(),
            ),
            SimpleInstruction::TestVariantGeneric => self.handle_instr_and_inc_pc(
                Opcodes::TEST_VARIANT_GENERIC,
                interpreter.view_last_n_values(1).unwrap(),
            ),
            SimpleInstruction::CastU8 => self.handle_instr_and_inc_pc(
                Opcodes::CAST_U8,
                interpreter.view_last_n_values(1).unwrap(),
            ),
            SimpleInstruction::CastU64 => self.handle_instr_and_inc_pc(
                Opcodes::CAST_U64,
                interpreter.view_last_n_values(1).unwrap(),
            ),
            SimpleInstruction::CastU128 => self.handle_instr_and_inc_pc(
                Opcodes::CAST_U128,
                interpreter.view_last_n_values(1).unwrap(),
            ),
            SimpleInstruction::Add => self
                .handle_instr_and_inc_pc(Opcodes::ADD, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Sub => self
                .handle_instr_and_inc_pc(Opcodes::SUB, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Mul => self
                .handle_instr_and_inc_pc(Opcodes::MUL, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Mod => self
                .handle_instr_and_inc_pc(Opcodes::MOD, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Div => self
                .handle_instr_and_inc_pc(Opcodes::DIV, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::BitOr => self.handle_instr_and_inc_pc(
                Opcodes::BIT_OR,
                interpreter.view_last_n_values(2).unwrap(),
            ),
            SimpleInstruction::BitAnd => self.handle_instr_and_inc_pc(
                Opcodes::BIT_AND,
                interpreter.view_last_n_values(2).unwrap(),
            ),
            SimpleInstruction::Xor => self
                .handle_instr_and_inc_pc(Opcodes::XOR, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Shl => self
                .handle_instr_and_inc_pc(Opcodes::SHL, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Shr => self
                .handle_instr_and_inc_pc(Opcodes::SHR, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Or => self
                .handle_instr_and_inc_pc(Opcodes::OR, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::And => self
                .handle_instr_and_inc_pc(Opcodes::AND, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Not => self
                .handle_instr_and_inc_pc(Opcodes::NOT, interpreter.view_last_n_values(1).unwrap()),
            SimpleInstruction::Lt => self
                .handle_instr_and_inc_pc(Opcodes::LT, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Gt => self
                .handle_instr_and_inc_pc(Opcodes::GT, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Le => self
                .handle_instr_and_inc_pc(Opcodes::LE, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Ge => self
                .handle_instr_and_inc_pc(Opcodes::GE, interpreter.view_last_n_values(2).unwrap()),
            SimpleInstruction::Abort => self.handle_instr_and_inc_pc(
                Opcodes::ABORT,
                interpreter.view_last_n_values(1).unwrap(),
            ),
            SimpleInstruction::LdU16 => {
                self.handle_instr_and_inc_pc(Opcodes::LD_U16, std::iter::empty::<&Value>())
            },
            SimpleInstruction::LdU32 => {
                self.handle_instr_and_inc_pc(Opcodes::LD_U32, std::iter::empty::<&Value>())
            },
            SimpleInstruction::LdU256 => {
                self.handle_instr_and_inc_pc(Opcodes::LD_U256, std::iter::empty::<&Value>())
            },
            SimpleInstruction::CastU16 => self.handle_instr_and_inc_pc(
                Opcodes::CAST_U16,
                interpreter.view_last_n_values(1).unwrap(),
            ),
            SimpleInstruction::CastU32 => self.handle_instr_and_inc_pc(
                Opcodes::CAST_U32,
                interpreter.view_last_n_values(1).unwrap(),
            ),
            SimpleInstruction::CastU256 => self.handle_instr_and_inc_pc(
                Opcodes::CAST_U256,
                interpreter.view_last_n_values(1).unwrap(),
            ),
        }
        let res = self.base.charge_simple_instr(instr, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_br_true(
        &mut self,
        target_offset: Option<CodeOffset>,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.handle_instr(Opcodes::BR_TRUE, Vec::<&Value>::new().into_iter());
        if let Some(offset) = target_offset {
            *self.get_pc_mut() = offset;
        } else {
            *self.get_pc_mut() += 1;
        }
        let res = self.base.charge_br_true(target_offset, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_br_false(
        &mut self,
        target_offset: Option<CodeOffset>,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.handle_instr(Opcodes::BR_FALSE, Vec::<&Value>::new().into_iter());
        if let Some(offset) = target_offset {
            *self.get_pc_mut() = offset;
        } else {
            *self.get_pc_mut() += 1;
        }
        let res = self.base.charge_br_false(target_offset, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_branch(
        &mut self,
        target_offset: CodeOffset,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.handle_instr(Opcodes::BRANCH, Vec::<&Value>::new().into_iter());
        *self.get_pc_mut() = target_offset;
        let res = self.base.charge_branch(target_offset, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_pop(
        &mut self,
        popped_val: impl ValueView,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.handle_generic_instr_and_inc_pc(Opcodes::POP, vec![], [&popped_val].into_iter());
        let res = self.base.charge_pop(popped_val, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_call(
        &mut self,
        module_id: &move_core_types::language_storage::ModuleId,
        func_name: &str,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
        num_locals: aptos_gas_algebra::NumArgs,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.handle_instr_and_inc_pc(Opcodes::CALL, args.clone().into_iter());
        let new_frame = FrameName::Function {
            module_id: module_id.clone(),
            name: Identifier::new(func_name).unwrap(),
            ty_args: vec![],
        };
        self.gen_new_frame(new_frame.clone());
        self.response_handler.handle_response(Response::NewFrame(new_frame));
        let res = self
            .base
            .charge_call(module_id, func_name, args, num_locals, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_call_generic(
        &mut self,
        module_id: &move_core_types::language_storage::ModuleId,
        func_name: &str,
        ty_args: impl ExactSizeIterator<Item = impl TypeView> + Clone,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
        num_locals: aptos_gas_algebra::NumArgs,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        let ty_tags = ty_args
            .clone()
            .map(|ty| ty.to_type_tag())
            .collect::<Vec<_>>();

        self.handle_generic_instr_and_inc_pc(
            Opcodes::CALL,
            ty_tags.clone(),
            args.clone().into_iter(),
        );
        self.gen_new_frame(FrameName::Function {
            module_id: module_id.clone(),
            name: Identifier::new(func_name).unwrap(),
            ty_args: ty_tags,
        });
        let res = self.base.charge_call_generic(
            module_id,
            func_name,
            ty_args,
            args,
            num_locals,
            interpreter,
        );
        self.handle_new_requests();
        res
    }

    fn charge_ld_const(
        &mut self,
        size: aptos_gas_algebra::NumBytes,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.handle_instr_and_inc_pc(Opcodes::LD_CONST, std::iter::empty::<&Value>());
        let res = self.base.charge_ld_const(size, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_ld_const_after_deserialization(
        &mut self,
        val: impl ValueView,
    ) -> PartialVMResult<()> {
        let res = self.base.charge_ld_const_after_deserialization(val);
        self.handle_new_requests();
        res
    }

    fn charge_copy_loc(
        &mut self,
        val: impl ValueView,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.handle_instr_and_inc_pc(Opcodes::COPY_LOC, std::iter::empty::<&Value>());
        let res = self.base.charge_copy_loc(val, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_move_loc(
        &mut self,
        val: impl ValueView,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.handle_instr_and_inc_pc(Opcodes::MOVE_LOC, std::iter::empty::<&Value>());
        let res = self.base.charge_move_loc(val, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_store_loc(
        &mut self,
        val: impl ValueView,
        interpreter_view: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.handle_instr_and_inc_pc(Opcodes::ST_LOC, std::iter::once(&val));
        let res = self.base.charge_store_loc(val, interpreter_view);
        self.handle_new_requests();
        res
    }

    fn charge_pack(
        &mut self,
        is_generic: bool,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
        interpreter_view: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.handle_instr_and_inc_pc(Opcodes::PACK, args.clone().into_iter());
        let res = self.base.charge_pack(is_generic, args, interpreter_view);
        self.handle_new_requests();
        res
    }

    fn charge_unpack(
        &mut self,
        is_generic: bool,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        // TODO: this is technically wrong, the argument should be the struct value instead of the fields
        self.handle_instr_and_inc_pc(Opcodes::UNPACK, args.clone());
        let res = self.base.charge_unpack(is_generic, args, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_read_ref(
        &mut self,
        val: impl ValueView,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.handle_instr_and_inc_pc(Opcodes::READ_REF, std::iter::once(&val));
        let res = self.base.charge_read_ref(val, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_write_ref(
        &mut self,
        new_val: impl ValueView,
        old_val: impl ValueView,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.handle_instr_and_inc_pc(
            Opcodes::WRITE_REF,
            // TODO
            std::iter::empty::<&Value>(),
        );
        let res = self.base.charge_write_ref(new_val, old_val, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_eq(
        &mut self,
        lhs: impl ValueView,
        rhs: impl ValueView,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.handle_instr_and_inc_pc(
            Opcodes::EQ,
            // TODO
            std::iter::empty::<&Value>(),
        );
        let res = self.base.charge_eq(lhs, rhs, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_neq(
        &mut self,
        lhs: impl ValueView,
        rhs: impl ValueView,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.handle_instr_and_inc_pc(
            Opcodes::NEQ,
            // TODO
            std::iter::empty::<&Value>(),
        );
        let res = self.base.charge_neq(lhs, rhs, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_borrow_global(
        &mut self,
        is_mut: bool,
        is_generic: bool,
        ty: impl TypeView,
        is_success: bool,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.handle_generic_instr_and_inc_pc(
            if is_mut {
                Opcodes::MUT_BORROW_GLOBAL
            } else {
                Opcodes::IMM_BORROW_GLOBAL
            },
            vec![ty.to_type_tag()],
            // TODO
            std::iter::empty::<&Value>(),
        );
        let res = self
            .base
            .charge_borrow_global(is_mut, is_generic, ty, is_success, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_exists(
        &mut self,
        is_generic: bool,
        ty: impl TypeView,
        // TODO(Gas): see if we can get rid of this param
        exists: bool,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.handle_generic_instr_and_inc_pc(
            Opcodes::EXISTS,
            vec![ty.to_type_tag()],
            Vec::<&Value>::new().into_iter(),
        );
        let res = self.base.charge_exists(is_generic, ty, exists, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_move_from(
        &mut self,
        is_generic: bool,
        ty: impl TypeView,
        val: Option<impl ValueView>,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        let ty_args = vec![ty.to_type_tag()];
        self.handle_generic_instr_and_inc_pc(
            Opcodes::MOVE_FROM,
            ty_args,
            // TODO
            std::iter::empty::<&Value>(),
        );
        let res = self.base.charge_move_from(is_generic, ty, val, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_move_to(
        &mut self,
        is_generic: bool,
        ty: impl TypeView,
        val: impl ValueView,
        is_success: bool,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.handle_generic_instr_and_inc_pc(
            Opcodes::MOVE_TO,
            vec![ty.to_type_tag()],
            std::iter::once(&val),
        );
        let res = self
            .base
            .charge_move_to(is_generic, ty, val, is_success, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_vec_pack<'b>(
        &mut self,
        ty: impl TypeView + 'b,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.handle_generic_instr_and_inc_pc(
            Opcodes::VEC_PACK,
            vec![ty.to_type_tag()],
            args.clone().into_iter(),
        );
        let res = self.base.charge_vec_pack(ty, args, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_vec_len(
        &mut self,
        ty: impl TypeView,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.handle_generic_instr_and_inc_pc(
            Opcodes::VEC_LEN,
            vec![ty.to_type_tag()],
            // TODO
            std::iter::empty::<&Value>(),
        );
        let res = self.base.charge_vec_len(ty, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_vec_borrow(
        &mut self,
        is_mut: bool,
        ty: impl TypeView,
        is_success: bool,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.handle_generic_instr_and_inc_pc(
            if is_mut {
                Opcodes::VEC_MUT_BORROW
            } else {
                Opcodes::VEC_IMM_BORROW
            },
            vec![ty.to_type_tag()],
            // TODO
            std::iter::empty::<&Value>(),
        );
        let res = self
            .base
            .charge_vec_borrow(is_mut, ty, is_success, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_vec_push_back(
        &mut self,
        ty: impl TypeView,
        val: impl ValueView,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.handle_generic_instr_and_inc_pc(
            Opcodes::VEC_PUSH_BACK,
            vec![ty.to_type_tag()],
            std::iter::once(&val),
        );
        let res = self.base.charge_vec_push_back(ty, val, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_vec_pop_back(
        &mut self,
        ty: impl TypeView,
        val: Option<impl ValueView>,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.handle_generic_instr_and_inc_pc(
            Opcodes::VEC_POP_BACK,
            vec![ty.to_type_tag()],
            // TODO
            std::iter::empty::<&Value>(),
        );
        let res = self.base.charge_vec_pop_back(ty, val, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_vec_unpack(
        &mut self,
        ty: impl TypeView,
        expect_num_elements: aptos_gas_algebra::NumArgs,
        elems: impl ExactSizeIterator<Item = impl ValueView> + Clone,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.handle_generic_instr_and_inc_pc(
            Opcodes::VEC_UNPACK,
            vec![ty.to_type_tag()],
            elems.clone().into_iter(),
        );
        let res = self
            .base
            .charge_vec_unpack(ty, expect_num_elements, elems, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_vec_swap(
        &mut self,
        ty: impl TypeView,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.handle_generic_instr_and_inc_pc(
            Opcodes::VEC_SWAP,
            vec![ty.to_type_tag()],
            // TODO
            std::iter::empty::<&Value>(),
        );
        let res = self.base.charge_vec_swap(ty, interpreter);
        self.handle_new_requests();
        res
    }

    fn charge_load_resource(
        &mut self,
        addr: aptos_types::PeerId,
        ty: impl TypeView,
        val: Option<impl ValueView>,
        bytes_loaded: aptos_gas_algebra::NumBytes,
    ) -> PartialVMResult<()> {
        let res = self.base.charge_load_resource(addr, ty, val, bytes_loaded);
        res
    }

    fn charge_native_function(
        &mut self,
        amount: aptos_gas_algebra::InternalGas,
        ret_vals: Option<impl ExactSizeIterator<Item = impl ValueView> + Clone>,
        interpreter: impl InterpreterView,
    ) -> PartialVMResult<()> {
        self.inc_pc();
        let res = self
            .base
            .charge_native_function(amount, ret_vals, interpreter);
        res
    }

    fn charge_native_function_before_execution(
        &mut self,
        ty_args: impl ExactSizeIterator<Item = impl TypeView> + Clone,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()> {
        self.base
            .charge_native_function_before_execution(ty_args, args)
    }

    fn charge_drop_frame(
        &mut self,
        locals: impl Iterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()> {
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

impl<G, R: RequestReceiver, W: ResponseHandler<String>> AptosGasMeter for ExecutionTracer<G, R, W>
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

    fn charge_intrinsic_gas_for_transaction(
        &mut self,
        txn_size: aptos_gas_algebra::NumBytes,
    ) -> move_binary_format::errors::VMResult<()> {
        self.base.charge_intrinsic_gas_for_transaction(txn_size)
    }

    fn charge_keyless(&mut self) -> move_binary_format::errors::VMResult<()> {
        self.base.charge_keyless()
    }

    fn charge_io_gas_for_transaction(
        &mut self,
        txn_size: aptos_gas_algebra::NumBytes,
    ) -> move_binary_format::errors::VMResult<()> {
        self.base.charge_io_gas_for_transaction(txn_size)
    }

    fn charge_io_gas_for_event(
        &mut self,
        event: &aptos_types::contract_event::ContractEvent,
    ) -> move_binary_format::errors::VMResult<()> {
        self.base.charge_io_gas_for_event(event)
    }

    fn charge_io_gas_for_write(
        &mut self,
        key: &aptos_types::state_store::state_key::StateKey,
        op: &aptos_types::write_set::WriteOpSize,
    ) -> move_binary_format::errors::VMResult<()> {
        self.base.charge_io_gas_for_write(key, op)
    }
}

#[derive(Debug)]
pub enum Request {
    Step(usize),
    Continue,
    Help,
    Break(ModuleId, Identifier),
    Backtrace(Option<usize>),
}

pub enum Response<Value> {
    InstructionExecuted(Instruction<Value>),
    NewFrame(FrameName),
    Ret,
}

pub trait RequestReceiver {
    fn get_request(&mut self) -> Request;
}

pub trait ResponseHandler<Value> {
    fn handle_response(&mut self, response: Response<Value>);
}

pub struct PrintResponse;

// impl<Value: Display> ResponseHandler<Value> for PrintResponse {
//     fn handle_response(&mut self, response: Response<Value>) {
//         match response {

//         }
//     }
// }

// pub trait Client<Value>: RequestReceiver + ResponseHandler<Value=Value> {}

pub struct IncrementalStepper;

impl RequestReceiver for IncrementalStepper {
    fn get_request(&mut self) -> Request {
        Request::Step(1)
    }
}

pub struct CommandReader<R, W> {
    reader: R,
    writer: W,
}

impl<R: BufRead, W: Write> RequestReceiver for CommandReader<R, W> {
    fn get_request(&mut self) -> Request {
        let mut line = String::new();
        write!(self.writer, "> ").expect("Failed to write");
        self.writer.flush().expect("Failed to flush");
        self.reader.read_line(&mut line).expect("Invalid character");
        match line.trim() {
            "s" | "step" => Request::Step(1),
            "c" | "continue" => Request::Continue,
            "h" | "help" => Request::Help,
            "b" | "backtrace" => Request::Backtrace(None),
            x => {
                if x.starts_with("step") {
                    Request::Step(x[4..].trim().parse().expect("Invalid step count"))
                } else {
                    self.get_request()
                }
            },
        }
    }
}

pub type StandardIOCommandReader = CommandReader<std::io::BufReader<std::io::Stdin>, std::io::Stdout>;

pub fn new_standard_io_command_reader() -> StandardIOCommandReader {
    CommandReader {
        reader: std::io::BufReader::new(std::io::stdin()),
        writer: std::io::stdout(),
    }
}

pub fn get_env() -> GlobalEnv {
    let path_str = env::var("PKG_PATH").expect("PKG_PATH must be set");
    let path = Path::new(&path_str);
    let mut build_config = BuildConfig::default();
    build_config.generate_move_model = true;
    build_config.dev_mode = true;
    let (_, env) = build_config
        .compile_package_no_exit(path, vec![], &mut std::io::stdout())
        .unwrap();
    env.unwrap()
}

#[derive(Debug)]
enum TValue {
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    Struct(Vec<TValue>),
    VecU8(Vec<TValue>),
    Vec(Vec<TValue>),
    Ref(Vec<TValue>), // TODO
}

impl<T: ValueView> From<T> for TValue {
    fn from(val: T) -> Self {
        let mut visitor = Visitor::new();
        val.visit(&mut visitor);
        visitor.finish()
    }
}

/// Convert a `ValueView` back into a value.
struct Visitor {
    stack: Vec<TValue>,
}

impl Visitor {
    fn new() -> Self {
        Self { stack: Vec::new() }
    }
}

impl Visitor {
    fn return_1(&mut self) {
        let top = self.stack.pop().unwrap();
        if let Some(last) = self.stack.last_mut() {
            match last {
                TValue::Struct(fields) => fields.push(top),
                TValue::Vec(elems) => elems.push(top),
                TValue::VecU8(elems) => elems.push(top),
                TValue::Ref(elems) => elems.push(top),
                _ => panic!(),
            }
        } else {
            panic!();
        }
    }

    fn return_to(&mut self, len: usize) {
        while self.stack.len() > len {
            self.return_1();
        }
    }

    fn finish(mut self) -> TValue {
        self.return_to(1);
        self.stack.pop().unwrap()
    }
}

impl ValueVisitor for Visitor {
    fn visit_delayed(
        &mut self,
        depth: usize,
        id: move_vm_types::delayed_values::delayed_field_id::DelayedFieldID,
    ) {
        todo!()
    }

    fn visit_u8(&mut self, depth: usize, val: u8) {
        self.return_to(depth);
        self.stack.push(TValue::U8(val));
    }

    fn visit_u16(&mut self, depth: usize, val: u16) {
        self.return_to(depth);
        self.stack.push(TValue::U16(val));
    }

    fn visit_u32(&mut self, depth: usize, val: u32) {
        self.return_to(depth);
        self.stack.push(TValue::U32(val));
    }

    fn visit_u64(&mut self, depth: usize, val: u64) {
        self.return_to(depth);
        self.stack.push(TValue::U64(val));
    }

    fn visit_u128(&mut self, depth: usize, val: u128) {
        todo!()
    }

    fn visit_u256(&mut self, depth: usize, val: move_core_types::u256::U256) {
        todo!()
    }

    fn visit_bool(&mut self, depth: usize, val: bool) {
        self.return_to(depth);
        self.stack.push(TValue::Bool(val));
    }

    fn visit_address(&mut self, depth: usize, val: aptos_types::PeerId) {
        todo!()
    }

    fn visit_struct(&mut self, depth: usize, len: usize) -> bool {
        self.return_to(depth);
        self.stack.push(TValue::Struct(Vec::new()));
        true
    }

    fn visit_vec(&mut self, depth: usize, len: usize) -> bool {
        self.return_to(depth);
        self.stack.push(TValue::Vec(Vec::new()));
        true
    }

    fn visit_ref(&mut self, depth: usize, is_global: bool) -> bool {
        self.return_to(depth);
        self.stack.push(TValue::Ref(Vec::new()));
        true
    }
}

struct ResponsePrinter;

impl<T: Display> ResponseHandler<T> for ResponsePrinter {

    fn handle_response(&mut self, response: Response<T>) {
        match response {
            Response::InstructionExecuted(instruction) => {
                print!(
                    "{}[{}]: ",
                    instruction.frame_name.as_ref().unwrap(),
                    instruction.offset
                );
                print!("{} ", instruction.display_qualified_instr());
                for arg in &instruction.args {
                    print!("{} ", arg);
                }
                println!();
            },
            Response::NewFrame(_frame_name) => (),
            Response::Ret => (),
        }
    }
}

/// Records the execution of a function call
#[derive(Debug, Clone)]
pub struct CallFrame1<Value> {
    pub name: FrameName,
    pub events: Vec<Event1<Value>>,
}

impl<Value> CallFrame1<Value> {
    pub fn new_function(module_id: ModuleId, name: Identifier, ty_args: Vec<TypeTag>) -> Self {
        Self { name: FrameName::Function { module_id, name, ty_args }, events: Vec::new() }
    }
}

#[derive(Debug, Clone)]
pub enum Event1<Value> {
    Call(CallFrame1<Value>),
    Instruction(Instruction<Value>),
}

pub struct TraceCollector<Value> {
    frames: Vec<CallFrame1<Value>>,
}

impl<Value> TraceCollector<Value> {
    pub fn new_from_entry_fun(module_id: ModuleId, function: Identifier, ty_args: Vec<TypeTag>) -> Self {
        Self { frames: vec![CallFrame1::new_function(module_id, function, ty_args)] }
    }

    pub fn new() -> Self {
        Self { frames: Vec::new() }
    }

    fn get_cur_frame_mut(&mut self) -> &mut CallFrame1<Value> {
        self.frames.last_mut().unwrap()
    }

    fn get_cur_events_mut(&mut self) -> &mut Vec<Event1<Value>> {
        self.get_cur_frame_mut().events.as_mut()
    }

    fn record_instr(&mut self, instr: Instruction<Value>) {
        self.get_cur_events_mut().push(Event1::Instruction(instr));
    }

    fn gen_new_frame(&mut self, name: FrameName) {
        self.frames.push(CallFrame1 {
            name,
            events: Vec::new(),
        });
    }

    fn ret(&mut self) {
        if self.frames.len() == 1 {
            return;
        }
        let frame = self.frames.pop().unwrap();
        self.get_cur_events_mut().push(Event1::Call(frame));
    }

    pub fn dump_trace(&mut self) -> ExecutionTrace<Value> {
        // debug_assert!(self.frames.len() == 1);
        // TODO: fix for abort
        ExecutionTrace(self.frames.pop().expect("non-empty stack of frames"))
    }
}

impl<Value> ResponseHandler<Value> for TraceCollector<Value> {
    fn handle_response(&mut self, response: Response<Value>) {
        match response {
            Response::InstructionExecuted(instruction) => {
                self.record_instr(instruction);
            },
            Response::NewFrame(frame_name) => {
                self.gen_new_frame(frame_name);
            },
            Response::Ret => {
                self.ret();
            },
        }
    }
}
