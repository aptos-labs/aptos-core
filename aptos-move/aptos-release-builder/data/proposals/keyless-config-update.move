
script {
    use aptos_framework::keyless_account;
    use aptos_framework::aptos_governance;
    use std::option;
    fun main(proposal_id: u64) {
        let framework_signer = aptos_governance::resolve_multi_step_proposal(proposal_id, @0x1, {{ script_hash }},);

        let alpha_g1 = x"e2f26dbea299f5223b646cb1fb33eadb059d9407559d7441dfd902e3a79a4d2d";
        let beta_g2 = x"abb73dc17fbc13021e2471e0c08bd67d8401f52b73d6d07483794cad4778180e0c06f33bbc4c79a9cadef253a68084d382f17788f885c9afd176f7cb2f036789";
        let gamma_g2 = x"edf692d95cbdde46ddda5ef7d422436779445c5e66006a42761e1f12efde0018c212f3aeb785e49712e7a9353349aaf1255dfb31b7bf60723a480d9293938e19";
        let delta_g2 = x"b106619932d0ef372c46909a2492e246d5de739aa140e27f2c71c0470662f125219049cfe15e4d140d7e4bb911284aad1cad19880efb86f2d9dd4b1bb344ef8f";
        let gamma_abc_g1 = vector[
            x"6123b6fea40de2a7e3595f9c35210da8a45a7e8c2f7da9eb4548e9210cfea81a",
            x"32a9b8347c512483812ee922dc75952842f8f3083edb6fe8d5c3c07e1340b683",
        ];
        let vk = keyless_account::new_groth16_verification_key(alpha_g1, beta_g2, gamma_g2, delta_g2, gamma_abc_g1);
        keyless_account::set_groth16_verification_key_for_next_epoch(&framework_signer, vk);
        let pk_bytes = x"1388de358cf4701696bd58ed4b96e9d670cbbb914b888be1ceda6374a3098ed4";
        keyless_account::update_training_wheels_for_next_epoch(&framework_signer, option::some(pk_bytes));
        aptos_governance::reconfigure(&framework_signer);
    }
}
