// Script hash: 7d66abab 
// Set transaction_limits execution and IO tiers via governance.
// Execution: 2x @ 1M APT, 4x @ 5M APT, 8x @ 10M APT.
// IO:        2x @ 5M APT, 4x @ 10M APT, 8x @ 20M APT.
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::transaction_limits;

    fun main(proposal_id: u64) {
        let framework_signer = aptos_governance::resolve_multi_step_proposal(
            proposal_id,
            @0x1,
            vector[],
        );

        // Octas = APT * 10^8.
        let execution_min_stakes = vector[
            100_000_000_000_000,    //  1 M APT
            500_000_000_000_000,    //  5 M APT
          1_000_000_000_000_000,    // 10 M APT
        ];
        let execution_multipliers_percent = vector[200, 400, 800];

        let io_min_stakes = vector[
            500_000_000_000_000,    //  5 M APT
          1_000_000_000_000_000,    // 10 M APT
          2_000_000_000_000_000,    // 20 M APT
        ];
        let io_multipliers_percent = vector[200, 400, 800];

        transaction_limits::update_config(
            &framework_signer,
            execution_min_stakes,
            execution_multipliers_percent,
            io_min_stakes,
            io_multipliers_percent,
        );
    }
}
