spec aptos_framework::chunky_dkg {

    spec initialize(aptos_framework: &signer) {
        use std::signer;
        let aptos_framework_addr = signer::address_of(aptos_framework);
        aborts_if aptos_framework_addr != @aptos_framework;
    }

    spec start(
        dealer_epoch: u64,
        chunky_dkg_config: ChunkyDKGConfig,
        dealer_validator_set: vector<ValidatorConsensusInfo>,
        target_validator_set: vector<ValidatorConsensusInfo>,
    ) {
        aborts_if !exists<ChunkyDKGState>(@aptos_framework);
        aborts_if !exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework);
    }

    spec finish(aggregated_subtranscript: vector<u8>) {
        use std::option;
        requires exists<ChunkyDKGState>(@aptos_framework);
        requires option::is_some(global<ChunkyDKGState>(@aptos_framework).in_progress);
        aborts_if false;
    }

    spec try_clear_incomplete_session(fx: &signer) {
        use std::signer;
        let addr = signer::address_of(fx);
        aborts_if addr != @aptos_framework;
    }

    spec incomplete_session(): Option<ChunkyDKGSessionState> {
        aborts_if false;
    }

    spec session_dealer_epoch(session: &ChunkyDKGSessionState): u64 {
        aborts_if false;
    }
}
