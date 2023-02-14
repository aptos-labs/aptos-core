// Consensus config upgrade proposal

// config: V1(
//     ConsensusConfigV1 {
//         decoupled_execution: true,
//         back_pressure_limit: 10,
//         exclude_round: 20,
//         proposer_election_type: LeaderReputation(
//             ProposerAndVoterV2(
//                 ProposerAndVoterConfig {
//                     active_weight: 1000,
//                     inactive_weight: 10,
//                     failed_weight: 1,
//                     failure_threshold_percent: 10,
//                     proposer_window_num_validators_multiplier: 10,
//                     voter_window_num_validators_multiplier: 1,
//                     weight_by_voting_power: true,
//                     use_history_from_previous_epoch_max_count: 5,
//                 },
//             ),
//         ),
//         max_failed_authors_to_store: 10,
//     },
// )

script {
    use aptos_framework::aptos_governance;
    use aptos_framework::consensus_config;

    fun main(core_resources: &signer) {
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);

        let framework_signer = &core_signer;

        let consensus_blob: vector<u8> = vector[
            0, 1, 10, 0, 0, 0, 0, 0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0, 2, 1,
            232, 3, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0,
            0, 0, 0, 0, 10, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0,
            0, 0, 0, 0, 1, 5, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0,
        ];

        consensus_config::set(framework_signer, consensus_blob);
    }
}
