script {
    use aptos_framework::aptos_governance;
    use aptos_framework::chunky_dkg_config;
    use aptos_std::fixed_point64;

    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        let config = chunky_dkg_config::new_v1(
            fixed_point64::create_from_rational(1, 2),
            fixed_point64::create_from_rational(2, 3)
        );
        chunky_dkg_config::set_for_next_epoch(&framework_signer, config);
        aptos_governance::reconfigure(&framework_signer);
    }
}
