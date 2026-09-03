// Initialize on-chain resources for the encrypted mempool stack: ChunkyDKG
// (config + seqnum + state), the epoch force-end watchdog, and the per-block
// decryption key.
//
// Each `initialize` call is a no-op if its resource already exists, so this is
// safe to run even if some of these resources were created previously.
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::chunky_dkg;
    use aptos_framework::chunky_dkg_config;
    use aptos_framework::chunky_dkg_config_seqnum;
    use aptos_framework::decryption;
    use aptos_framework::epoch_timeout_config;

    fun main(proposal_id: u64) {
        let framework = aptos_governance::resolve_multi_step_proposal(
            proposal_id,
            @0x1,
            {{ script_hash }},
        );

        chunky_dkg_config_seqnum::initialize(&framework);
        chunky_dkg_config::initialize(&framework, chunky_dkg_config::new_off());
        chunky_dkg::initialize(&framework);
        epoch_timeout_config::initialize(&framework);
        decryption::initialize(&framework);
    }
}
