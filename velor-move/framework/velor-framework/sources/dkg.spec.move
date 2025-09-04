spec velor_framework::dkg {

    spec module {
        use velor_framework::chain_status;
        invariant [suspendable] chain_status::is_operating() ==> exists<DKGState>(@velor_framework);
    }

    spec initialize(velor_framework: &signer) {
        use std::signer;
        let velor_framework_addr = signer::address_of(velor_framework);
        aborts_if velor_framework_addr != @velor_framework;
    }

    spec start(
        dealer_epoch: u64,
        randomness_config: RandomnessConfig,
        dealer_validator_set: vector<ValidatorConsensusInfo>,
        target_validator_set: vector<ValidatorConsensusInfo>,
    ) {
        aborts_if !exists<DKGState>(@velor_framework);
        aborts_if !exists<timestamp::CurrentTimeMicroseconds>(@velor_framework);
    }

    spec finish(transcript: vector<u8>) {
        use std::option;
        requires exists<DKGState>(@velor_framework);
        requires option::is_some(global<DKGState>(@velor_framework).in_progress);
        aborts_if false;
    }

    spec fun has_incomplete_session(): bool {
        if (exists<DKGState>(@velor_framework)) {
            option::spec_is_some(global<DKGState>(@velor_framework).in_progress)
        } else {
            false
        }
    }

    spec try_clear_incomplete_session(fx: &signer) {
        use std::signer;
        let addr = signer::address_of(fx);
        aborts_if addr != @velor_framework;
    }

    spec incomplete_session(): Option<DKGSessionState> {
        aborts_if false;
    }
}
