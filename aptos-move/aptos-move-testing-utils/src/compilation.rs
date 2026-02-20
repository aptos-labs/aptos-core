// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Compilation caching utilities for Move packages.
//!
//! This module provides structures for caching compiled Move packages to avoid
//! redundant compilation during testing and comparison.

use move_core_types::{account_address::AccountAddress, language_storage::ModuleId};
use move_package::compilation::compiled_package::CompiledPackage;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fmt,
};

/// Information about a Move package for identification and caching purposes.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct PackageInfo {
    /// The address where the package is deployed
    pub address: AccountAddress,
    /// The name of the package
    pub package_name: String,
    /// Optional upgrade number for package versioning
    pub upgrade_number: Option<u64>,
}

impl fmt::Display for PackageInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut name = format!("{}.{}", self.package_name, self.address);
        if let Some(upgrade_number) = self.upgrade_number {
            name = format!("{}.{}", name, upgrade_number);
        }
        write!(f, "{}", name)
    }
}

impl PackageInfo {
    /// Creates a new PackageInfo instance.
    pub fn new(address: AccountAddress, package_name: String, upgrade_number: Option<u64>) -> Self {
        Self {
            address,
            package_name,
            upgrade_number,
        }
    }

    /// Checks if this package can be compiled (non-zero address).
    pub fn is_compilable(&self) -> bool {
        self.address != AccountAddress::ZERO
    }

    /// Creates a non-compilable PackageInfo placeholder.
    pub fn non_compilable_info() -> Self {
        Self {
            address: AccountAddress::ZERO,
            package_name: String::new(),
            upgrade_number: None,
        }
    }
}

/// Cache for compiled Move packages to avoid redundant compilation.
///
/// This structure maintains caches for:
/// - Compiled packages (full CompiledPackage objects)
/// - Failed compilation attempts (to avoid retrying)
/// - Compiled bytecode blobs (for different compiler versions)
#[derive(Default)]
pub struct CompilationCache {
    /// Map of successfully compiled packages
    compiled_package_map: HashMap<PackageInfo, CompiledPackage>,
    /// Set of packages that failed to compile (base compiler)
    failed_packages_base: HashSet<PackageInfo>,
    /// Set of packages that failed to compile (compared compiler)
    failed_packages_compared: HashSet<PackageInfo>,
    /// Bytecode cache for base compiler
    base_compiled_package_cache: HashMap<PackageInfo, HashMap<ModuleId, Vec<u8>>>,
    /// Bytecode cache for compared compiler
    compared_compiled_package_cache: HashMap<PackageInfo, HashMap<ModuleId, Vec<u8>>>,
}

impl CompilationCache {
    /// Creates a new empty compilation cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets a cached compiled package if it exists.
    pub fn get_compiled_package(&self, info: &PackageInfo) -> Option<&CompiledPackage> {
        self.compiled_package_map.get(info)
    }

    /// Inserts a compiled package into the cache.
    pub fn insert_compiled_package(&mut self, info: PackageInfo, package: CompiledPackage) {
        self.compiled_package_map.insert(info, package);
    }

    /// Checks if a package failed to compile with the base compiler.
    pub fn is_failed_base(&self, info: &PackageInfo) -> bool {
        self.failed_packages_base.contains(info)
    }

    /// Checks if a package failed to compile with the compared compiler.
    pub fn is_failed_compared(&self, info: &PackageInfo) -> bool {
        self.failed_packages_compared.contains(info)
    }

    /// Marks a package as failed for the base compiler.
    pub fn mark_failed_base(&mut self, info: PackageInfo) {
        self.failed_packages_base.insert(info);
    }

    /// Marks a package as failed for the compared compiler.
    pub fn mark_failed_compared(&mut self, info: PackageInfo) {
        self.failed_packages_compared.insert(info);
    }

    /// Gets the base compiler bytecode cache.
    pub fn get_base_cache(&self) -> &HashMap<PackageInfo, HashMap<ModuleId, Vec<u8>>> {
        &self.base_compiled_package_cache
    }

