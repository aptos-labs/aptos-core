spec aptos_framework::chunky_dkg_config_seqnum {
    spec on_new_epoch(framework: &signer) {
        requires @aptos_framework == std::signer::address_of(framework);
        include config_buffer::OnNewEpochRequirement<ChunkyDKGConfigSeqNum>;
        aborts_if false;
    }
}
