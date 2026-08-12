/// Module initialization support. This is internally used by the compiler.
module aptos_framework::init {
    use std::error;
    use std::features;
    use std::hash;
    use std::option::{Self, Option};
    use aptos_std::from_bcs;
    use aptos_framework::create_signer;
    use aptos_framework::object::{Self, ObjectCore};
    use aptos_framework::ordered_map::{Self, OrderedMap};

    /// Module initialization can only be requested by module code, not by scripts.
    const EINVALID_INITIALIZE_CALLER: u64 = 0x1;

    /// A module on an object cannot self-initialize once the object changed owner, or was deleted,
    /// since that module's code was last published.
    const EOWNER_CHANGED_SINCE_DEPLOY: u64 = 0x2;

    /// Lazy module initialization is not enabled (see the `LAZY_MODULE_INITIALIZATION` feature).
    const ELAZY_MODULE_INITIALIZATION_NOT_ENABLED: u64 = 0x3;

    /// Opaque representation of module names.
    struct ModuleId has store, copy, drop {
        hash: u128
    }

    /// Per-module initialization metadata.
    ///
    /// `only_once` is the flag first passed at initialization: `none` before init, `some(false)`
    /// re-inits after each upgrade (cleared by `reset_initialized`), `some(true)` inits only once.
    /// `deploy_owner` is the object's transitive root owner recorded at this module's last publish
    /// and gates object self-init (see `assert_may_self_initialize`); `none` for account addresses.
    struct ModuleState has store, copy, drop {
        only_once: Option<bool>,
        deploy_owner: Option<address>
    }

    /// Per-address initialization metadata, keyed by module so each module is gated on the owner at
    /// its own last publish -- republishing one module does not re-arm a sibling.
    enum InitializationState has key {
        V1 {
            modules: OrderedMap<ModuleId, ModuleState>
        }
    }

    /// If the calling module needs initialization -- never initialized, or (with `only_once` false)
    /// upgraded since -- marks it initialized and returns a signer for its address; else returns
    /// `none`. Marking happens before the initializer runs, so a cyclic call terminates.
    ///
    /// The caller module is derived by the VM from the call stack, so a module can only initialize
    /// itself. Aborts if the feature is disabled (`ELAZY_MODULE_INITIALIZATION_NOT_ENABLED`), the
    /// caller is not module code (`EINVALID_INITIALIZE_CALLER`), or an object-hosted module changed
    /// owner since publish (`EOWNER_CHANGED_SINCE_DEPLOY`).
    public fun internal_maybe_initialize(only_once: bool): Option<signer> {
        assert!(
            features::is_lazy_module_initialization_enabled(),
            error::invalid_state(ELAZY_MODULE_INITIALIZATION_NOT_ENABLED),
        );
        let (addr, module_id) = get_caller_address_and_module_id();
        if (check_and_set_initialized(addr, module_id, only_once)) {
            option::none()
        } else {
            // Guard only when actually minting: a legitimate transfer after initialization must not
            // brick ordinary calls. An abort here rolls back the mark set above.
            assert_may_self_initialize(addr, module_id);
            option::some(create_signer::create_signer(addr))
        }
    }

    /// Aborts unless the module at `addr` may self-initialize now. Only object-hosted modules are
    /// gated: an object must still have the transitive root owner recorded for this module at
    /// publish, so a transfer of the object or an ancestor, or its deletion, blocks self-init; an
    /// object with no record is fail-closed. Account addresses authorize their own code by publishing.
    fun assert_may_self_initialize(addr: address, module_id: ModuleId) {
        let recorded = recorded_deploy_owner(addr, module_id);
        let ok = if (recorded.is_some()) {
            object::is_object(addr)
                && recorded.destroy_some() == object::address_to_object<ObjectCore>(addr).root_owner()
        } else {
            !object::is_object(addr)
        };
        assert!(ok, error::permission_denied(EOWNER_CHANGED_SINCE_DEPLOY));
    }