    /// Gets the compared compiler bytecode cache.
    pub fn get_compared_cache(&self) -> &HashMap<PackageInfo, HashMap<ModuleId, Vec<u8>>> {
        &self.compared_compiled_package_cache
    }

    /// Gets a mutable reference to the base compiler bytecode cache.
    pub fn get_base_cache_mut(&mut self) -> &mut HashMap<PackageInfo, HashMap<ModuleId, Vec<u8>>> {
        &mut self.base_compiled_package_cache
    }

    /// Gets a mutable reference to the compared compiler bytecode cache.
    pub fn get_compared_cache_mut(
        &mut self,
    ) -> &mut HashMap<PackageInfo, HashMap<ModuleId, Vec<u8>>> {
        &mut self.compared_compiled_package_cache
    }

    /// Clears all caches.
    pub fn clear(&mut self) {
        self.compiled_package_map.clear();
        self.failed_packages_base.clear();
        self.failed_packages_compared.clear();
        self.base_compiled_package_cache.clear();
        self.compared_compiled_package_cache.clear();
    }

    /// Returns the number of cached compiled packages.
    pub fn len(&self) -> usize {
        self.compiled_package_map.len()
    }

    /// Checks if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.compiled_package_map.is_empty()
    }

    /// Returns statistics about the cache.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            compiled_packages: self.compiled_package_map.len(),
            failed_base: self.failed_packages_base.len(),
            failed_compared: self.failed_packages_compared.len(),
            base_bytecode_entries: self.base_compiled_package_cache.len(),
            compared_bytecode_entries: self.compared_compiled_package_cache.len(),
        }
    }
}

/// Statistics about the compilation cache.
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub compiled_packages: usize,
    pub failed_base: usize,
    pub failed_compared: usize,
    pub base_bytecode_entries: usize,
    pub compared_bytecode_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_package_info() -> PackageInfo {
        PackageInfo::new(
            AccountAddress::from_hex_literal("0x1").unwrap(),
            "test-package".to_string(),
            None,
        )
    }

    #[test]
    fn test_package_info_display() {
        let info = create_test_package_info();
        let display = format!("{}", info);
        assert!(display.contains("test-package"));
        assert!(display.contains("0x1"));
    }

    #[test]
    fn test_package_info_is_compilable() {
        let compilable = create_test_package_info();
        assert!(compilable.is_compilable());

        let non_compilable = PackageInfo::non_compilable_info();
        assert!(!non_compilable.is_compilable());
    }

    #[test]
    fn test_compilation_cache_new() {
        let cache = CompilationCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_compilation_cache_failed_tracking() {
        let mut cache = CompilationCache::new();
        let info = create_test_package_info();

        assert!(!cache.is_failed_base(&info));
        assert!(!cache.is_failed_compared(&info));

        cache.mark_failed_base(info.clone());
        assert!(cache.is_failed_base(&info));
        assert!(!cache.is_failed_compared(&info));

        cache.mark_failed_compared(info.clone());
        assert!(cache.is_failed_base(&info));
        assert!(cache.is_failed_compared(&info));
    }

    #[test]
    fn test_compilation_cache_clear() {
        let mut cache = CompilationCache::new();
        let info = create_test_package_info();

        cache.mark_failed_base(info.clone());
        cache.mark_failed_compared(info.clone());

        cache.clear();

        assert!(cache.is_empty());
        assert!(!cache.is_failed_base(&info));
        assert!(!cache.is_failed_compared(&info));
    }

    #[test]
    fn test_compilation_cache_stats() {
        let mut cache = CompilationCache::new();
        let info = create_test_package_info();

        cache.mark_failed_base(info.clone());
        cache.mark_failed_compared(info);

        let stats = cache.stats();
        assert_eq!(stats.compiled_packages, 0);
        assert_eq!(stats.failed_base, 1);
        assert_eq!(stats.failed_compared, 1);
    }
}
