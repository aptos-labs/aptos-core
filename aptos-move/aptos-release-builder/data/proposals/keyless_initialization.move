// Initialize Groth16 verification key and misc config for keyless before enabling.
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::keyless_account;
    use std::option;
    use std::string::utf8;

    fun main(proposal_id: u64) {
        let framework = aptos_governance::resolve_multi_step_proposal(
            proposal_id,
            @0x1,
            {{ script_hash }},
        );

        // Use the verification key from https://github.com/aptos-labs/aptos-keyless-trusted-setup-contributions-may-2024.
        let alpha_g1 = x"e2f26dbea299f5223b646cb1fb33eadb059d9407559d7441dfd902e3a79a4d2d";
        let beta_g2 = x"abb73dc17fbc13021e2471e0c08bd67d8401f52b73d6d07483794cad4778180e0c06f33bbc4c79a9cadef253a68084d382f17788f885c9afd176f7cb2f036789";
        let gamma_g2 = x"edf692d95cbdde46ddda5ef7d422436779445c5e66006a42761e1f12efde0018c212f3aeb785e49712e7a9353349aaf1255dfb31b7bf60723a480d9293938e19";
        let delta_g2 = x"6176de7d77e614e09ef5e8e19cbf785ffed405d6531cee13cd71a46e2b4ef30deb18f6976c172bdcd7ea8ab2b509991bb5ce34f9fbb42486b78aac62a894a480";
        let gamma_abc_g1 = vector[
            x"7e92d0c6818f2e51248cd1e8e82eb14521d990b0bb155ab0e3cf99b888bc5387",
            x"be1ad9f5fec081770956f846e1d0ea97219a3f6499acc33e1a67aef6d6e16898",
        ];
        let vk = keyless_account::new_groth16_verification_key(alpha_g1, beta_g2, gamma_g2, delta_g2, gamma_abc_g1);

        // Prepare misc configs.
        let override_aud_val = vector[utf8(b"test.recovery.aud")];
        let max_signatures_per_txn = 3;
        let max_exp_horizon_secs = 10_000_000;
        let training_wheels_pubkey = option::some(b"");
        let max_commited_epk_bytes = 93;
        let max_iss_val_bytes = 120;
        let max_extra_field_bytes = 350;
        let max_jwt_header_b64_bytes = 300;
        let config = keyless_account::new_configuration(
            override_aud_val,
            max_signatures_per_txn,
            max_exp_horizon_secs,
            training_wheels_pubkey,
            max_commited_epk_bytes,
            max_iss_val_bytes,
            max_extra_field_bytes,
            max_jwt_header_b64_bytes,
        );
        keyless_account::set_groth16_verification_key_for_next_epoch(&framework, vk);
        keyless_account::set_configuration_for_next_epoch(&framework, config);
        aptos_governance::reconfigure(&framework);
    }
}
