// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    frame_type_cache::{FrameTypeCache, RuntimeCacheTraits},
    Function, LoadedFunction,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    hash::{Hash, Hasher},
    rc::Rc,
    sync::Arc,
};

/// Stable pointer identity for a [Function] within a single interpreter invocation.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) struct FunctionPtr(*const Function);

impl FunctionPtr {
    pub(crate) fn from_loaded_function(function: &LoadedFunction) -> Self {
        FunctionPtr(Arc::as_ptr(&function.function))
    }
}

impl Hash for FunctionPtr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.0 as usize);
    }
}

/// Interpreter-level caches for function data (single-threaded)
pub struct InterpreterFunctionCaches {
    function_instruction_caches: HashMap<FunctionPtr, Rc<RefCell<FrameTypeCache>>>,
}

impl InterpreterFunctionCaches {
    pub fn new() -> Self {
        Self {
            function_instruction_caches: HashMap::new(),
        }
    }

    pub(crate) fn get_or_create_frame_cache<RTCaches: RuntimeCacheTraits>(
        &mut self,
        function: &LoadedFunction,
    ) -> Rc<RefCell<FrameTypeCache>> {
        if RTCaches::caches_enabled() {
            if function.ty_args.is_empty() {
                self.get_or_create_frame_cache_non_generic(function)
            } else {
                self.get_or_create_frame_cache_generic(function)
            }
        } else {
            FrameTypeCache::make_rc()
        }
    }

    /// Returns existing cache, or creates a new one for a non-generic function.
    pub(crate) fn get_or_create_frame_cache_non_generic(
        &mut self,
        function: &LoadedFunction,
    ) -> Rc<RefCell<FrameTypeCache>> {
        debug_assert!(function.ty_args().is_empty());

        let ptr = FunctionPtr::from_loaded_function(function);
        self.function_instruction_caches
            .entry(ptr)
            .or_insert_with(|| FrameTypeCache::make_rc_for_function(function))
            .clone()
    }

    /// Returns existing cache, or creates a new one for a generic function.
    pub(crate) fn get_or_create_frame_cache_generic(
        &mut self,
        function: &LoadedFunction,
    ) -> Rc<RefCell<FrameTypeCache>> {
        // TODO(caches): cache per instantiation!
        FrameTypeCache::make_rc_for_function(function)
    }
}
