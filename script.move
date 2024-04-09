script {
    use aptos_framework::keyless_account;
    use aptos_framework::aptos_governance;
    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);

        let new_vk = keyless_account::new_groth16_verification_key(
            x"e2f26dbea299f5223b646cb1fb33eadb059d9407559d7441dfd902e3a79a4d2d",
            x"abb73dc17fbc13021e2471e0c08bd67d8401f52b73d6d07483794cad4778180e0c06f33bbc4c79a9cadef253a68084d382f17788f885c9afd176f7cb2f036789",
            x"edf692d95cbdde46ddda5ef7d422436779445c5e66006a42761e1f12efde0018c212f3aeb785e49712e7a9353349aaf1255dfb31b7bf60723a480d9293938e19",
            x"edf692d95cbdde46ddda5ef7d422436779445c5e66006a42761e1f12efde0018c212f3aeb785e49712e7a9353349aaf1255dfb31b7bf60723a480d9293938e19",
            vector[
                x"b72262d85f8026d978cc6def7624fa0558ff1209a2dab44c5fa7092f04b3af2b",
                x"058f1d4a9de6065b522a789948750b654a80f105047afff3a31f5d7dda2a59a8",
            ],
        );
        keyless_account::update_groth16_verification_key(&framework_signer, new_vk);
    }

}
