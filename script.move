script {
    use aptos_framework::keyless_account;
    use aptos_framework::aptos_governance;
    use std::option;
    use std::vector;
    use std::string::utf8;
    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);

        let new_vk = keyless_account::new_groth16_verification_key(
            x"6d1c152d2705e35fe7a07a66eb8a10a7f42f1e38c412fbbc3ac7f9affc25dc24",
            x"e20a834c55ae6e2fcbd66636e09322727f317aff8957dd342afa11f936ef7c02cfdc8c9862849a0442bcaa4e03f45343e8bf261ef4ab58cead2efc17100a3b16",
            x"edf692d95cbdde46ddda5ef7d422436779445c5e66006a42761e1f12efde0018c212f3aeb785e49712e7a9353349aaf1255dfb31b7bf60723a480d9293938e19",
            x"edf692d95cbdde46ddda5ef7d422436779445c5e66006a42761e1f12efde0018c212f3aeb785e49712e7a9353349aaf1255dfb31b7bf60723a480d9293938e19",
            vector[
                x"9aae6580d6040e77969d70e748e861664228e3567e77aa99822f8a4a19c29101",
                x"e38ad8b845e3ef599232b43af2a64a73ada04d5f0e73f1848e6631e17a247415",
            ],
        );
        keyless_account::update_groth16_verification_key(&framework_signer, new_vk);
    }
}
