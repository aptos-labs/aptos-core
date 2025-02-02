// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::mono::{
    code::MonoCode,
    context::{ExecutionContext, FunctionContext, FunctionId},
    memory::{Memory, Reference},
};
use move_binary_format::{errors::PartialVMResult, file_format::LocalIndex};

pub struct Executor<'ctx> {
    memory: Memory,
    call_stack: Vec<Frame<'ctx>>,
}

pub struct Frame<'ctx> {
    function_ctx: FunctionContext<'ctx>,
    program_counter: usize,
    stack_base: usize,
}

impl<'ctx> Executor<'ctx> {
    pub fn new(ctx: &'ctx dyn ExecutionContext) -> Self {
        Self {
            memory: Memory::new(ctx),
            call_stack: vec![],
        }
    }

    pub fn execute(
        &mut self,
        ctx: &'ctx dyn ExecutionContext,
        entry_fun: &FunctionId,
        argument_block: &[u8],
    ) -> PartialVMResult<Vec<u8>> {
        // Push argument block onto the value stack
        self.memory.push_blob(ctx, argument_block)?;
        // Create function frame and execute
        self.new_call_frame(ctx, entry_fun)?;
        self.run(ctx)
    }

    pub fn new_call_frame(
        &mut self,
        ctx: &'ctx dyn ExecutionContext,
        id: &FunctionId,
    ) -> PartialVMResult<()> {
        let function_ctx = ctx.fetch_function(id)?;
        // Add space for non-parameter locals on the value stack
        self.memory
            .push_uninit(ctx, function_ctx.locals_size - function_ctx.params_size)?;
        // Create a frame on the call stack
        let stack_base = self.memory.stack_len() - function_ctx.locals_size;
        self.call_stack.push(Frame {
            function_ctx,
            program_counter: 0,
            stack_base,
        });
        Ok(())
    }

    pub fn run(&mut self, ctx: &'ctx dyn ExecutionContext) -> PartialVMResult<Vec<u8>> {
        use MonoCode::*;

        loop {
            let frame = self.call_stack.last_mut().expect("call stack not empty");
            let instruction = &frame.function_ctx.code[frame.program_counter];
            frame.program_counter += 1;

            let local_ref = |idx: &LocalIndex| {
                let offset = frame.function_ctx.local_table[*idx as usize];
                Reference::local(frame.stack_base + offset)
            };

            match instruction {
                LoadConst { value } => {
                    self.memory.push_blob(ctx, value)?;
                },
                CopyLocal { size, local } => self.memory.push_from(ctx, local_ref(local), *size)?,
                StoreLocal { size, local } => self.memory.pop_to(ctx, local_ref(local), *size)?,
                BorrowLocal { local } => self.memory.push_value(ctx, local_ref(local))?,
                ReadRef { size } => {
                    let reference = self.memory.pop_value::<Reference>(ctx)?;
                    self.memory.push_from(ctx, reference, *size)?
                },
                WriteRef { size } => {
                    let reference = self.memory.pop_value::<Reference>(ctx)?;
                    self.memory.pop_to(ctx, reference, *size)?;
                },
                BorrowGlobal { storage_key } => {
                    let reference = self.memory.borrow_global(ctx, storage_key);
                    self.memory.push_value(ctx, reference)?
                },
                BorrowField { byte_offset } => {
                    let reference = self.memory.pop_value::<Reference>(ctx)?;
                    self.memory
                        .push_value(ctx, reference.select_field(*byte_offset))?;
                },
                Branch { offset } => frame.program_counter = *offset,
                BranchTrue { offset } => {
                    let cond = self.memory.pop_value::<bool>(ctx)?;
                    if cond {
                        frame.program_counter = *offset
                    }
                },
                BranchFalse { offset } => {
                    let cond = self.memory.pop_value::<bool>(ctx)?;
                    if !cond {
                        frame.program_counter = *offset
                    }
                },
                CallPrimitive { size, operation } => {
                    operation(ctx, &mut self.memory, *size)?;
                },
                CallFunction { function_id } => {
                    self.new_call_frame(ctx, function_id)?;
                },
                Return { size } => {
                    self.memory
                        .collapse(ctx, frame.function_ctx.locals_size, *size)?;
                    self.call_stack.pop();
                    if self.call_stack.is_empty() {
                        return Ok(self.memory.top_view(*size).view_as_slice().to_vec());
                    }
                },
            }
        }
    }
}
