spec aptos_framework::dkg {

    spec module {
        use aptos_framework::chain_status;
        invariant [suspendable] chain_status::is_operating() ==> exists<DKGState>(@aptos_framework);
    }

    spec initialize(aptos_framework: &signer) {
        use std::signer;
        let aptos_framework_addr = signer::address_of(aptos_framework);
        aborts_if aptos_framework_addr != @aptos_framework;
    }

    spec start(
        dealer_epoch: u64,
        randomness_config: RandomnessConfig,
        dealer_validator_set: vector<ValidatorConsensusInfo>,
        target_validator_set: vector<ValidatorConsensusInfo>,
    ) {
        aborts_if !exists<DKGState>(@aptos_framework);
        // Non-idempotent path also reads the clock; idempotent path does not.
        aborts_if !spec_is_session_started(dealer_epoch)
            && !exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework);
        // Post: a session for `dealer_epoch` is started.
        ensures spec_is_session_started(dealer_epoch);
        // Invariant "at most one DKGStartEvent per epoch": if a session was
        // already started for `dealer_epoch`, the state is unchanged (and no
        // new event is emitted).
        ensures old(spec_is_session_started(dealer_epoch))
            ==> global<DKGState>(@aptos_framework) == old(global<DKGState>(@aptos_framework));
    }

    spec last_completed_session(): Option<DKGSessionState> {
        aborts_if false;
        ensures result == if (exists<DKGState>(@aptos_framework)) {
            global<DKGState>(@aptos_framework).last_completed
        } else {
            option::spec_none()
        };
    }

    spec is_session_started(epoch: u64): bool {
        aborts_if false;
        ensures result == spec_is_session_started(epoch);
    }

    /// A session has been started for `epoch` iff either the in-progress or
    /// the last-completed session on chain has `dealer_epoch == epoch`.
    spec fun spec_is_session_started(epoch: u64): bool {
        exists<DKGState>(@aptos_framework) && (
            (option::is_some(global<DKGState>(@aptos_framework).in_progress)
             && option::borrow(global<DKGState>(@aptos_framework).in_progress)
                    .metadata.dealer_epoch == epoch)
            || (option::is_some(global<DKGState>(@aptos_framework).last_completed)
                && option::borrow(global<DKGState>(@aptos_framework).last_completed)
                       .metadata.dealer_epoch == epoch)
        )
    }

    spec finish(transcript: vector<u8>) {
        use std::option;
        requires exists<DKGState>(@aptos_framework);
        requires option::is_some(global<DKGState>(@aptos_framework).in_progress);
        aborts_if false;
    }

    spec fun has_incomplete_session(): bool {
        if (exists<DKGState>(@aptos_framework)) {
            option::is_some(global<DKGState>(@aptos_framework).in_progress)
        } else {
            false
        }
    }

    spec try_clear_incomplete_session(fx: &signer) {
        use std::signer;
        let addr = signer::address_of(fx);
        aborts_if addr != @aptos_framework;
    }

    spec incomplete_session(): Option<DKGSessionState> {
        aborts_if false;
    }
}
