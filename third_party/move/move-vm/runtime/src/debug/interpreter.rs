// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    frame::Frame,
    interpreter::InterpreterImpl,
    module_traversal::{TraversalContext, TraversalStorage},
    source_locator, Loader, RuntimeEnvironment,
};
use move_binary_format::errors::{ExecutionState, PartialVMResult};
use move_vm_types::{
    debug_write, debug_writeln,
    gas::UnmeteredGasMeter,
    loaded_data::{runtime_types::StructType, struct_name_indexing::StructNameIndex},
};
use std::{cmp::min, fmt::Write, sync::Arc};

pub(crate) trait InterpreterDebugInterface {
    fn get_stack_frames(&self, count: usize) -> ExecutionState;
    #[allow(unused)]
    fn get_stack_depth(&self) -> usize;
    fn debug_print_stack_trace(
        &self,
        buf: &mut String,
        runtime_environment: &RuntimeEnvironment,
    ) -> PartialVMResult<()>;
    /// Required for the `Type::Struct { idx } -> StructType` transformation, to retrieve field types in debugger.
    fn load_struct_type(&self, idx: &StructNameIndex) -> Option<Arc<StructType>>;
}

impl<LoaderImpl> InterpreterDebugInterface for InterpreterImpl<'_, LoaderImpl>
where
    LoaderImpl: Loader,
{
    #[cold]
    fn debug_print_stack_trace(
        &self,
        buf: &mut String,
        runtime_environment: &RuntimeEnvironment,
    ) -> PartialVMResult<()> {
        debug_writeln!(buf, "Call Stack:")?;
        for (i, frame) in self.call_stack.0.iter().enumerate() {
            self.debug_print_frame(buf, runtime_environment, i, frame)?;
        }
        debug_writeln!(buf, "Operand Stack:")?;
        for (idx, val) in self.operand_stack.value.iter().enumerate() {
            // TODO: Currently we do not know the types of the values on the operand stack.
            // Revisit.
            debug_write!(buf, "    [{}] ", idx)?;
            source_locator::print_value(buf, val)?;
            debug_writeln!(buf)?;
        }
        Ok(())
    }

    /// Get count stack frames starting from the top of the stack.
    fn get_stack_frames(&self, count: usize) -> ExecutionState {
        // collect frames in the reverse order as this is what is
        // normally expected from the stack trace (outermost frame
        // is the last one)
        let stack_trace = self
            .call_stack
            .0
            .iter()
            .rev()
            .take(count)
            .map(|frame| {
                (
                    frame.function.module_id().cloned(),
                    frame.function.index(),
                    frame.pc,
                )
            })
            .collect();
        ExecutionState::new(stack_trace)
    }

    fn get_stack_depth(&self) -> usize {
        self.call_stack.0.len()
    }

    fn load_struct_type(&self, idx: &StructNameIndex) -> Option<Arc<StructType>> {
        let storage = TraversalStorage::new();
        let mut ctx = TraversalContext::new(&storage);
        self.loader
            .load_struct_definition(&mut UnmeteredGasMeter, &mut ctx, idx)
            .ok()
    }
}

impl<LoaderImpl> InterpreterImpl<'_, LoaderImpl>
where
    LoaderImpl: Loader,
{
    pub(crate) fn debug_print_frame<B: Write>(
        &self,
        buf: &mut B,
        runtime_environment: &RuntimeEnvironment,
        idx: usize,
        frame: &Frame,
    ) -> PartialVMResult<()> {
        debug_write!(buf, "    [{}] ", idx)?;

        // Print out the function name.
        let function = &frame.function;
        debug_write!(buf, "{}", function.name_as_pretty_string())?;

        // Print out type arguments, if they exist.
        let ty_args = function.ty_args();
        if !ty_args.is_empty() {
            let mut ty_tags = vec![];
            for ty in ty_args {
                let tag = runtime_environment.ty_to_ty_tag(ty)?;
                ty_tags.push(tag);
            }
            debug_write!(buf, "<")?;
            let mut it = ty_tags.iter();
            if let Some(tag) = it.next() {
                debug_write!(buf, "{}", tag.to_canonical_string())?;
                for tag in it {
                    debug_write!(buf, ", ")?;
                    debug_write!(buf, "{}", tag.to_canonical_string())?;
                }
            }
            debug_write!(buf, ">")?;
        }
        debug_writeln!(buf)?;

        // Print source location if available.
        if let Some(module_id) = function.module_id() {
            if let Some(loc) =
                source_locator::get_bytecode_source_location(module_id, function.index(), frame.pc)
            {
                debug_writeln!(buf, "          at {}", loc)?;
            }
        }

        // Print out the current instruction.
        debug_writeln!(buf)?;
        debug_writeln!(buf, "        Code:")?;
        let pc = frame.pc as usize;
        let code = function.code();
        let before = pc.saturating_sub(3);
        let after = min(code.len(), pc + 4);
        for (idx, instr) in code.iter().enumerate().take(pc).skip(before) {
            debug_writeln!(buf, "            [{}] {:?}", idx, instr)?;
        }
        debug_writeln!(buf, "          > [{}] {:?}", pc, &code[pc])?;
        for (idx, instr) in code.iter().enumerate().take(after).skip(pc + 1) {
            debug_writeln!(buf, "            [{}] {:?}", idx, instr)?;
        }

        // Print out the locals.
        debug_writeln!(buf)?;
        if function.local_tys().is_empty() {
            debug_writeln!(buf, "        Locals:")?;
            debug_writeln!(buf, "            (none)")?;
        } else {
            source_locator::print_locals_enriched(
                buf,
                function,
                &frame.locals,
                runtime_environment,
                self,
                true,
            )?;
            debug_writeln!(buf)?;
        }

        debug_writeln!(buf)?;
        Ok(())
    }
}
