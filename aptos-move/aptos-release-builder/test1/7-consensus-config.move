// Consensus config upgrade proposal

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
