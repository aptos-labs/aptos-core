module aptos_framework::dkg {
    use std::error;
    use std::option;
    use std::option::Option;
    use aptos_framework::event::emit;
    use aptos_framework::stake::ValidatorSet;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;
    friend aptos_framework::block;
    friend aptos_framework::genesis;
    friend aptos_framework::reconfiguration_with_dkg;

    const EDKG_IN_PROGRESS: u64 = 1;
    const EDKG_NOT_IN_PROGRESS: u64 = 2;
    const EINVALID_GUID_FOR_EVENT: u64 = 3;

    #[event]
    struct DKGStartEvent has drop, store {
        target_epoch: u64,
        target_validator_set: ValidatorSet,
    }

    /// The input and output of a DKG session.
    /// The validator set of epoch `x` works together for an DKG output for the target validator set of epoch `x+1`.
    struct DKGSessionState has copy, store, drop {
        dealer_epoch: u64,
        dealer_validator_set: ValidatorSet,
        target_epoch: u64,
        target_validator_set: ValidatorSet,
        result: vector<u8>,
        deadline_microseconds: u64,
    }

    /// The complete and ongoing DKG sessions.
    struct DKGState has key {
        last_complete: Option<DKGSessionState>,
        in_progress: Option<DKGSessionState>,
    }

    public(friend) fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        move_to<DKGState>(
            aptos_framework,
            DKGState {
                last_complete: std::option::none(),
                in_progress: std::option::none(),
            }
        );
    }

    public(friend) fun start(
        dealer_epoch: u64,
        dealer_validator_set: ValidatorSet,
        target_epoch: u64,
        target_validator_set: ValidatorSet
    ) acquires DKGState {
        let dkg_state = borrow_global_mut<DKGState>(@aptos_framework);
        assert!(std::option::is_none(&dkg_state.in_progress), error::invalid_state(EDKG_IN_PROGRESS));
        dkg_state.in_progress = std::option::some(DKGSessionState {
            dealer_epoch,
            dealer_validator_set,
            target_epoch,
            target_validator_set,
            deadline_microseconds: timestamp::now_microseconds() + 999999999, // Expect DKG to finish.
            result: vector[],
        });

        let event = DKGStartEvent {
            target_epoch,
            target_validator_set,
        };
        emit(event);
    }

    /// Update the current DKG state with a potential result.
    /// Return true if the current DKG becomes inactive and we should start a new epoch.
    /// Abort if no DKG is in progress.
    public(friend) fun update(dkg_result_available: bool, dkg_result: vector<u8>): bool acquires DKGState {
        let dkg_state = borrow_global_mut<DKGState>(@aptos_framework);
        assert!(option::is_some(&dkg_state.in_progress), error::invalid_state(EDKG_NOT_IN_PROGRESS));
        let session = option::extract(&mut dkg_state.in_progress);
        let dkg_completed = false;
        if (dkg_result_available) {
            session.result = dkg_result;
            dkg_completed = true;
        };
        if (timestamp::now_microseconds() >= session.deadline_microseconds || dkg_completed) {
            dkg_state.last_complete = option::some(session);
            dkg_state.in_progress = option::none();
            true
        } else {
            dkg_state.in_progress = option::some(session);
            false
        }
    }

    public(friend) fun in_progress(): bool acquires DKGState {
        option::is_some(&borrow_global<DKGState>(@aptos_framework).in_progress)
    }

    public(friend) fun current_deadline(): u64 acquires DKGState {
        let in_progress_session = option::borrow(&borrow_global<DKGState>(@aptos_framework).in_progress);
        in_progress_session.deadline_microseconds
    }
}
