script {
    use aptos_framework::jwks;
    use std::string::utf8;

    fun main(jwk_owner: &signer, iss: vector<u8>, kid: vector<u8>, alg: vector<u8>, e: vector<u8>, n: vector<u8>) {
        let jwk = jwks::new_rsa_jwk(
            utf8(kid),
            utf8(alg),
            utf8(e),
            utf8(n)
        );

        let patches = vector[
            jwks::new_patch_upsert_jwk(iss, jwk),
        ];
        jwks::patch_federated_jwks(jwk_owner, patches);
    }
}
