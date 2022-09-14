script {
    use aptos_framework::aptos_governance;
    use aptos_framework::version;

    fun main(core_resources: &signer) {
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);

        let framework_signer = &core_signer;

        version::set_version(framework_signer, 4);
    }
}
