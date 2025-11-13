// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Interpreter caches keep a record of cached data for each function call observed during
//! execution (e.g., caching [FrameTypeCache]). As a result, when calling function `foo<u64>`
//! at multiple call-sites, cached information from `foo<u64>` is re-used and only computed once
//! on the first call.

use crate::{
    frame_type_cache::FrameTypeCache,
    loader::{FunctionPtr, GenericFunctionPtr},
    LoadedFunction,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

/// Interpreter-level caches for function data (single-threaded)
pub struct InterpreterFunctionCaches {
    function_instruction_caches: HashMap<FunctionPtr, Rc<RefCell<FrameTypeCache>>>,
    generic_function_instruction_caches: HashMap<GenericFunctionPtr, Rc<RefCell<FrameTypeCache>>>,
}

impl InterpreterFunctionCaches {
    pub fn new() -> Self {
        Self {
            function_instruction_caches: HashMap::new(),
            generic_function_instruction_caches: HashMap::new(),
        }
    }

    pub(crate) fn get_or_create_frame_cache(
        &mut self,
        function: &LoadedFunction,
    ) -> Rc<RefCell<FrameTypeCache>> {
        if function.ty_args.is_empty() {
            self.get_or_create_frame_cache_non_generic(function)
        } else {
            self.get_or_create_frame_cache_generic(function)
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
        let ptr = GenericFunctionPtr::from_loaded_function(function);
        self.generic_function_instruction_caches
            .entry(ptr)
            .or_insert_with(|| FrameTypeCache::make_rc_for_function(function))
            .clone()
    }
}
