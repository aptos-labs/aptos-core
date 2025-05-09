// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{
    errors::{PartialVMError, PartialVMResult},
    file_format::CompiledScript,
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, language_storage::ModuleId,
    vm_status::StatusCode,
};
use std::{collections::BTreeMap, sync::Arc};
use typed_arena::Arena;

pub struct TraversalStorage {
    referenced_scripts: Arena<Arc<CompiledScript>>,
    referenced_modules: Arena<Arc<CompiledModule>>,
    referenced_module_ids: Arena<ModuleId>,
    referenced_module_bundles: Arena<Vec<CompiledModule>>,
}

pub struct TraversalContext<'a> {
    visited: BTreeMap<(&'a AccountAddress, &'a IdentStr), ()>,

    pub referenced_scripts: &'a Arena<Arc<CompiledScript>>,
    pub referenced_modules: &'a Arena<Arc<CompiledModule>>,
    pub referenced_module_ids: &'a Arena<ModuleId>,
    pub referenced_module_bundles: &'a Arena<Vec<CompiledModule>>,
}

impl TraversalStorage {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            referenced_scripts: Arena::new(),
            referenced_modules: Arena::new(),
            referenced_module_ids: Arena::new(),
            referenced_module_bundles: Arena::new(),
        }
    }
}

impl<'a> TraversalContext<'a> {
    pub fn new(storage: &'a TraversalStorage) -> Self {
        Self {
            visited: BTreeMap::new(),

            referenced_scripts: &storage.referenced_scripts,
            referenced_modules: &storage.referenced_modules,
            referenced_module_ids: &storage.referenced_module_ids,
            referenced_module_bundles: &storage.referenced_module_bundles,
        }
    }

    /// If the specified address is not special, adds the address-name pair to the visited set.
    /// If the address is special, or if the set already contains the pair, returns false. Returns
    /// true otherwise.
    pub fn visit_if_not_special_address(
        &mut self,
        addr: &'a AccountAddress,
        name: &'a IdentStr,
    ) -> bool {
        !addr.is_special() && self.visited.insert((addr, name), ()).is_none()
    }

    /// If the address of the specified module id is not special, adds the address-name pair to the
    /// visited set and returns true. If the address is special, or if the set already contains the
    /// pair, returns false.
    pub fn visit_if_not_special_module_id(&mut self, module_id: &ModuleId) -> bool {
        let addr = module_id.address();
        if addr.is_special() {
            return false;
        }

        let name = module_id.name();
        if self.visited.contains_key(&(addr, name)) {
            false
        } else {
            let module_id = self.referenced_module_ids.alloc(module_id.clone());
            self.visited
                .insert((module_id.address(), module_id.name()), ());
            true
        }
    }

    /// No-op if address is visited, otherwise returns an invariant violation error.
    fn check_visited_impl(&self, addr: &AccountAddress, name: &IdentStr) -> PartialVMResult<()> {
        if self.visited.contains_key(&(addr, name)) {
            return Ok(());
        }

        let msg = format!("Module {}::{} has not been visited", addr, name);
        Err(PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(msg))
    }

    /// Returns an error if the address is not special and is not in a visited set.
    pub fn check_is_special_or_visited(
        &self,
        addr: &AccountAddress,
        name: &IdentStr,
    ) -> PartialVMResult<()> {
        if addr.is_special() {
            return Ok(());
        }

        self.check_visited_impl(addr, name)
    }

    /// No-op if address is visited, otherwise returns an invariant violation error.
    ///
    /// Note: this is used ONLY by few existing native functions and exists purely for backwards-
    /// compatibility reasons.
    pub fn legacy_check_visited(
        &self,
        addr: &AccountAddress,
        name: &IdentStr,
    ) -> PartialVMResult<()> {
        self.check_visited_impl(addr, name)
    }

    /// If address-name pairs are not special and have not been visited, visits them and pushes
    /// them to the provided stack.
    pub(crate) fn push_next_ids_to_visit<I>(
        &mut self,
        stack: &mut Vec<(&'a AccountAddress, &'a IdentStr)>,
        ids: I,
    ) where
        I: IntoIterator<Item = (&'a AccountAddress, &'a IdentStr)>,
        I::IntoIter: DoubleEndedIterator,
    {
        for (addr, name) in ids.into_iter().rev() {
            if self.visit_if_not_special_address(addr, name) {
                stack.push((addr, name));
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use move_core_types::ident_str;

    #[test]
    fn test_traversal_context() {
        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);

        let special = AccountAddress::ONE;
        let non_special_1 = AccountAddress::from_hex_literal("0x123").unwrap();
        let non_special_2 = AccountAddress::from_hex_literal("0x234").unwrap();
        assert!(special.is_special() && !non_special_1.is_special() && !non_special_2.is_special());

        let allocated_module_id = |addr| {
            let module_id = ModuleId::new(addr, ident_str!("foo").to_owned());
            traversal_context.referenced_module_ids.alloc(module_id)
        };

        let special = allocated_module_id(special);
        traversal_context
            .check_is_special_or_visited(special.address(), special.name())
            .expect("0x1 is special address and should not be visited");
        traversal_context
            .legacy_check_visited(special.address(), special.name())
            .expect_err("0x1 is special address and should not be visited");

        assert!(!traversal_context.visit_if_not_special_address(special.address(), special.name()));
        assert!(!traversal_context.visit_if_not_special_module_id(special));
        assert!(traversal_context.visited.is_empty());
        traversal_context
            .legacy_check_visited(special.address(), special.name())
            .expect_err("0x1 is special address but we don't allow them to be non-visited");

        let non_special_1 = allocated_module_id(non_special_1);
        let non_special_2 = ModuleId::new(non_special_2, ident_str!("foo").to_owned());
        traversal_context
            .check_is_special_or_visited(non_special_1.address(), non_special_1.name())
            .expect_err("0x123 is non-special address and have not been visited");
        traversal_context
            .check_is_special_or_visited(non_special_2.address(), non_special_2.name())
            .expect_err("0x234 is non-special address and have not been visited");

        assert!(traversal_context
            .visit_if_not_special_address(non_special_1.address(), non_special_1.name()));
        assert!(traversal_context.visit_if_not_special_module_id(&non_special_2));
        assert_eq!(traversal_context.visited.len(), 2);
        traversal_context
            .check_is_special_or_visited(non_special_1.address(), non_special_1.name())
            .expect("0x123 is non-special address but have been visited");
        traversal_context
            .check_is_special_or_visited(non_special_2.address(), non_special_2.name())
            .expect("0x234 is non-special address but have been visited");

        // Double insertion: should not be visiting anymore.
        assert!(!traversal_context
            .visit_if_not_special_address(non_special_1.address(), non_special_1.name()));
        assert!(!traversal_context.visit_if_not_special_module_id(&non_special_2));
    }
}
