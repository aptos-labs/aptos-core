spec aptos_framework::jwks {
    spec on_new_epoch(framework: &signer) {
        requires @aptos_framework == std::signer::address_of(framework);
        include config_buffer::OnNewEpochRequirement<SupportedOIDCProviders>;
        aborts_if false;
    }

    spec patch_federated_jwks(jwk_owner: &signer, patches: vector<Patch>) {
        pragma verify_duration_estimate = 80;
    }

    spec update_federated_jwk_set(jwk_owner: &signer, iss: vector<u8>, kid_vec: vector<String>, alg_vec: vector<String>, e_vec: vector<String>, n_vec: vector<String>) {
        pragma verify_duration_estimate = 80;
    }

    spec get_patched_jwk(issuer: vector<u8>, jwk_id: vector<u8>): JWK {
        pragma verify_duration_estimate = 80;
    }

    spec upsert_into_observed_jwks(fx: &signer, provider_jwks_vec: vector<ProviderJWKs>)  {
        pragma verify_duration_estimate = 80;
    }

    spec regenerate_patched_jwks() {
        pragma verify_duration_estimate = 80;
    }

    spec try_get_jwk_by_issuer(jwks: &AllProvidersJWKs, issuer: vector<u8>, jwk_id: vector<u8>): Option<JWK> {
        pragma verify_duration_estimate = 80;
    }

    spec remove_jwk(jwks: &mut ProviderJWKs, jwk_id: vector<u8>): Option<JWK> {
        pragma verify_duration_estimate = 80;
    }

    spec apply_patch(jwks: &mut AllProvidersJWKs, patch: Patch) {
        pragma verify_duration_estimate = 80;
    }

    spec try_get_patched_jwk(issuer: vector<u8>, jwk_id: vector<u8>): Option<JWK> {
        pragma verify_duration_estimate = 80;
    }

    spec set_patches(fx: &signer, patches: vector<Patch>) {
        pragma verify_duration_estimate = 80;
    }

    spec remove_issuer_from_observed_jwks(fx: &signer, issuer: vector<u8>): Option<ProviderJWKs> {
        pragma verify_duration_estimate = 80;
    }

    spec try_get_jwk_by_id(provider_jwks: &ProviderJWKs, jwk_id: vector<u8>): Option<JWK> {
        pragma verify_duration_estimate = 80;
    }

    spec remove_issuer(jwks: &mut AllProvidersJWKs, issuer: vector<u8>): Option<ProviderJWKs> {
        use std::option;
        use std::vector;
        pragma opaque;
        ensures option::spec_is_none(result) <==> (forall jwk: ProviderJWKs where vector::spec_contains(old(jwks).entries, jwk): jwk.issuer != issuer);
        ensures option::spec_is_none(result) ==> old(jwks) == jwks;
        ensures option::spec_is_some(result) ==> vector::spec_contains(old(jwks).entries, option::spec_borrow(result));
    }

}
