// Enable ChunkyDKG (ConfigV1) — full activation, replacing regular DKG.
// Prerequisite: ChunkyDKG framework resources must already be initialized
// (see encrypted_mempool_initialization.move).
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::chunky_dkg_config;
    use aptos_std::fixed_point64;

    fun main(proposal_id: u64) {
        let framework = aptos_governance::resolve_multi_step_proposal(
            proposal_id,
            @0x1,
            {{ script_hash }},
        );

        let config = chunky_dkg_config::new_v1(
            fixed_point64::create_from_rational(1, 2), // secrecy_threshold: 1/2
            fixed_point64::create_from_rational(2, 3), // reconstruction_threshold: 2/3
        );
        chunky_dkg_config::set_for_next_epoch(&framework, config);
        aptos_governance::reconfigure(&framework);
    }
}
