script {
    use aptos_framework::keyless_account;
    use aptos_framework::aptos_governance;
    use std::option;
    use std::vector;
    use std::string::utf8;
    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);

        let new_config = keyless_account::new_configuration(
                vector[utf8(b"test.recovery.aud")],
                3,
                10000000, // ~1160 days
                option::some(x"c9c9c08c2e3fdbf0c818274a34a943263eebd7c6683e8b37b61f21f62af4dea1"),
                3 * 31,
                120,
                350,
                300,
            );
        keyless_account::update_configuration(&framework_signer, new_config);
    }
}
