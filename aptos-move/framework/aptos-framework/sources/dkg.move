module aptos_framework::dkg {
    use std::error;
    use std::signer;
    use aptos_std::debug;
    use aptos_framework::account;
    use aptos_framework::event;
    use aptos_framework::system_addresses;

    friend aptos_framework::block;
    friend aptos_framework::genesis;
    friend aptos_framework::reconfiguration;

    /// An invalid block time was encountered.
    const EINVALID_GUID_FOR_EVENT: u64 = 5;

    struct StartDKGEvent has drop, store {
        validator_set_and_stake_dist: vector<u8>, //TODO...
    }

    struct DKGState has key {
        state_id: u64, // 0: not started, 1: started;
        events: event::EventHandle<StartDKGEvent>,
    }

    public(friend) fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(5 == account::get_guid_next_creation_num(signer::address_of(aptos_framework)), error::invalid_state(EINVALID_GUID_FOR_EVENT));
        move_to<DKGState>(
            aptos_framework,
            DKGState {
                state_id: 0,
                events: account::new_event_handle<StartDKGEvent>(aptos_framework),
            }
        );
    }

    public (friend) fun get_state(): u64 acquires DKGState  {
        let dkg_state = borrow_global<DKGState>(@aptos_framework);
        dkg_state.state_id
    }

    public (friend) fun state_started(): u64 {
        1
    }

    public (friend) fun state_not_started(): u64 {
        0
    }

    public(friend) fun start(validator_set_and_stake_dist: vector<u8>) acquires DKGState {
        debug::print(&std::string::utf8(b"dkg::start() started."));
        let dkg_state = borrow_global_mut<DKGState>(@aptos_framework);
        if (dkg_state.state_id != 0) {
            debug::print(&std::string::utf8(b"dkg::start() called while dkg already started."));
            return;
        };
        dkg_state.state_id = 1;
        event::emit_event<StartDKGEvent>(
            &mut dkg_state.events,
            StartDKGEvent {
                validator_set_and_stake_dist,
            },
        );
        debug::print(&std::string::utf8(b"dkg::start() finished."));
    }

    public(friend) fun finish() acquires DKGState {
        debug::print(&std::string::utf8(b"dkg::finish() started."));
        let dkg_state = borrow_global_mut<DKGState>(@aptos_framework);
        dkg_state.state_id = 0;
        debug::print(&std::string::utf8(b"dkg::finish() finished."));
    }
}
