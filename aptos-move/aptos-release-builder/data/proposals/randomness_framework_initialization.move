// Initialize on-chain randomness resources.
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::config_buffer;
    use aptos_framework::dkg;
    use aptos_framework::randomness;
    use aptos_framework::randomness_config;
    use aptos_framework::reconfiguration_state;

    fun main(proposal_id: u64) {
        let framework = aptos_governance::resolve_multi_step_proposal(
            proposal_id,
            @0x1,
            {{ script_hash }},
        );
        config_buffer::initialize(&framework); // on-chain config buffer
        dkg::initialize(&framework); // DKG state holder
        reconfiguration_state::initialize(&framework); // reconfiguration in progress global indicator
        randomness::initialize(&framework); // randomness seed holder

        let config = randomness_config::new_off();
        randomness_config::initialize(&framework, config);
    }
}
