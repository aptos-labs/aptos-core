script {
    use aptos_framework::aptos_governance;
    use aptos_framework::block;

    fun main(core_resources: &signer) {
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
        block::update_epoch_interval_microsecs(&core_signer, 200_000_000); // 200 secs
    }
}
