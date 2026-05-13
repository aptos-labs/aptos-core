/// DKG on-chain states and helper functions.
module aptos_framework::dkg {
    use std::error;
    use std::option;
    use std::option::Option;
    use aptos_framework::event::emit;
    use aptos_framework::randomness_config::RandomnessConfig;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;
    use aptos_framework::validator_consensus_info::ValidatorConsensusInfo;
    friend aptos_framework::block;
    friend aptos_framework::reconfiguration_with_dkg;

    const EDKG_IN_PROGRESS: u64 = 1;
    const EDKG_NOT_IN_PROGRESS: u64 = 2;

    /// This can be considered as the public input of DKG.
    struct DKGSessionMetadata has copy, drop, store {
        dealer_epoch: u64,
        randomness_config: RandomnessConfig,
        dealer_validator_set: vector<ValidatorConsensusInfo>,
        target_validator_set: vector<ValidatorConsensusInfo>,
    }

    #[event]
    struct DKGStartEvent has drop, store {
        session_metadata: DKGSessionMetadata,
        start_time_us: u64,
    }

    /// The input and output of a DKG session.
    /// The validator set of epoch `x` works together for an DKG output for the target validator set of epoch `x+1`.
    struct DKGSessionState has copy, store, drop {
        metadata: DKGSessionMetadata,
        start_time_us: u64,
        transcript: vector<u8>,
    }

    /// The completed and in-progress DKG sessions.
    struct DKGState has key {
        last_completed: Option<DKGSessionState>,
        in_progress: Option<DKGSessionState>,
    }

    /// Called in genesis to initialize on-chain states.
    public fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        if (!exists<DKGState>(@aptos_framework)) {
            move_to<DKGState>(
                aptos_framework,
                DKGState {
                    last_completed: std::option::none(),
                    in_progress: std::option::none(),
                }
            );
        }
    }

    /// Mark on-chain DKG state as in-progress. Notify validators to start DKG.
    /// Idempotent for `dealer_epoch`: if a session for this epoch has already
    /// been started (in_progress or last_completed), returns without
    /// overwriting state or re-emitting an event. This enforces the
    /// invariant "at most one DKGStartEvent per epoch."
    public(friend) fun start(
        dealer_epoch: u64,
        randomness_config: RandomnessConfig,
        dealer_validator_set: vector<ValidatorConsensusInfo>,
        target_validator_set: vector<ValidatorConsensusInfo>,
    ) acquires DKGState {
        if (is_session_started(dealer_epoch)) { return };
        let dkg_state = borrow_global_mut<DKGState>(@aptos_framework);
        let new_session_metadata = DKGSessionMetadata {
            dealer_epoch,
            randomness_config,
            dealer_validator_set,
            target_validator_set,
        };
        let start_time_us = timestamp::now_microseconds();
        dkg_state.in_progress = std::option::some(DKGSessionState {
            metadata: new_session_metadata,
            start_time_us,
            transcript: vector[],
        });

        emit(DKGStartEvent {
            start_time_us,
            session_metadata: new_session_metadata,
        });
    }

    /// Put a transcript into the currently incomplete DKG session, then mark it completed.
    ///
    /// Abort if DKG is not in progress.
    public(friend) fun finish(transcript: vector<u8>) acquires DKGState {
        let dkg_state = borrow_global_mut<DKGState>(@aptos_framework);
        assert!(dkg_state.in_progress.is_some(), error::invalid_state(EDKG_NOT_IN_PROGRESS));
        let session = dkg_state.in_progress.extract();
        session.transcript = transcript;
        dkg_state.last_completed = option::some(session);
        dkg_state.in_progress = option::none();
    }

    /// Delete the currently incomplete session, if it exists.
    public fun try_clear_incomplete_session(fx: &signer) acquires DKGState {
        system_addresses::assert_aptos_framework(fx);
        if (exists<DKGState>(@aptos_framework)) {
            let dkg_state = borrow_global_mut<DKGState>(@aptos_framework);
            dkg_state.in_progress = option::none();
        }
    }

    /// Return the incomplete DKG session state, if it exists.
    public fun incomplete_session(): Option<DKGSessionState> acquires DKGState {
        if (exists<DKGState>(@aptos_framework)) {
            borrow_global<DKGState>(@aptos_framework).in_progress
        } else {
            option::none()
        }
    }

    /// Return the last completed DKG session state, if any.
    public fun last_completed_session(): Option<DKGSessionState> acquires DKGState {
        if (exists<DKGState>(@aptos_framework)) {
            borrow_global<DKGState>(@aptos_framework).last_completed
        } else {
            option::none()
        }
    }

    /// Return the dealer epoch of a `DKGSessionState`.
    public fun session_dealer_epoch(session: &DKGSessionState): u64 {
        session.metadata.dealer_epoch
    }

    /// True iff a DKG session has ever been started for `epoch` (in_progress
    /// for `epoch` OR last_completed for `epoch`). Used by `dkg::start` to
    /// enforce "at most one DKGStartEvent per epoch".
    public fun is_session_started(epoch: u64): bool acquires DKGState {
        let in_prog = incomplete_session();
        if (in_prog.is_some() && session_dealer_epoch(in_prog.borrow()) == epoch) {
            return true
        };
        let last = last_completed_session();
        if (last.is_some() && session_dealer_epoch(last.borrow()) == epoch) {
            return true
        };
        false
    }
}
