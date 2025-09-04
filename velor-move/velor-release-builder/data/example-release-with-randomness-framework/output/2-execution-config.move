// Script hash: e281bacb
// Execution config upgrade proposal

// config: V4(
//     ExecutionConfigV4 {
//         transaction_shuffler_type: Fairness {
//             sender_conflict_window_size: 256,
//             module_conflict_window_size: 2,
//             entry_fun_conflict_window_size: 3,
//         },
//         block_gas_limit_type: ComplexLimitV1 {
//             effective_block_gas_limit: 80001,
//             execution_gas_effective_multiplier: 1,
//             io_gas_effective_multiplier: 1,
//             conflict_penalty_window: 6,
//             use_granular_resource_group_conflicts: false,
//             use_module_publishing_block_conflict: true,
//             block_output_limit: Some(
//                 12582912,
//             ),
//             include_user_txn_size_in_block_output: true,
//             add_block_limit_outcome_onchain: false,
//         },
//         transaction_deduper_type: TxnHashAndAuthenticatorV1,
//     },
// )

script {
    use velor_framework::velor_governance;
    use velor_framework::execution_config;

    fun main(core_resources: &signer) {
        let core_signer = velor_governance::get_signer_testnet_only(core_resources, @0x1);

        let framework_signer = &core_signer;

        let execution_blob: vector<u8> = vector[
            4, 3, 0, 1, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 2, 129, 56, 1, 0, 0,
            0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 6,
            0, 0, 0, 0, 1, 1, 0, 0, 192, 0, 0, 0, 0, 0, 1, 0, 1,
        ];

        execution_config::set_for_next_epoch(framework_signer, execution_blob);
        velor_governance::reconfigure(framework_signer);
    }
}
