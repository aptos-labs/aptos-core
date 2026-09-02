spec aptos_framework::init {
    // native function
    spec get_caller_address_and_module_id(): (address, ModuleId) {
        // The result is derived by the VM from the calling stack frame, which cannot be
        // modeled in the specification language; treated as uninterpreted.
        pragma opaque;
    }

    spec internal_maybe_initialize(only_once: bool): Option<signer> {
        use 0x1::option;
        use 0x1::features;
        use 0x1::create_signer;
        // The caller address is the opaque, VM-derived result of
        // `get_caller_address_and_module_id` (see above), so the object-ownership abort
        // condition cannot be modeled here; abort conditions are treated as partial.
        pragma aborts_if_is_partial;
        pragma opaque = true;
        ensures [inferred]({
            let a = ..S1 |~ result_of<features::is_lazy_module_initialization_enabled>();
            let b = {
                let (c_0, c_1) = S1..S2 |~ result_of<get_caller_address_and_module_id>();
                S2..S3 |~ result_of<check_and_set_initialized>(c_0, c_1, only_once)
            };
            a && b ==> result == option::none<signer>()
        });
        ensures [inferred]({
            let a = ..S1 |~ result_of<features::is_lazy_module_initialization_enabled>();
            let (b_0, b_1) = S1..S2 |~ result_of<get_caller_address_and_module_id>();
            let c = S2..S3 |~ result_of<check_and_set_initialized>(b_0, b_1, only_once);
            (S2..S3 |~ a && !result_of<check_and_set_initialized>(b_0, b_1, only_once)) ==>
                result == option::some<signer>(create_signer::spec_create_signer(b_0))
        });
        ensures [inferred](
            ..S1 |~ result_of<features::is_lazy_module_initialization_enabled>()
        ) && (
            S2..S3 |~ !{
                let (a_0, a_1) = S1..S2 |~ result_of<get_caller_address_and_module_id>();
                result_of<check_and_set_initialized>(a_0, a_1, only_once)
            }
        ) ==>
            {
                let (b_0, b_1) = S1..S2 |~ result_of<get_caller_address_and_module_id>();
                S3.. |~ ensures_of<assert_may_self_initialize>(b_0, b_1)
            };
        aborts_if [inferred]..S1 |~(
            !result_of<features::is_lazy_module_initialization_enabled>()
        );
        aborts_if [inferred](
            ..S1 |~ result_of<features::is_lazy_module_initialization_enabled>()
        )
            && (
                (
                    S2..S3 |~ !{
                        let (a_0, a_1) = S1..S2 |~ result_of<get_caller_address_and_module_id>();
                        result_of<check_and_set_initialized>(a_0, a_1, only_once)
                    }
                )
                    && {
                        let (b_0, b_1) =
                            S1..S2 |~ result_of<get_caller_address_and_module_id>();
                        S3 |~ aborts_of<assert_may_self_initialize>(b_0, b_1)
                    }
            );
        aborts_if [inferred](
            ..S1 |~ result_of<features::is_lazy_module_initialization_enabled>()
        ) && (S1 |~ aborts_of<get_caller_address_and_module_id>());
        aborts_if [inferred] aborts_of<features::is_lazy_module_initialization_enabled>();
    }

    spec assert_may_self_initialize(
        addr: address, module_id: 0x1::init::ModuleId
    ) {
        use 0x1::option;
        use 0x1::error;
        use 0x1::object;
        pragma opaque = true;
        aborts_if [inferred]!option::is_some<address>(
            ..S1 |~ result_of<recorded_deploy_owner>(addr, module_id)
        ) && (S1.. |~ result_of<object::is_object>(addr));
        aborts_if [inferred]({
            let a = S1.. |~ result_of<object::is_object>(addr);
            let b = ..S1 |~ result_of<recorded_deploy_owner>(addr, module_id);
            !option::is_some<address>(b)
                && (a
                    && aborts_of<error::permission_denied>(2))
        });
        aborts_if [inferred]({
            let a = S1..S2 |~ result_of<object::is_object>(addr);
            let b = ..S1 |~ result_of<recorded_deploy_owner>(addr, module_id);
            let c =
                S2.. |~ result_of<object::root_owner<object::ObjectCore>> (
                    object::address_to_object<object::ObjectCore>(addr)
                );
            option::is_some<address>(b) && (a && option::destroy_some<address>(b) != c)
        });
        aborts_if [inferred]({
            let a = S1..S2 |~ result_of<object::is_object>(addr);
            let b = ..S1 |~ result_of<recorded_deploy_owner>(addr, module_id);
            let c =
                S2.. |~ result_of<object::root_owner<object::ObjectCore>> (
                    object::address_to_object<object::ObjectCore>(addr)
                );
            option::is_some<address>(b)
                && (
                    a
                        && (
                            option::destroy_some<address>(b) != c
                                && aborts_of<error::permission_denied>(2)
                        )
                )
        });
        aborts_if [inferred]({
            let a = S1..S2 |~ result_of<object::is_object>(addr);
            let b = ..S1 |~ result_of<recorded_deploy_owner>(addr, module_id);
            option::is_some<address>(b)
                && (a
                    && aborts_of<object::address_to_object<object::ObjectCore>> (addr))
        });
        aborts_if [inferred]({
            let a = S1..S2 |~ result_of<object::is_object>(addr);
            let b = ..S1 |~ result_of<recorded_deploy_owner>(addr, module_id);
            option::is_some<address>(b)
                && (a && aborts_of<option::destroy_some<address>> (b))
        });
        aborts_if [inferred]({
            let a = S1..S2 |~ result_of<object::is_object>(addr);
            let b = ..S1 |~ result_of<recorded_deploy_owner>(addr, module_id);
            option::is_some<address>(b) && !a
        });
        aborts_if [inferred]({
            let a = S1..S2 |~ result_of<object::is_object>(addr);
            let b = ..S1 |~ result_of<recorded_deploy_owner>(addr, module_id);
            option::is_some<address>(b)
                && (!a
                    && aborts_of<error::permission_denied>(2))
        });
    }

    spec check_and_set_initialized(
        addr: address, module_id: 0x1::init::ModuleId, only_once: bool
    ): bool {
        use 0x1::option;
        use 0x1::ordered_map;
        pragma opaque = true, aborts_if_is_partial = true;
        modifies InitializationState[addr];
        ensures [inferred = sathard]({
            let a = {
                let b = S1 |~ global<InitializationState>(addr);
                S1.. |~ result_of<ordered_map::borrow_mut<ModuleId, ModuleState>> (
                    b.modules, module_id
                )
            };
            !option::is_some<bool>(a.only_once) ==> result == false
        });
        ensures [inferred = sathard]({
            let a = S1 |~ global<InitializationState>(addr);
            let b =
                S1.. |~ result_of<ordered_map::borrow_mut<ModuleId, ModuleState>> (
                    a.modules, module_id
                );
            !option::is_some<bool>(
                (
                    S1.. |~ result_of<ordered_map::borrow_mut<ModuleId,
                    ModuleState>> (a.modules, module_id)
                ).only_once
            ) ==>
                InitializationState[addr].modules == a.modules
        });
        ensures [inferred = sathard]({
            let a = {
                let b = S1 |~ global<InitializationState>(addr);
                S1.. |~ result_of<ordered_map::borrow_mut<ModuleId, ModuleState>> (
                    b.modules, module_id
                )
            };
            option::is_some<bool>(a.only_once) ==> result == true
        });
        ensures [inferred]..S1 |~(ensures_of<ensure_module_state>(addr, module_id));
        aborts_if [inferred] S1 |~(!exists<InitializationState>(addr));
    }

    spec ensure_module_state(
        addr: address, module_id: 0x1::init::ModuleId
    ) {
        use 0x1::option;
        use 0x1::ordered_map;
        pragma opaque = true, aborts_if_is_partial = false;
        modifies InitializationState[addr];
        let empty_module = ModuleState {
            only_once: option::none<bool>(),
            deploy_owner: option::none<address>()
        };
        ensures exists<InitializationState>(addr);
        ensures ordered_map::spec_contains_key<ModuleId, ModuleState>(
            InitializationState[addr].modules, module_id
        );
        ensures ordered_map::spec_get<ModuleId, ModuleState>(
            InitializationState[addr].modules, module_id
        ) == if (old(exists<InitializationState>(addr))
            && ordered_map::spec_contains_key<ModuleId, ModuleState>(
                old(InitializationState[addr]).modules, module_id
            )) {
            ordered_map::spec_get<ModuleId, ModuleState>(
                old(InitializationState[addr]).modules, module_id
            )
        } else {
            empty_module
        };
        ensures old(exists<InitializationState>(addr))
            && ordered_map::spec_contains_key<ModuleId, ModuleState>(
                old(InitializationState[addr]).modules, module_id
            ) ==>
            InitializationState[addr] == old(InitializationState[addr]);
        ensures old(exists<InitializationState>(addr))
            && !ordered_map::spec_contains_key<ModuleId, ModuleState>(
                old(InitializationState[addr]).modules, module_id
            ) ==>
            InitializationState[addr].modules
                == ordered_map::spec_set<ModuleId, ModuleState>(
                    old(InitializationState[addr]).modules, module_id, empty_module
                );
        ensures !old(exists<InitializationState>(addr)) ==>
            ordered_map::spec_len<ModuleId, ModuleState>(
                InitializationState[addr].modules
            ) == 1;
        aborts_if false;
    }

    spec module_id_from_name(name: vector<u8>): 0x1::init::ModuleId {
        use 0x1::from_bcs;
        use 0x1::hash;
        pragma opaque = true, aborts_if_is_partial = false;
        let digest = hash::sha3_256(name);
        let prefix = digest[0..16];
        ensures result == ModuleId { hash: result_of<from_bcs::to_u128>(prefix) };
        aborts_if aborts_of<hash::sha3_256>(name);
        aborts_if !aborts_of<hash::sha3_256>(name)
            && aborts_of<from_bcs::to_u128>(prefix);
    }

    spec record_deploy_owner(
        addr: address, module_name: vector<u8>, owner: address
    ) {
        use 0x1::option;
        use 0x1::ordered_map;
        pragma opaque = true, aborts_if_is_partial = false;
        modifies InitializationState[addr];
        aborts_if aborts_of<module_id_from_name>(module_name);
        ensures exists<InitializationState>(addr);
        ensures exists module_id: ModuleId:
            ensures_of<module_id_from_name>(module_name, module_id)
                && ordered_map::spec_contains_key<ModuleId, ModuleState>(
                    InitializationState[addr].modules, module_id
                )
                && ordered_map::spec_get<ModuleId, ModuleState>(
                    InitializationState[addr].modules, module_id
                ).deploy_owner == option::some(owner)
                && ordered_map::spec_get<ModuleId, ModuleState>(
                    InitializationState[addr].modules, module_id
                ).only_once
                    == if (old(exists<InitializationState>(addr))
                        && ordered_map::spec_contains_key<ModuleId, ModuleState>(
                            old(InitializationState[addr]).modules, module_id
                        )) {
                        ordered_map::spec_get<ModuleId, ModuleState>(
                            old(InitializationState[addr]).modules, module_id
                        ).only_once
                    } else {
                        option::none<bool>()
                    };
    }

    spec recorded_deploy_owner(
        addr: address, module_id: 0x1::init::ModuleId
    ): 0x1::option::Option<address> {
        use 0x1::option;
        use 0x1::ordered_map;
        pragma opaque = true;
        ensures [inferred]!exists<InitializationState>(addr) ==>
            result == option::none<address>();
        ensures [inferred] exists<InitializationState>(addr)
            && ordered_map::spec_contains_key<ModuleId, ModuleState>(
                InitializationState[addr].modules, module_id
            ) ==>
            result
                == ordered_map::spec_get<ModuleId, ModuleState>(
                    InitializationState[addr].modules, module_id
                ).deploy_owner;
        ensures [inferred] exists<InitializationState>(addr)
            && !ordered_map::spec_contains_key<ModuleId, ModuleState>(
                InitializationState[addr].modules, module_id
            ) ==>
            result == option::none<address>();
        aborts_if [inferred] false;
    }

    spec reset_initialized(addr: address, module_name: vector<u8>) {
        use 0x1::option;
        use 0x1::ordered_map;
        pragma opaque = true;
        modifies InitializationState[addr];
        ensures [inferred]!old(exists<InitializationState>(addr)) ==>
            !exists<InitializationState>(addr);
        ensures [inferred]({
            let module_id = ..S1 |~ result_of<module_id_from_name>(module_name);
            let present =
                S1.. |~ result_of<ordered_map::contains>(
                    old(InitializationState[addr]).modules, module_id
                );
            old(exists<InitializationState>(addr)) && !present ==>
                InitializationState[addr] == old(InitializationState[addr])
        });
        ensures [inferred]({
            let module_id = ..S1 |~ result_of<module_id_from_name>(module_name);
            let state =
                S1.. |~ result_of<ordered_map::borrow_mut>(
                    old(InitializationState[addr]).modules, module_id
                );
            old(exists<InitializationState>(addr))
                && ordered_map::spec_contains_key(
                    old(InitializationState[addr]).modules, module_id
                )
                && state.only_once == option::some(false) ==>
                InitializationState[addr].modules
                    == ordered_map::spec_set(
                        old(InitializationState[addr]).modules,
                        module_id,
                        update_field(
                            ordered_map::spec_get(
                                old(InitializationState[addr]).modules, module_id
                            ),
                            only_once,
                            option::none<bool>()
                        )
                    )
        });
        ensures [inferred]({
            let module_id = ..S1 |~ result_of<module_id_from_name>(module_name);
            let state =
                S1.. |~ result_of<ordered_map::borrow_mut>(
                    old(InitializationState[addr]).modules, module_id
                );
            old(exists<InitializationState>(addr))
                && ordered_map::spec_contains_key(
                    old(InitializationState[addr]).modules, module_id
                )
                && state.only_once != option::some(false) ==>
                InitializationState[addr] == old(InitializationState[addr])
        });
        aborts_if [inferred] exists<InitializationState>(addr)
            && aborts_of<module_id_from_name>(module_name);
    }
}
