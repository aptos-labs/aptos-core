// Enable on-chain randomness.
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::randomness_api_v0_config;
    use std::option;

    fun main(proposal_id: u64) {
        let framework = aptos_governance::resolve_multi_step_proposal(
            proposal_id,
            @0x1,
            {{ script_hash }},
        );
        // Default gas deposit is 10000.
        randomness_api_v0_config::set_for_next_epoch(&framework, option::some(10000));
        aptos_governance::reconfigure(&framework);
    }
}
