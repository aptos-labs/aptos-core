// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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

/// Maximum number of per-instruction cache entries one interpreter invocation may allocate,
/// summed over its frame caches. There is one frame cache per distinct
/// `(function, type arguments)` pair, and one for an N-instruction function consumes N entries.
///
/// Sized to sit above demand and below capacity. At 24 bytes per entry, max entries cost
/// ~24 MB per invocation for one thread. Exceeding it only costs speed; see
/// [InstructionCacheBudget].
const MAX_PER_INSTRUCTION_CACHE_ENTRIES: usize = 1_000_000;

/// Bounds the number of per-instruction cache entries allocated by one interpreter invocation.
///
/// Refusing an allocation is not observable in gas or in execution results. A frame cache without
/// a per-instruction cache falls through to [FrameTypeCache]'s function caches, which resolve the
/// same function, hold the same frame cache, and charge the same gas. The budget therefore trades
/// speed for a hard memory bound, and needs no feature gate.
pub(crate) struct InstructionCacheBudget {
    remaining: usize,
}

impl InstructionCacheBudget {
    pub(crate) fn new() -> Self {
        Self {
            remaining: max_per_instruction_cache_entries(),
        }
    }

    /// Reserves `num_entries`. Returns false, leaving the budget untouched, if it cannot cover the
    /// request.
    pub(crate) fn try_reserve(&mut self, num_entries: usize) -> bool {
        match self.remaining.checked_sub(num_entries) {
            Some(remaining) => {
                self.remaining = remaining;
                record_entries_allocated(num_entries);
                true
            },
            None => false,
        }
    }
}

#[cfg(not(any(test, feature = "testing")))]
fn record_entries_allocated(_num_entries: usize) {}

#[cfg(any(test, feature = "testing"))]
fn record_entries_allocated(num_entries: usize) {
    ENTRIES_ALLOCATED.with(|allocated| allocated.set(allocated.get().saturating_add(num_entries)));
}

#[cfg(not(any(test, feature = "testing")))]
fn max_per_instruction_cache_entries() -> usize {
    MAX_PER_INSTRUCTION_CACHE_ENTRIES
}

#[cfg(any(test, feature = "testing"))]
fn max_per_instruction_cache_entries() -> usize {
    INSTRUCTION_CACHE_BUDGET_OVERRIDE.with(|budget| budget.get())
}

#[cfg(any(test, feature = "testing"))]
thread_local! {
    static INSTRUCTION_CACHE_BUDGET_OVERRIDE: std::cell::Cell<usize> =
        const { std::cell::Cell::new(MAX_PER_INSTRUCTION_CACHE_ENTRIES) };

    static ENTRIES_ALLOCATED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Overrides the per-instruction cache budget on the current thread until dropped, and counts the
/// entries allocated while it is installed.
///
/// Because the budget is invisible to gas and to execution results, tests can vary it freely and
/// assert that outputs are unchanged. Setting it to [usize::MAX] reproduces the unbounded
/// allocation that preceded the bound.
#[cfg(any(test, feature = "testing"))]
pub struct InstructionCacheBudgetOverride {
    previous_max_entries: usize,
    previous_entries_allocated: usize,
}

#[cfg(any(test, feature = "testing"))]
impl InstructionCacheBudgetOverride {
    pub fn new(max_entries: usize) -> Self {
        Self {
            previous_max_entries: INSTRUCTION_CACHE_BUDGET_OVERRIDE
                .with(|budget| budget.replace(max_entries)),
            previous_entries_allocated: ENTRIES_ALLOCATED.with(|allocated| allocated.replace(0)),
        }
    }

    /// Per-instruction cache entries allocated on this thread since the override was installed.
    pub fn entries_allocated(&self) -> usize {
        ENTRIES_ALLOCATED.with(|allocated| allocated.get())
    }
}

#[cfg(any(test, feature = "testing"))]
impl Drop for InstructionCacheBudgetOverride {
    fn drop(&mut self) {
        INSTRUCTION_CACHE_BUDGET_OVERRIDE.with(|budget| budget.set(self.previous_max_entries));
        ENTRIES_ALLOCATED.with(|allocated| allocated.set(self.previous_entries_allocated));
    }
}

/// Interpreter-level caches for function data (single-threaded)
pub struct InterpreterFunctionCaches {
    function_instruction_caches: HashMap<FunctionPtr, Rc<RefCell<FrameTypeCache>>>,
    generic_function_instruction_caches: HashMap<GenericFunctionPtr, Rc<RefCell<FrameTypeCache>>>,
    budget: InstructionCacheBudget,
}

impl InterpreterFunctionCaches {
    pub fn new() -> Self {
        Self {
            function_instruction_caches: HashMap::new(),
            generic_function_instruction_caches: HashMap::new(),
            budget: InstructionCacheBudget::new(),
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

        let Self {
            function_instruction_caches,
            budget,
            ..
        } = self;

        let ptr = FunctionPtr::from_loaded_function(function);
        function_instruction_caches
            .entry(ptr)
            .or_insert_with(|| FrameTypeCache::make_rc_for_code_size(function.code_size(), budget))
            .clone()
    }

    /// Returns existing cache, or creates a new one for a generic function.
    pub(crate) fn get_or_create_frame_cache_generic(
        &mut self,
        function: &LoadedFunction,
    ) -> Rc<RefCell<FrameTypeCache>> {
        debug_assert!(!function.ty_args().is_empty());

        let Self {
            generic_function_instruction_caches,
            budget,
            ..
        } = self;

        let ptr = GenericFunctionPtr::from_loaded_function(function);
        generic_function_instruction_caches
            .entry(ptr)
            .or_insert_with(|| FrameTypeCache::make_rc_for_code_size(function.code_size(), budget))
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_reserves_until_exhausted() {
        let mut budget = InstructionCacheBudget { remaining: 100 };

        assert!(budget.try_reserve(60));
        assert_eq!(budget.remaining, 40);

        // Does not fit, and must leave the budget untouched so smaller later requests still fit.
        assert!(!budget.try_reserve(41));
        assert_eq!(budget.remaining, 40);

        assert!(budget.try_reserve(40));
        assert_eq!(budget.remaining, 0);
        assert!(!budget.try_reserve(1));
    }

    #[test]
    fn budget_reserve_does_not_underflow() {
        let mut budget = InstructionCacheBudget { remaining: 0 };
        assert!(!budget.try_reserve(usize::MAX));
        assert_eq!(budget.remaining, 0);
    }

    #[test]
    fn budget_override_is_scoped_and_restored() {
        let default = InstructionCacheBudget::new().remaining;
        assert_eq!(default, MAX_PER_INSTRUCTION_CACHE_ENTRIES);

        {
            let _override = InstructionCacheBudgetOverride::new(7);
            assert_eq!(InstructionCacheBudget::new().remaining, 7);
        }

        assert_eq!(
            InstructionCacheBudget::new().remaining,
            MAX_PER_INSTRUCTION_CACHE_ENTRIES
        );
    }
}
