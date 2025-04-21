spec aptos_framework::reconfiguration_state {

    spec module {
        use aptos_framework::chain_status;
        invariant [suspendable] chain_status::is_operating() ==> exists<State>(@aptos_framework);
    }

    spec initialize(fx: &signer) {
        use std::signer;
        use aptos_std::from_bcs;
        aborts_if signer::address_of(fx) != @aptos_framework;
        let post post_state = global<State>(@aptos_framework);
        ensures exists<State>(@aptos_framework);
        ensures !exists<State>(@aptos_framework) ==> from_bcs::deserializable<StateInactive>(post_state.variant.data);
    }

    spec initialize_for_testing(fx: &signer) {
        use std::signer;
        aborts_if signer::address_of(fx) != @aptos_framework;
    }

    spec is_in_progress(): bool {
        aborts_if false;
    }

    spec fun spec_is_in_progress(): bool {
        if (!exists<State>(@aptos_framework)) {
            false
        } else {
            copyable_any::type_name(global<State>(@aptos_framework).variant).bytes == b"0x1::reconfiguration_state::StateActive"
        }
    }

    spec State {
        use aptos_std::from_bcs;
        use aptos_std::type_info;
        invariant copyable_any::type_name(variant).bytes == b"0x1::reconfiguration_state::StateActive" ||
            copyable_any::type_name(variant).bytes == b"0x1::reconfiguration_state::StateInactive";
        invariant copyable_any::type_name(variant).bytes == b"0x1::reconfiguration_state::StateActive"
            ==> from_bcs::deserializable<StateActive>(variant.data);
        invariant copyable_any::type_name(variant).bytes == b"0x1::reconfiguration_state::StateInactive"
            ==> from_bcs::deserializable<StateInactive>(variant.data);
        invariant copyable_any::type_name(variant).bytes == b"0x1::reconfiguration_state::StateActive" ==>
            type_info::type_name<StateActive>() == variant.type_name;
        invariant copyable_any::type_name(variant).bytes == b"0x1::reconfiguration_state::StateInactive" ==>
            type_info::type_name<StateInactive>() == variant.type_name;
    }

    spec on_reconfig_start {
        use aptos_std::from_bcs;
        use aptos_std::type_info;
        use std::bcs;
        aborts_if false;
        requires exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework);
        let state = Any {
            type_name: type_info::type_name<StateActive>(),
            data: bcs::serialize(StateActive {
                start_time_secs: timestamp::spec_now_seconds()
            })
        };
        let pre_state = global<State>(@aptos_framework);
        let post post_state = global<State>(@aptos_framework);
        ensures (exists<State>(@aptos_framework) && copyable_any::type_name(pre_state.variant).bytes
            == b"0x1::reconfiguration_state::StateInactive") ==> copyable_any::type_name(post_state.variant).bytes
            == b"0x1::reconfiguration_state::StateActive";
        ensures (exists<State>(@aptos_framework) && copyable_any::type_name(pre_state.variant).bytes
            == b"0x1::reconfiguration_state::StateInactive") ==> post_state.variant == state;
        ensures (exists<State>(@aptos_framework) && copyable_any::type_name(pre_state.variant).bytes
            == b"0x1::reconfiguration_state::StateInactive") ==> from_bcs::deserializable<StateActive>(post_state.variant.data);
    }

    spec start_time_secs(): u64 {
        include StartTimeSecsAbortsIf;
    }

    spec fun spec_start_time_secs(): u64 {
        use aptos_std::from_bcs;
        let state = global<State>(@aptos_framework);
        from_bcs::deserialize<StateActive>(state.variant.data).start_time_secs
    }

    spec schema StartTimeSecsRequirement {
        requires exists<State>(@aptos_framework);
        requires copyable_any::type_name(global<State>(@aptos_framework).variant).bytes
            == b"0x1::reconfiguration_state::StateActive";
        include UnpackRequiresStateActive {
            x:  global<State>(@aptos_framework).variant
        };
    }

    spec schema UnpackRequiresStateActive {
        use aptos_std::from_bcs;
        use aptos_std::type_info;
        x: Any;
        requires type_info::type_name<StateActive>() == x.type_name && from_bcs::deserializable<StateActive>(x.data);
    }

    spec schema StartTimeSecsAbortsIf {
        aborts_if !exists<State>(@aptos_framework);
        include  copyable_any::type_name(global<State>(@aptos_framework).variant).bytes
            == b"0x1::reconfiguration_state::StateActive" ==>
        copyable_any::UnpackAbortsIf<StateActive> {
            self: global<State>(@aptos_framework).variant
        };
        aborts_if copyable_any::type_name(global<State>(@aptos_framework).variant).bytes
            != b"0x1::reconfiguration_state::StateActive";
    }

}
