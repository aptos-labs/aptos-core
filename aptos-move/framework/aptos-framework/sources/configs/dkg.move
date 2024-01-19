/// DKG on-chain states and helper functions.
module aptos_framework::dkg {
    use std::error;
    use std::option;
    use std::option::Option;
    use aptos_framework::event::emit;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;
    friend aptos_framework::block;
    friend aptos_framework::genesis;

    const EDKG_IN_PROGRESS: u64 = 1;
    const EDKG_NOT_IN_PROGRESS: u64 = 2;

    /// Information about a validator that participates DKG.
    struct ValidatorInfo has copy, drop, store {
        addr: address,
        pk_bytes: vector<u8>,
        voting_power: u64,
    }

    /// This can be considered as the public input of DKG.
    struct DKGSessionMetadata has copy, drop, store {
        dealer_epoch: u64,
        dealer_validator_set: vector<ValidatorInfo>,
        target_validator_set: vector<ValidatorInfo>,
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

    /// Create a `ValidatorInfo` object.
    public fun new_validator_info(addr: address, pk_bytes: vector<u8>, voting_power: u64): ValidatorInfo {
        ValidatorInfo {
            addr,
            pk_bytes,
            voting_power,
        }
    }

    /// Called in genesis to initialize on-chain states.
    public(friend) fun initialize(aptos_framework: &signer) {
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
        dealer_validator_set: vector<ValidatorInfo>,
        target_validator_set: vector<ValidatorInfo>,
    ) acquires DKGState {
        let dkg_state = borrow_global_mut<DKGState>(@aptos_framework);
        assert!(std::option::is_none(&dkg_state.in_progress), error::invalid_state(EDKG_IN_PROGRESS));
        let new_session_metadata = DKGSessionMetadata {
            dealer_epoch,
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
}
