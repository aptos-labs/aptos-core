module aptos_framework::dkg {
    use std::error;
    use std::option::Option;
    use std::signer;
    use std::string::utf8;
    use aptos_std::debug;
    use aptos_framework::account;
    use aptos_framework::event;
    use aptos_framework::stake::ValidatorSet;
    use aptos_framework::system_addresses;

    friend aptos_framework::block;
    friend aptos_framework::genesis;
    friend aptos_framework::reconfiguration;

    /// An invalid block time was encountered.
    const EINVALID_GUID_FOR_EVENT: u64 = 5;

    struct StartDKGEvent has drop, store {
        target_epoch: u64,
        target_validator_set: ValidatorSet,
    }

    /// The input and output of a DKG session.
    /// The validator set of epoch `x` works together and outputs a transcript for the target validator set of epoch `y` (typically `x+1`).
    struct DKGSessionState has copy, store, drop {
        dealer_epoch: u64,
        dealer_validator_set: ValidatorSet,
        target_epoch: u64,
        target_validator_set: ValidatorSet,
        serialized_transcript: vector<u8>,
    }

    /// The complete and ongoing DKG sessions.
    struct DKGState has key {
        last_complete: Option<DKGSessionState>,
        in_progress: Option<DKGSessionState>,
        events: event::EventHandle<StartDKGEvent>,
    }

    public(friend) fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(5 == account::get_guid_next_creation_num(signer::address_of(aptos_framework)), error::invalid_state(EINVALID_GUID_FOR_EVENT));
        move_to<DKGState>(
            aptos_framework,
            DKGState {
                last_complete: std::option::none(),
                in_progress: std::option::none(),
                events: account::new_event_handle<StartDKGEvent>(aptos_framework),
            }
        );
    }

    /// Return the currently in-progress DKG session, if there is one.
    public(friend) fun session_in_progress(): Option<DKGSessionState> acquires DKGState {
        let dkg_state = borrow_global<DKGState>(@aptos_framework);
        dkg_state.in_progress
    }

    /// Mark the start of a new DKG session by storing the input. Also emit the `StartDKGEvent`.
    public(friend) fun start(dealer_epoch: u64, dealer_validator_set: ValidatorSet, target_epoch: u64, target_validator_set: ValidatorSet) acquires DKGState {
        debug::print(&utf8(b"dkg::start() - Started."));
        let dkg_state = borrow_global_mut<DKGState>(@aptos_framework);
        assert!(std::option::is_none(&dkg_state.in_progress), 1);
        dkg_state.in_progress = std::option::some(DKGSessionState {
            dealer_epoch,
            dealer_validator_set,
            target_epoch,
            target_validator_set,
            serialized_transcript: vector[],
        });
        event::emit_event<StartDKGEvent>(
            &mut dkg_state.events,
            StartDKGEvent {
                target_epoch,
                target_validator_set,
            },
        );
        debug::print(&utf8(b"dkg::start() - Finished."))
    }

    /// Mark the ongoing DKG session complete.
    public(friend) fun finish(serialized_transcript: vector<u8>) acquires DKGState {
        debug::print(&std::string::utf8(b"dkg::finish() - Started."));
        let session_in_progress = std::option::extract(&mut session_in_progress());
        session_in_progress.serialized_transcript = serialized_transcript;
        let state = borrow_global_mut<DKGState>(@aptos_framework);
        state.last_complete = std::option::some(session_in_progress);
        state.in_progress = std::option::none();
        debug::print(&std::string::utf8(b"dkg::finish() - Finished."))
    }
}
