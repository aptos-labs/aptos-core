/// DKG on-chain states and helper functions.
module velor_framework::dkg {
    use std::error;
    use std::option;
    use std::option::Option;
    use velor_framework::event::emit;
    use velor_framework::randomness_config::RandomnessConfig;
    use velor_framework::system_addresses;
    use velor_framework::timestamp;
    use velor_framework::validator_consensus_info::ValidatorConsensusInfo;
    friend velor_framework::block;
    friend velor_framework::reconfiguration_with_dkg;

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
    public fun initialize(velor_framework: &signer) {
        system_addresses::assert_velor_framework(velor_framework);
        if (!exists<DKGState>(@velor_framework)) {
            move_to<DKGState>(
                velor_framework,
                DKGState {
                    last_completed: std::option::none(),
                    in_progress: std::option::none(),
                }
            );
        }
    }

    /// Mark on-chain DKG state as in-progress. Notify validators to start DKG.
    /// Abort if a DKG is already in progress.
    public(friend) fun start(
        dealer_epoch: u64,
        randomness_config: RandomnessConfig,
        dealer_validator_set: vector<ValidatorConsensusInfo>,
        target_validator_set: vector<ValidatorConsensusInfo>,
    ) acquires DKGState {
        let dkg_state = borrow_global_mut<DKGState>(@velor_framework);
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
        let dkg_state = borrow_global_mut<DKGState>(@velor_framework);
        assert!(option::is_some(&dkg_state.in_progress), error::invalid_state(EDKG_NOT_IN_PROGRESS));
        let session = option::extract(&mut dkg_state.in_progress);
        session.transcript = transcript;
        dkg_state.last_completed = option::some(session);
        dkg_state.in_progress = option::none();
    }

    /// Delete the currently incomplete session, if it exists.
    public fun try_clear_incomplete_session(fx: &signer) acquires DKGState {
        system_addresses::assert_velor_framework(fx);
        if (exists<DKGState>(@velor_framework)) {
            let dkg_state = borrow_global_mut<DKGState>(@velor_framework);
            dkg_state.in_progress = option::none();
        }
    }

    /// Return the incomplete DKG session state, if it exists.
    public fun incomplete_session(): Option<DKGSessionState> acquires DKGState {
        if (exists<DKGState>(@velor_framework)) {
            borrow_global<DKGState>(@velor_framework).in_progress
        } else {
            option::none()
        }
    }

    /// Return the dealer epoch of a `DKGSessionState`.
    public fun session_dealer_epoch(session: &DKGSessionState): u64 {
        session.metadata.dealer_epoch
    }
}
