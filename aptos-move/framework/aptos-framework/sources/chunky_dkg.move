/// Chunky DKG on-chain states and helper functions.
module aptos_framework::chunky_dkg {
    use std::error;
    use std::option;
    use std::option::Option;
    use aptos_framework::event::emit;
    use aptos_framework::chunky_dkg_config::ChunkyDKGConfig;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;
    use aptos_framework::validator_consensus_info::ValidatorConsensusInfo;
    friend aptos_framework::block;
    friend aptos_framework::reconfiguration_with_dkg;

    const ECHUNKY_DKG_IN_PROGRESS: u64 = 1;
    const ECHUNKY_DKG_NOT_IN_PROGRESS: u64 = 2;

    /// This can be considered as the public input of Chunky DKG.
    struct ChunkyDKGSessionMetadata has copy, drop, store {
        dealer_epoch: u64,
        chunky_dkg_config: ChunkyDKGConfig,
        dealer_validator_set: vector<ValidatorConsensusInfo>,
        target_validator_set: vector<ValidatorConsensusInfo>
    }

    #[event]
    struct ChunkyDKGStartEvent has drop, store {
        session_metadata: ChunkyDKGSessionMetadata,
        start_time_us: u64
    }

    /// The input and output of a Chunky DKG session.
    /// The validator set of epoch `x` works together for a Chunky DKG output for the target validator set of epoch `x+1`.
    struct ChunkyDKGSessionState has copy, store, drop {
        metadata: ChunkyDKGSessionMetadata,
        start_time_us: u64,
        aggregated_subtranscript: vector<u8>
    }

    /// The completed and in-progress Chunky DKG sessions.
    struct ChunkyDKGState has key {
        last_completed: Option<ChunkyDKGSessionState>,
        in_progress: Option<ChunkyDKGSessionState>
    }

    /// Called in genesis to initialize on-chain states.
    public fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        if (!exists<ChunkyDKGState>(@aptos_framework)) {
            move_to<ChunkyDKGState>(
                aptos_framework,
                ChunkyDKGState {
                    last_completed: std::option::none(),
                    in_progress: std::option::none()
                }
            );
        }
    }

    /// Mark on-chain Chunky DKG state as in-progress. Notify validators to start Chunky DKG.
    public(friend) fun start(
        dealer_epoch: u64,
        chunky_dkg_config: ChunkyDKGConfig,
        dealer_validator_set: vector<ValidatorConsensusInfo>,
        target_validator_set: vector<ValidatorConsensusInfo>
    ) acquires ChunkyDKGState {
        let chunky_dkg_state = borrow_global_mut<ChunkyDKGState>(@aptos_framework);
        let new_session_metadata = ChunkyDKGSessionMetadata {
            dealer_epoch,
            chunky_dkg_config,
            dealer_validator_set,
            target_validator_set
        };
        let start_time_us = timestamp::now_microseconds();
        chunky_dkg_state.in_progress = std::option::some(
            ChunkyDKGSessionState {
                metadata: new_session_metadata,
                start_time_us,
                aggregated_subtranscript: vector[]
            }
        );

        emit(
            ChunkyDKGStartEvent { start_time_us, session_metadata: new_session_metadata }
        );
    }

    /// Put a transcript into the currently incomplete Chunky DKG session, then mark it completed.
    ///
    /// Abort if Chunky DKG is not in progress.
    public(friend) fun finish(aggregated_subtranscript: vector<u8>) acquires ChunkyDKGState {
        let chunky_dkg_state = borrow_global_mut<ChunkyDKGState>(@aptos_framework);
        assert!(
            chunky_dkg_state.in_progress.is_some(),
            error::invalid_state(ECHUNKY_DKG_NOT_IN_PROGRESS)
        );
        let session = chunky_dkg_state.in_progress.extract();
        session.aggregated_subtranscript = aggregated_subtranscript;
        chunky_dkg_state.last_completed = option::some(session);
        chunky_dkg_state.in_progress = option::none();
    }

    /// Delete the currently incomplete session, if it exists.
    public fun try_clear_incomplete_session(fx: &signer) acquires ChunkyDKGState {
        system_addresses::assert_aptos_framework(fx);
        if (exists<ChunkyDKGState>(@aptos_framework)) {
            let chunky_dkg_state = borrow_global_mut<ChunkyDKGState>(@aptos_framework);
            chunky_dkg_state.in_progress = option::none();
        }
    }

    /// Return the incomplete Chunky DKG session state, if it exists.
    public fun incomplete_session(): Option<ChunkyDKGSessionState> acquires ChunkyDKGState {
        if (exists<ChunkyDKGState>(@aptos_framework)) {
            borrow_global<ChunkyDKGState>(@aptos_framework).in_progress
        } else {
            option::none()
        }
    }

    /// Return the dealer epoch of a `ChunkyDKGSessionState`.
    public fun session_dealer_epoch(session: &ChunkyDKGSessionState): u64 {
        session.metadata.dealer_epoch
    }
}
