script {
    use aptos_framework::keyless_account;
    use aptos_framework::aptos_governance;
    use std::option;
    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        keyless_account::update_training_wheels_for_next_epoch(&framework_signer, option::some(x"FF"));
        // Action: obtain the values below from vk file. https://github.com/aptos-labs/snarkjs-to-aptos may help.
        let alpha_g1 = x"";
        let beta_g2 = x"";
        let gamma_g2 = x"";
        let delta_g2 = x"";
        let gamma_abc_g1 = vector[x""];
        let vk = keyless_account::new_groth16_verification_key(alpha_g1, beta_g2, gamma_g2, delta_g2, gamma_abc_g1);
        keyless::set_groth16_verification_key_for_next_epoch(&framework_signer, vk);
        aptos_governance::reconfigure(&framework_signer);
    }
}
