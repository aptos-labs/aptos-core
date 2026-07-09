spec aptos_framework::jwks {
    spec remove_oidc_provider_internal(provider_set: &mut SupportedOIDCProviders, name: vector<u8>): Option<vector<u8>> {
        pragma opaque;
        aborts_if false;
    }

    spec on_new_epoch(framework: &signer) {
        requires @aptos_framework == std::signer::address_of(framework);
        include config_buffer::OnNewEpochRequirement<SupportedOIDCProviders>;
        aborts_if false;
    }

    spec initialize(fx: &signer) {
        pragma opaque;
        aborts_if std::signer::address_of(fx) != @aptos_framework;
        aborts_if exists<SupportedOIDCProviders>(@aptos_framework);
        aborts_if exists<ObservedJWKs>(@aptos_framework);
        aborts_if exists<Patches>(@aptos_framework);
        aborts_if exists<PatchedJWKs>(@aptos_framework);
        modifies global<SupportedOIDCProviders>(@aptos_framework);
        modifies global<ObservedJWKs>(@aptos_framework);
        modifies global<Patches>(@aptos_framework);
        modifies global<PatchedJWKs>(@aptos_framework);
        ensures exists<SupportedOIDCProviders>(@aptos_framework);
        ensures exists<ObservedJWKs>(@aptos_framework);
        ensures exists<Patches>(@aptos_framework);
        ensures exists<PatchedJWKs>(@aptos_framework);
    }

    spec initialize_with_defaults(fx: &signer, providers: vector<OIDCProvider>, patches: vector<Patch>) {
        pragma verify = false;
    }

    spec upsert_oidc_provider(fx: &signer, name: vector<u8>, config_url: vector<u8>): Option<vector<u8>> {
        use aptos_framework::chain_status;
        pragma opaque;
        pragma aborts_if_is_partial;
        aborts_if std::signer::address_of(fx) != @aptos_framework;
        aborts_if !chain_status::is_genesis();
        aborts_if !exists<SupportedOIDCProviders>(@aptos_framework);
        modifies global<SupportedOIDCProviders>(@aptos_framework);
    }

    spec remove_oidc_provider(fx: &signer, name: vector<u8>): Option<vector<u8>> {
        use aptos_framework::chain_status;
        pragma opaque;
        pragma aborts_if_is_partial;
        aborts_if std::signer::address_of(fx) != @aptos_framework;
        aborts_if !chain_status::is_genesis();
        aborts_if !exists<SupportedOIDCProviders>(@aptos_framework);
        modifies global<SupportedOIDCProviders>(@aptos_framework);
    }

    spec upsert_oidc_provider_for_next_epoch(fx: &signer, name: vector<u8>, config_url: vector<u8>): Option<vector<u8>> {
        pragma opaque;
        pragma aborts_if_is_partial;
        aborts_if std::signer::address_of(fx) != @aptos_framework;
        modifies global<config_buffer::PendingConfigs>(@aptos_framework);
    }

    spec remove_oidc_provider_for_next_epoch(fx: &signer, name: vector<u8>): Option<vector<u8>> {
        pragma opaque;
        pragma aborts_if_is_partial;
        aborts_if std::signer::address_of(fx) != @aptos_framework;
        modifies global<config_buffer::PendingConfigs>(@aptos_framework);
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
        ensures option::is_none(result) <==> (forall jwk: ProviderJWKs where vector::spec_contains(old(jwks).entries, jwk): jwk.issuer != issuer);
        ensures option::is_none(result) ==> old(jwks) == jwks;
        ensures option::is_some(result) ==> vector::spec_contains(old(jwks).entries, option::borrow(result));
    }

}