    /// The object root owner recorded for `module_id` at its last publish, or `none` if the module
    /// has no such record (an account module, or an object module never recorded).
    fun recorded_deploy_owner(addr: address, module_id: ModuleId): Option<address> {
        if (!exists<InitializationState>(addr)) return option::none();
        let modules = &InitializationState[addr].modules;
        if (modules.contains(&module_id)) modules.borrow(&module_id).deploy_owner else option::none()
    }

    /// Records `owner` as the object root owner of the module named `module_name` at (re)publish, to
    /// gate its later self-init (see `assert_may_self_initialize`). Called per module by
    /// `code::publish_package` for object addresses only.
    package fun record_deploy_owner(addr: address, module_name: vector<u8>, owner: address) {
        let module_id = module_id_from_name(module_name);
        ensure_module_state(addr, module_id);
        InitializationState[addr].modules.borrow_mut(&module_id).deploy_owner = option::some(owner);
    }

    /// Called on code upgrade to request re-run of initialization for the named module. Skipped for
    /// modules that used `only_once = true` when first initialized. Keeps the recorded deploy owner.
    package fun reset_initialized(addr: address, module_name: vector<u8>) {
        if (exists<InitializationState>(addr)) {
            let modules = &mut InitializationState[addr].modules;
            let module_id = module_id_from_name(module_name);
            if (modules.contains(&module_id)) {
                let state = modules.borrow_mut(&module_id);
                if (state.only_once == option::some(false)) {
                    state.only_once = option::none();
                }
            }
        }
    }

    /// Creates the id for a module name (see `get_caller_address_and_module_id` for the format).
    fun module_id_from_name(name: vector<u8>): ModuleId {
        let hash = hash::sha3_256(name);
        hash.trim(16);
        ModuleId { hash: from_bcs::to_u128(hash) }
    }

    /// Returns true if the module is already initialized. Otherwise marks it initialized, recording
    /// `only_once`: if true the entry survives upgrades (initializer never re-runs); if false an
    /// upgrade resets it (initializer re-runs).
    fun check_and_set_initialized(addr: address, module_id: ModuleId, only_once: bool): bool {
        ensure_module_state(addr, module_id);
        let state = InitializationState[addr].modules.borrow_mut(&module_id);
        if (state.only_once.is_some()) {
            true
        } else {
            state.only_once = option::some(only_once);
            false
        }
    }

    /// Ensures an `InitializationState` exists at `addr` holding an entry for `module_id`.
    fun ensure_module_state(addr: address, module_id: ModuleId) {
        if (!exists<InitializationState>(addr)) {
            move_to(
                &create_signer::create_signer(addr),
                InitializationState::V1 { modules: ordered_map::new() },
            );
        };
        let modules = &mut InitializationState[addr].modules;
        if (!modules.contains(&module_id)) {
            modules.add(module_id, ModuleState { only_once: option::none(), deploy_owner: option::none() });
        };
    }

    /// Returns the address and module id of the module that directly called the function invoking
    /// this native. Aborts with `EINVALID_INITIALIZE_CALLER` if there is no such module (e.g. a
    /// script). The module id is the sha3-256 of the module name, trimmed to 16 bytes and read as
    /// the bcs encoding of a u128.
    native fun get_caller_address_and_module_id(): (address, ModuleId);

    // ------------------------------------------------------------------------------------------
    // Tests for `assert_may_self_initialize`. Construct objects/accounts directly and record the
    // deploy owner as `code::publish_package` would.

    #[test_only]
    const EOWNER_CHANGED: u64 = 0x5_0002; // error::permission_denied(EOWNER_CHANGED_SINCE_DEPLOY)

    #[test_only]
    fun record_current_owner(addr: address, module_name: vector<u8>) {
        record_deploy_owner(addr, module_name, object::address_to_object<ObjectCore>(addr).root_owner());
    }

    #[test_only]
    fun assert_may_init(addr: address, module_name: vector<u8>) {
        assert_may_self_initialize(addr, module_id_from_name(module_name))
    }

    #[test]
    fun self_init_allowed_when_root_owner_unchanged() {
        let addr = object::address_from_constructor_ref(&object::create_object(@0xcafe));
        record_current_owner(addr, b"m");
        assert_may_init(addr, b"m");
    }

