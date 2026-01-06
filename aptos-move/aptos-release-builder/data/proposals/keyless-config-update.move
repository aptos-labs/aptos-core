
script {
    use aptos_framework::keyless_account;
    use aptos_framework::aptos_governance;
    use std::option;
    fun main(proposal_id: u64) {
        let framework_signer = aptos_governance::resolve_multi_step_proposal(proposal_id, @0x1, {{ script_hash }},);

        let alpha_g1 = x"e2f26dbea299f5223b646cb1fb33eadb059d9407559d7441dfd902e3a79a4d2d";
        let beta_g2 = x"abb73dc17fbc13021e2471e0c08bd67d8401f52b73d6d07483794cad4778180e0c06f33bbc4c79a9cadef253a68084d382f17788f885c9afd176f7cb2f036789";
        let gamma_g2 = x"edf692d95cbdde46ddda5ef7d422436779445c5e66006a42761e1f12efde0018c212f3aeb785e49712e7a9353349aaf1255dfb31b7bf60723a480d9293938e19";
        let delta_g2 = x"e65b1be749c3cc85f853dea2663f24fe1b38fc76d954d6fa44853e671dcc8c090b8195e0f864d126a8833df648f13b9a7a0ac4aefdb15a11f3e553f72b5f5e81";
        let gamma_abc_g1 = vector[
            x"2c2b6d282eaae41af93ef0856dc51f6f6ffe46993e6178234c873c0974920e2f",
            x"520fa3bc196582b8f51f69e608f74a15578402174fcd6bbad8b09bbe440d680f",
        ];
        let vk = keyless_account::new_groth16_verification_key(alpha_g1, beta_g2, gamma_g2, delta_g2, gamma_abc_g1);
        keyless_account::set_groth16_verification_key_for_next_epoch(&framework_signer, vk);
        let pk_bytes = x"d8a906db9c5ddedb9d2e5833bd3f25380ae4db60aacbf2c7d0726c2b3992fe83";
        keyless_account::update_training_wheels_for_next_epoch(&framework_signer, option::some(pk_bytes));
        aptos_governance::reconfigure(&framework_signer);
    }
}
