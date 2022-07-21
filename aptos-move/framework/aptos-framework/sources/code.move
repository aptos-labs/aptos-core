/// This module supports functionality related to code management.
module aptos_framework::code {
    use std::string::String;
    use std::error;
    use std::signer;
    use std::vector;

    // ----------------------------------------------------------------------
    // Code Publishing

     /// The package registry at the given address.
    struct PackageRegistry has key {
        /// Packages installed at this address.
        packages: vector<PackageMetadata>,
    }

    /// Metadata for a package.
    struct PackageMetadata has store, copy, drop {
        /// Name of this package.
        name: String,
        /// Whether this package is immutable. An immutable package cannot be
        /// upgraded.
        immutable: bool,
        /// Package version, a counter for the number of upgrades + 1.
        /// (Except special case 0-version which can be updated in place.)
        version: u64,
        /// The list of modules installed by this package.
        modules: vector<ModuleMetadata>,
        /// Address aliases which where used when the modules above were compiled.
        address_aliases: vector<AddressAlias>,
        /// Dependencies which were used when this package was compiled.
        deps: vector<PackageDep>,
    }

    /// Metadata about a module in a package.
    struct ModuleMetadata has store, copy, drop {
        /// Name of the module.
        name: String,
        /// Source map, in internal encoding
        source_map: vector<u8>,
        /// Source text.
        source: String,
    }

    /// A package dependency. This consists of an address and the name of a package
    /// at this address.
    struct PackageDep has store, copy, drop {
        addr: address,
        name: String,
    }

    /// An address alias.
    struct AddressAlias has store, copy, drop {
        alias: String,
        addr: address
    }

    /// A package is attempted to publish with module names clashing with modules published by other packages on this
    /// address.
    const EMODULE_NAME_CLASH: u64 = 0x1;

    /// A package is attempted to upgrade which is marked as immutable.
    const EUPGRADE_IMMUTABLE: u64 = 0x2;

    /// A package is attempted to upgrade with a bad version number.
    const EUPGRADE_BAD_VERSION: u64 = 0x3;

    /// Publishes a package at the given signer's address. The caller must provide package metadata describing the
    /// package.
    ///
    /// TODO(wrwg): document detailed semantics by savaging the design doc
    public fun publish_package(owner: &signer, pack: PackageMetadata, code: vector<vector<u8>>) acquires PackageRegistry {
        let addr = signer::address_of(owner);
        if (!exists<PackageRegistry>(addr)) {
            move_to(owner, PackageRegistry{packages: vector::empty()})
        };

        // Check package
        let module_names = get_module_names(&pack);
        let packages = &mut borrow_global_mut<PackageRegistry>(addr).packages;
        let len = vector::length(packages);
        let index = len;
        let comp_check_needed = false;
        let i = 0;
        while (i < len) {
            let old = vector::borrow(packages, i);
            if (old.name == pack.name) {
                comp_check_needed = check_upgradability(old, &pack);
                index = i;
            } else {
                check_coexistence(old, &module_names)
            };
            i = i + 1;
        };

        // Update registry
        if (index < len) {
            *vector::borrow_mut(packages, index) = pack
        } else {
            vector::push_back(packages, pack)
        };

        // Request publish
        native_request_publish(addr, module_names, code, comp_check_needed)
    }

    /// Checks whether the given package is upgradable, and returns true if a compatibility check is needed.
    fun check_upgradability(old_pack: &PackageMetadata, new_pack: &PackageMetadata): bool {
        assert!(!old_pack.immutable, EUPGRADE_IMMUTABLE);
        assert!(
            old_pack.version == new_pack.version && old_pack.version == 0 ||
            old_pack.version + 1 == new_pack.version,
            EUPGRADE_BAD_VERSION
        );
        if (old_pack.version > 0) {
            // Compatibility check required
            true
        } else {
            false
        }
    }

    /// Checks whether a new package with given names can co-exist with old package.
    fun check_coexistence(old_pack: &PackageMetadata, new_modules: &vector<String>) {
        // The modules introduced by each package must not overlap with `names`.
        let i = 0;
        while (i < vector::length(&old_pack.modules)) {
            let old_mod = vector::borrow(&old_pack.modules, i);
            let j = 0;
            while (j < vector::length(new_modules)) {
                let name = vector::borrow(new_modules, j);
                assert!(&old_mod.name != name, error::already_exists(EMODULE_NAME_CLASH))
            }
        }
    }

    /// Get the names of the modules in a package.
    fun get_module_names(pack: &PackageMetadata): vector<String> {
        let module_names = vector::empty();
        let i = 0;
        while (i < vector::length(&pack.modules)) {
            vector::push_back(&mut module_names, vector::borrow(&pack.modules, i).name)
        };
        module_names
    }

    /// Native function to initiate module loading
    native fun native_request_publish(
        owner: address,
        expected_modules: vector<String>,
        bundle: vector<vector<u8>>,
        check_compatibility: bool
    );
}
