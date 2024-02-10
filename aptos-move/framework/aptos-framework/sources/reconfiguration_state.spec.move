spec aptos_framework::reconfiguration_state {

    spec module {
        use aptos_framework::chain_status;
        invariant [suspendable] chain_status::is_operating() ==> exists<State>(@aptos_framework);
    }

    spec initialize(fx: &signer) {
        use std::signer;
        aborts_if signer::address_of(fx) != @aptos_framework;
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

    spec try_mark_as_in_progress() {
        include TryMarkAsInProgressAbortsIf;
    }

    spec schema TryMarkAsInProgressAbortsIf {
        aborts_if !exists<State>(@aptos_framework);
        aborts_if copyable_any::type_name(global<State>(@aptos_framework).variant).bytes
            == b"0x1::reconfiguration_state::StateInactive" && !exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework);
    }

    spec start_time_secs(): u64 {
        include StartTimeSecsAbortsIf;
    }

    spec schema StartTimeSecsAbortsIf {
        aborts_if !exists<State>(@aptos_framework);
        include  copyable_any::type_name(global<State>(@aptos_framework).variant).bytes
            == b"0x1::reconfiguration_state::StateActive" ==>
        copyable_any::UnpackAbortsIf<StateActive> {
            x:  global<State>(@aptos_framework).variant
        };
        aborts_if copyable_any::type_name(global<State>(@aptos_framework).variant).bytes
            != b"0x1::reconfiguration_state::StateActive";
    }

}
