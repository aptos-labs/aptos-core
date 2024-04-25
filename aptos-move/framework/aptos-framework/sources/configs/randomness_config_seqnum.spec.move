spec aptos_framework::randomness_config_seqnum {
    spec on_new_epoch() {
        pragma verify = false;
    }

    spec on_new_epoch_v2(framework: &signer) {
        requires @aptos_framework == std::signer::address_of(framework);
        include config_buffer::OnNewEpochRequirement<RandomnessConfigSeqNum>;
        aborts_if false;
    }
}
