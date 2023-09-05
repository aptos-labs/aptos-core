module aptos_framework::dkg {
    use std::error;
    use std::option::Option;
    use std::signer;
    use std::string::utf8;
    use aptos_std::debug;
    use aptos_framework::account;
    use aptos_framework::event;
    use aptos_framework::stake::ValidatorInfo;
    use aptos_framework::system_addresses;

    friend aptos_framework::block;
    friend aptos_framework::genesis;
    friend aptos_framework::reconfiguration;

    /// An invalid block time was encountered.
    const EINVALID_GUID_FOR_EVENT: u64 = 5;

    struct StartDKGEvent has drop, store {
        target_epoch: u64,
        locked_new_validator_set: vector<ValidatorInfo>,
    }

    struct DKGState has key {
        target_epoch: u64,
        state_id: u64, // 0: done, 1: in progress,
        countdown: u64, // For debugging...
        /// DKG Transcript for current epoch.
        serialized_transcript: vector<u8>,
        events: event::EventHandle<StartDKGEvent>,
    }

    public(friend) fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(5 == account::get_guid_next_creation_num(signer::address_of(aptos_framework)), error::invalid_state(EINVALID_GUID_FOR_EVENT));
        move_to<DKGState>(
            aptos_framework,
            DKGState {
                target_epoch: 1,
                state_id: 0,
                countdown: 0,
                serialized_transcript: vector[],
                events: account::new_event_handle<StartDKGEvent>(aptos_framework),
            }
        );
    }

    public (friend) fun get_state(): (u64, u64) acquires DKGState  {
        let dkg_state = borrow_global<DKGState>(@aptos_framework);
        (dkg_state.target_epoch, dkg_state.state_id)
    }

    public (friend) fun state_active(): u64 {
        1
    }

    public (friend) fun state_inactive(): u64 {
        0
    }

    public(friend) fun start(target_epoch: u64, locked_new_validator_set: vector<ValidatorInfo>) acquires DKGState {
        debug::print(&utf8(b"dkg::start() started."));
        let dkg_state = borrow_global_mut<DKGState>(@aptos_framework);
        debug::print(&utf8(b"dkg_state="));
        debug::print(dkg_state);
        debug::print(&utf8(b"target_epoch="));
        debug::print(&target_epoch);
        if (target_epoch == dkg_state.target_epoch + 1 && dkg_state.state_id == 0) {
            dkg_state.target_epoch = target_epoch;
            dkg_state.state_id = 1;
            dkg_state.countdown = 999999999;
            event::emit_event<StartDKGEvent>(
                &mut dkg_state.events,
                StartDKGEvent {
                    target_epoch,
                    locked_new_validator_set,
                },
            );
        } else {
            debug::print(&utf8(b"unexpected dkg::start()..."));
        };
        debug::print(&utf8(b"dkg::start() finished."));
    }

    public(friend) fun on_potential_transcript(maybe_serialized_transcript: Option<vector<u8>>): bool acquires DKGState {
        debug::print(&std::string::utf8(b"dkg::on_potential_transcript() - Started."));
        let dkg_state = borrow_global_mut<DKGState>(@aptos_framework);
        assert!(state_active() == dkg_state.state_id, 1);
        let ret = if (std::option::is_some(&maybe_serialized_transcript)) {
            debug::print(&std::string::utf8(b"dkg::on_potential_transcript() - A transcript is given!"));
            dkg_state.state_id = 0;
            dkg_state.countdown = 0;
            dkg_state.serialized_transcript = std::option::extract(&mut maybe_serialized_transcript);
            debug::print(&dkg_state.serialized_transcript);
            true
        } else if (dkg_state.countdown == 0) {
            debug::print(&std::string::utf8(b"dkg::on_potential_transcript() - Current DKG is taking too long. Aborting."));
            dkg_state.state_id = 0;
            dkg_state.serialized_transcript = vector[];
            true
        } else {
            debug::print(&std::string::utf8(b"dkg::on_potential_transcript() - No transcript is given. Hopefully next block."));
            dkg_state.countdown = dkg_state.countdown - 1;
            false
        };
        debug::print(&std::string::utf8(b"dkg::on_potential_transcript() - Finished."));
        ret
    }
}
