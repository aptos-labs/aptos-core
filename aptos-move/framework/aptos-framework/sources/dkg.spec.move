spec aptos_framework::dkg {

    spec module {
        use aptos_framework::chain_status;
        invariant [suspendable] chain_status::is_operating() ==> exists<DKGState>(@aptos_framework);
    }

    spec initialize(aptos_framework: &signer) {
        use std::signer;
        let aptos_framework_addr = signer::address_of(aptos_framework);
        aborts_if aptos_framework_addr != @aptos_framework;
        aborts_if exists<DKGState>(@aptos_framework);
    }

    spec start(
        dealer_epoch: u64,
        dealer_validator_set: vector<ValidatorConsensusInfo>,
        target_validator_set: vector<ValidatorConsensusInfo>,
    ) {
        use std::option;
        aborts_if !exists<DKGState>(@aptos_framework);
        aborts_if option::is_some(global<DKGState>(@aptos_framework).in_progress);
        aborts_if !exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework);
    }

    spec finish(transcript: vector<u8>) {
        use std::option;
        aborts_if !exists<DKGState>(@aptos_framework);
        aborts_if option::is_none(global<DKGState>(@aptos_framework).in_progress);
    }

    spec in_progress(): bool {
        aborts_if false;
        ensures result == spec_in_progress();
    }

    spec fun spec_in_progress(): bool {
        if (exists<DKGState>(@aptos_framework)) {
            option::spec_is_some(global<DKGState>(@aptos_framework).in_progress)
        } else {
            false
        }
    }

}
