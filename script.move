script {
    use aptos_framework::keyless_account;
    use aptos_framework::aptos_governance;
    use std::option;
    use std::vector;
    use std::string::utf8;
    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);

        let new_vk = keyless_account::new_groth16_verification_key(
            x"e2f26dbea299f5223b646cb1fb33eadb059d9407559d7441dfd902e3a79a4d2d",
            x"abb73dc17fbc13021e2471e0c08bd67d8401f52b73d6d07483794cad4778180e0c06f33bbc4c79a9cadef253a68084d382f17788f885c9afd176f7cb2f036789",
            x"cdd997c4f8542deec11a01291180bf60face4fc2229d9bb0ecfcd026f7a367181fb18b26143cbfdd9ab65ed42a991b48916fa105c537c8e7d9cd6c4a844cff93",
            x"edf692d95cbdde46ddda5ef7d422436779445c5e66006a42761e1f12efde0018c212f3aeb785e49712e7a9353349aaf1255dfb31b7bf60723a480d9293938e19",
            vector[
                x"6561a29b3543bb9f976bd6ebc5704507391c3c0291b660b5ac47d6ddfa4b262c",
                x"469f2fcf618dd6c774283eec69e9cfa3bed955b9729b78ecbf68616202e996a1",
            ],
        );
        keyless_account::update_groth16_verification_key(&framework_signer, new_vk);
    }
}
