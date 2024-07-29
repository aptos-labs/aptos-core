spec supra_framework::dkg {

    spec module {
        use supra_framework::chain_status;
        invariant [suspendable] chain_status::is_operating() ==> exists<DKGState>(@supra_framework);
    }

    spec initialize(supra_framework: &signer) {
        use std::signer;
        let supra_framework_addr = signer::address_of(supra_framework);
        aborts_if supra_framework_addr != @supra_framework;
    }

    spec start(
        dealer_epoch: u64,
        randomness_config: RandomnessConfig,
        dealer_validator_set: vector<ValidatorConsensusInfo>,
        target_validator_set: vector<ValidatorConsensusInfo>,
    ) {
        aborts_if !exists<DKGState>(@supra_framework);
        aborts_if !exists<timestamp::CurrentTimeMicroseconds>(@supra_framework);
    }

    spec finish(transcript: vector<u8>) {
        use std::option;
        requires exists<DKGState>(@supra_framework);
        requires option::is_some(global<DKGState>(@supra_framework).in_progress);
        aborts_if false;
    }

    spec fun has_incomplete_session(): bool {
        if (exists<DKGState>(@supra_framework)) {
            option::spec_is_some(global<DKGState>(@supra_framework).in_progress)
        } else {
            false
        }
    }

    spec try_clear_incomplete_session(fx: &signer) {
        use std::signer;
        let addr = signer::address_of(fx);
        aborts_if addr != @supra_framework;
    }

    spec incomplete_session(): Option<DKGSessionState> {
        aborts_if false;
    }
}
