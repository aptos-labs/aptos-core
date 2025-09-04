// Enable on-chain randomness.
script {
    use velor_framework::velor_governance;
    use velor_framework::randomness_config;
    use velor_std::fixed_point64;

    fun main(proposal_id: u64) {
        let framework = velor_governance::resolve_multi_step_proposal(
            proposal_id,
            @0x1,
            {{ script_hash }},
        );

        let config = randomness_config::new_v1(
            fixed_point64::create_from_rational(1, 2), // secrecy_threshold: 1/2
            fixed_point64::create_from_rational(2, 3), // reconstruct_threshold: 2/3
        );
        randomness_config::set_for_next_epoch(&framework, config);
        velor_governance::reconfigure(&framework); // The resulting epoch does not have randomness. The one after it does.
    }
}