    #[test]
    fun self_init_allowed_when_nested_root_owner_unchanged() {
        let parent = object::address_from_constructor_ref(&object::create_object(@0xcafe));
        let child = object::address_from_constructor_ref(&object::create_object(parent));
        record_current_owner(child, b"m");
        assert_may_init(child, b"m");
    }

    #[test]
    fun self_init_allowed_for_account_address() {
        // A non-object address has no recorded owner and is allowed (it authorized its own code).
        assert_may_init(@0xcafe, b"m");
    }

    #[test]
    #[expected_failure(abort_code = EOWNER_CHANGED, location = Self)]
    fun self_init_blocked_when_owner_transferred() {
        let cref = object::create_object(@0xcafe);
        let addr = object::address_from_constructor_ref(&cref);
        record_current_owner(addr, b"m");
        object::transfer(
            &create_signer::create_signer(@0xcafe),
            object::object_from_constructor_ref<ObjectCore>(&cref),
            @0xbeef,
        );
        assert_may_init(addr, b"m");
    }

    #[test]
    #[expected_failure(abort_code = EOWNER_CHANGED, location = Self)]
    fun self_init_blocked_when_ancestor_transferred() {
        // Transferring the parent leaves the child's direct owner fixed but moves its root owner.
        let parent = object::create_object(@0xcafe);
        let parent_addr = object::address_from_constructor_ref(&parent);
        let child = object::address_from_constructor_ref(&object::create_object(parent_addr));
        record_current_owner(child, b"m");
        object::transfer(
            &create_signer::create_signer(@0xcafe),
            object::object_from_constructor_ref<ObjectCore>(&parent),
            @0xbeef,
        );
        assert_may_init(child, b"m");
    }

    #[test]
    #[expected_failure(abort_code = EOWNER_CHANGED, location = Self)]
    fun self_init_blocked_when_object_deleted() {
        let cref = object::create_object(@0xcafe);
        let addr = object::address_from_constructor_ref(&cref);
        record_current_owner(addr, b"m");
        object::delete(object::generate_delete_ref(&cref));
        assert_may_init(addr, b"m");
    }

    #[test]
    #[expected_failure(abort_code = EOWNER_CHANGED, location = Self)]
    fun self_init_blocked_when_object_has_no_recorded_owner() {
        // Fail-closed: an object whose owner was not recorded at publish cannot self-init.
        let addr = object::address_from_constructor_ref(&object::create_object(@0xcafe));
        assert_may_init(addr, b"m");
    }

    #[test]
    fun republished_module_allowed_under_new_owner() {
        // Two modules published under @0xcafe; object transferred; only `m2` republished under the
        // new owner. `m2`'s record now matches the current owner, so it may self-init.
        let cref = object::create_object(@0xcafe);
        let addr = object::address_from_constructor_ref(&cref);
        record_current_owner(addr, b"m1");
        record_current_owner(addr, b"m2");
        object::transfer(
            &create_signer::create_signer(@0xcafe),
            object::object_from_constructor_ref<ObjectCore>(&cref),
            @0xbeef,
        );
        record_current_owner(addr, b"m2");
        assert_may_init(addr, b"m2");
    }

    #[test]
    #[expected_failure(abort_code = EOWNER_CHANGED, location = Self)]
    fun republished_sibling_does_not_rearm_transferred_module() {
        // Same setup: republishing `m2` under the new owner must not re-arm `m1`, whose record
        // still holds the original owner -- so `m1` remains blocked after the transfer.
        let cref = object::create_object(@0xcafe);
        let addr = object::address_from_constructor_ref(&cref);
        record_current_owner(addr, b"m1");
        record_current_owner(addr, b"m2");
        object::transfer(
            &create_signer::create_signer(@0xcafe),
            object::object_from_constructor_ref<ObjectCore>(&cref),
            @0xbeef,
        );
        record_current_owner(addr, b"m2");
        assert_may_init(addr, b"m1");
    }
}
