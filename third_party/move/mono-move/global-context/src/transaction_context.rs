// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::ExecutionGuard;
use mono_move_alloc::ExecutableArenaPtr;
use mono_move_core::{ExecutableId, Function, FunctionResolver, TransactionContext};

// TODO:
//   This is a placeholder for per-txn data that can outlive multiple interpreter
//   invocations. We may want to move some things from InterpreterContext to here.
//   Gas counter will be here. Read-set recording will be here. All per-txn caches.
//   will be here as we..
pub struct PlaceholderContext<'ctx> {
    #[allow(dead_code)]
    guard: ExecutionGuard<'ctx>,
}

impl<'ctx> PlaceholderContext<'ctx> {
    pub fn new(guard: ExecutionGuard<'ctx>) -> Self {
        Self { guard }
    }
}

impl FunctionResolver for PlaceholderContext<'_> {
    fn resolve_function(
        &self,
        _executable_id: ExecutableArenaPtr<ExecutableId>,
        _name: ExecutableArenaPtr<str>,
    ) -> Option<&Function> {
        // TODO: implement once specializer support cross-module calls.
        todo!("PlaceholderContext::resolve_function")
    }
}

impl TransactionContext for PlaceholderContext<'_> {}
