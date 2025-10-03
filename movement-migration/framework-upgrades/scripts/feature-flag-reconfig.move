script {
    use aptos_framework::aptos_governance;
    use aptos_framework::signer;
    use std::features;
    use std::vector;

    fun main(core_resources: &signer) {
        let core_signer = aptos_governance::get_signer_testnet_only(
            core_resources,
            @0000000000000000000000000000000000000000000000000000000000000001
        );
        //let core_address: address = signer::address_of(core_resources);

        let enabled_blob: vector<u64> = vector[
            58, // RejectUnstableBytecode
            67, // ConcurrentFungibleBalance
            40, // VMBinaryFormat7
            74, // EnumTypes
        ];

        let disabled_blob: vector<u64> = vector[
            48, // RemoveDetailedError
            16, // PeriodicalRewardRateReduction
            46, // KeylessAccouns
            47, // KeylessButZklessAccounts
            54, // KeylessAccountsWithPasskeys
            71, // AtomicBridge
            72, // NativeBridge
            73, // GovernedGasPool
        ];

        features::change_feature_flags(&core_signer, enabled_blob, disabled_blob);
    }
}
