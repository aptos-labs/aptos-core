// Disable randomness fast path by downgrading from V2 to V1.
// V1 keeps standard randomness enabled but does not include the fast path.
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::randomness_config;
    use aptos_std::fixed_point64;

    fun main(proposal_id: u64) {
        let framework = aptos_governance::resolve_multi_step_proposal(
            proposal_id,
            @0x1,
            {{ script_hash }},
        );

        let config = randomness_config::new_v1(
            fixed_point64::create_from_rational(1, 2), // secrecy_threshold: 1/2
            fixed_point64::create_from_rational(2, 3), // reconstruct_threshold: 2/3
        );
        randomness_config::set_for_next_epoch(&framework, config);
        aptos_governance::reconfigure(&framework);
    }
}
