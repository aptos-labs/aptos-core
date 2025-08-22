script {
    use aptos_framework::aptos_governance;
    use aptos_framework::consensus_config;
    use std::vector;

    fun main(core_resources: &signer) {
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        let framework_signer = &core_signer;

        let consensus_blob: vector<u8> = x"0302010a0000000000000064000000000000000201e8030000000000000a0000000000000001000000000000000a0000000a00000000000000010000000000000001050000000a000000000000000101010200000000000000000020000000000000";

        consensus_config::set_for_next_epoch(framework_signer, consensus_blob);
        aptos_governance::reconfigure(framework_signer);
    }
}