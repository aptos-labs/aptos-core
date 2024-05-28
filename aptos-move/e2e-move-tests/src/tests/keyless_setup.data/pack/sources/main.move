script {
    use aptos_framework::jwks;
    use aptos_framework::aptos_governance;
    use aptos_framework::keyless_account;
    use std::string::utf8;

    fun main(core_resources: &signer, iss: vector<u8>, kid: vector<u8>, alg: vector<u8>, e: vector<u8>, n: vector<u8>, max_exp_horizon_secs: u64) {
        let fx = aptos_governance::get_signer_testnet_only(core_resources, @aptos_framework);
        let jwk = jwks::new_rsa_jwk(
            utf8(kid),
            utf8(alg),
            utf8(e),
            utf8(n)
        );

        let patches = vector[
            jwks::new_patch_remove_all(),
            jwks::new_patch_upsert_jwk(iss, jwk),
        ];
        jwks::set_patches(&fx, patches);

        keyless_account::update_max_exp_horizon_for_next_epoch(&fx, max_exp_horizon_secs);
        // sets the pending Configuration change to the max expiration horizon from above
        aptos_governance::force_end_epoch_test_only(core_resources);
    }
}
