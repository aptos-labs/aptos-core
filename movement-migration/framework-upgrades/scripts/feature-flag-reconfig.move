script {
    use aptos_framework::aptos_governance;
    use std::features;

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
            80, // NativeMemoryOperation
            223, // new GGP
        ];

        let disabled_blob: vector<u64> = vector[
            48, // RemoveDetailedError
            16, // PeriodicalRewardRateReduction
            46, // KeylessAccouns
            47, // KeylessButZklessAccounts
            54, // KeylessAccountsWithPasskeys
            71, // AtomicBridge
            72, // NativeBridge
            73, // Old GGP
        ];

        features::change_feature_flags_for_next_epoch(&core_signer, enabled_blob, disabled_blob);
    }
}
