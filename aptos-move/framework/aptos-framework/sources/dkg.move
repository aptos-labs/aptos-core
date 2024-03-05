/// DKG on-chain states and helper functions.
module aptos_framework::dkg {
    use std::error;
    use std::option;
    use std::option::Option;
    use aptos_framework::event::emit;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;
    use aptos_framework::validator_consensus_info::ValidatorConsensusInfo;
    friend aptos_framework::block;
    friend aptos_framework::reconfiguration_with_dkg;

    const EDKG_IN_PROGRESS: u64 = 1;
    const EDKG_NOT_IN_PROGRESS: u64 = 2;

    /// If this resource is present under 0x1, validators should not do DKG (so the epoch change get stuck).
    /// This is test-only.
    struct FailureInjectionBlockDKG has drop, key {}

    /// If this resource is present under 0x1, validators should not provider randomness to block (so the execution get stuck).
    /// This is test-only.
    struct FailureInjectionBlockRandomness has drop, key {}

    /// This can be considered as the public input of DKG.
    struct DKGSessionMetadata has copy, drop, store {
        dealer_epoch: u64,
        dealer_validator_set: vector<ValidatorConsensusInfo>,
        target_validator_set: vector<ValidatorConsensusInfo>,
        block_dkg: bool,
        block_randomness: bool,
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

    public fun block_dkg(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);
        if (!exists<FailureInjectionBlockDKG>(@aptos_framework)) {
            move_to(framework, FailureInjectionBlockDKG {})
        }
    }

    public fun unblock_dkg(framework: &signer) acquires FailureInjectionBlockDKG {
        system_addresses::assert_aptos_framework(framework);
        if (exists<FailureInjectionBlockDKG>(@aptos_framework)) {
            move_from<FailureInjectionBlockDKG>(@aptos_framework);
        }
    }

    public fun block_randomness(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);
        if (!exists<FailureInjectionBlockRandomness>(@aptos_framework)) {
            move_to(framework, FailureInjectionBlockRandomness {})
        }
    }

    public fun unblock_randomness(framework: &signer) acquires FailureInjectionBlockRandomness {
        system_addresses::assert_aptos_framework(framework);
        if (!exists<FailureInjectionBlockRandomness>(@aptos_framework)) {
            move_from<FailureInjectionBlockRandomness>(@aptos_framework);
        }
    }

    /// Called in genesis to initialize on-chain states.
    public fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        move_to<DKGState>(
            aptos_framework,
            DKGState {
                last_completed: std::option::none(),
                in_progress: std::option::none(),
            }
        );
    }

    /// Mark on-chain DKG state as in-progress. Notify validators to start DKG.
    /// Abort if a DKG is already in progress.
    public(friend) fun start(
        dealer_epoch: u64,
        dealer_validator_set: vector<ValidatorConsensusInfo>,
        target_validator_set: vector<ValidatorConsensusInfo>,
    ) acquires DKGState {
        let dkg_state = borrow_global_mut<DKGState>(@aptos_framework);
        assert!(std::option::is_none(&dkg_state.in_progress), error::invalid_state(EDKG_IN_PROGRESS));
        let new_session_metadata = DKGSessionMetadata {
            dealer_epoch,
            dealer_validator_set,
            target_validator_set,
            block_dkg: exists<FailureInjectionBlockDKG>(@aptos_framework),
            block_randomness: exists<FailureInjectionBlockRandomness>(@aptos_framework),
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

    /// Update the current DKG state at the beginning of every block in `block_prologue_ext()`,
    /// or when DKG result is available.
    ///
    /// Return true if and only if this update completes/aborts the DKG and we should proceed to the next epoch.
    ///
    /// Abort if DKG is not in progress.
    public(friend) fun finish(transcript: vector<u8>) acquires DKGState {
        let dkg_state = borrow_global_mut<DKGState>(@aptos_framework);
        assert!(option::is_some(&dkg_state.in_progress), error::invalid_state(EDKG_NOT_IN_PROGRESS));
        let session = option::extract(&mut dkg_state.in_progress);
        session.transcript = transcript;
        dkg_state.last_completed = option::some(session);
        dkg_state.in_progress = option::none();
    }

    /// Return whether a DKG is in progress.
    public(friend) fun in_progress(): bool acquires DKGState {
        if (exists<DKGState>(@aptos_framework)) {
            option::is_some(&borrow_global<DKGState>(@aptos_framework).in_progress)
        } else {
            false
        }
    }

    public fun clean_up_in_progress_session(fx: &signer) acquires DKGState {
        system_addresses::assert_aptos_framework(fx);
        if (exists<DKGState>(@aptos_framework)) {
            let dkg_state = borrow_global_mut<DKGState>(@aptos_framework);
            dkg_state.in_progress = option::none();
        }
    }
}
