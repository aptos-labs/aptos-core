spec aptos_framework::jwks {
    spec remove_oidc_provider_internal(provider_set: &mut SupportedOIDCProviders, name: vector<u8>): Option<vector<u8>> {
        pragma opaque;
        aborts_if false;
    }

    spec on_new_epoch(framework: &signer) {
        pragma opaque;
        include config_buffer::OnNewEpochApply<SupportedOIDCProviders>;
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
        // Aborts from applying `patches` are not specified.
        pragma aborts_if_is_partial;
        aborts_if std::signer::address_of(fx) != @aptos_framework;
        aborts_if exists<SupportedOIDCProviders>(@aptos_framework);
        aborts_if exists<ObservedJWKs>(@aptos_framework);
        aborts_if exists<Patches>(@aptos_framework);
        aborts_if exists<PatchedJWKs>(@aptos_framework);
        ensures global<SupportedOIDCProviders>(@aptos_framework).providers == providers;
        ensures global<ObservedJWKs>(@aptos_framework).jwks.entries == vector[];
        ensures global<Patches>(@aptos_framework).patches == patches;
        ensures exists<PatchedJWKs>(@aptos_framework);
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
        // Partial: the aborts of applying the patches and of the size check are not specified.
        pragma aborts_if_is_partial;
        let addr = std::signer::address_of(jwk_owner);
        aborts_if addr == @aptos_framework;
        ensures exists<FederatedJWKs>(addr);
    }

    spec update_federated_jwk_set(jwk_owner: &signer, iss: vector<u8>, kid_vec: vector<String>, alg_vec: vector<String>, e_vec: vector<String>, n_vec: vector<String>) {
        // Partial: the aborts of applying the patches and of the size check are not specified.
        pragma aborts_if_is_partial;
        let addr = std::signer::address_of(jwk_owner);
        aborts_if len(kid_vec) == 0;
        aborts_if len(alg_vec) != len(kid_vec);
        aborts_if len(e_vec) != len(kid_vec);
        aborts_if len(n_vec) != len(kid_vec);
        aborts_if addr == @aptos_framework;
        ensures exists<FederatedJWKs>(addr);
    }

    spec get_patched_jwk(issuer: vector<u8>, jwk_id: vector<u8>): JWK {
        // Partial: the aborts of a missing JWK and of decoding JWK ids are not specified.
        pragma aborts_if_is_partial;
        aborts_if !exists<PatchedJWKs>(@aptos_framework);
    }

    spec upsert_into_observed_jwks(fx: &signer, provider_jwks_vec: vector<ProviderJWKs>)  {
        // Partial: the aborts of the version checks and of regenerating the patched JWKs are not specified.
        pragma aborts_if_is_partial;
        aborts_if std::signer::address_of(fx) != @aptos_framework;
        aborts_if !exists<ObservedJWKs>(@aptos_framework);
    }

    spec regenerate_patched_jwks() {
        // Partial: the aborts of applying the patches are not specified.
        pragma aborts_if_is_partial;
        aborts_if !exists<ObservedJWKs>(@aptos_framework);
        aborts_if !exists<Patches>(@aptos_framework);
        aborts_if !exists<PatchedJWKs>(@aptos_framework);
    }

    spec try_get_jwk_by_issuer(jwks: &AllProvidersJWKs, issuer: vector<u8>, jwk_id: vector<u8>): Option<JWK> {
    }

    /// Either the returned JWK is removed or nothing changes; issuer and version are unchanged.
    spec remove_jwk(jwks: &mut ProviderJWKs, jwk_id: vector<u8>): Option<JWK> {
        use std::option;
        use std::vector;
        // Partial: the aborts of decoding JWK ids are not specified.
        pragma aborts_if_is_partial;
        ensures jwks.issuer == old(jwks).issuer;
        ensures jwks.version == old(jwks).version;
        ensures option::is_none(result) ==> jwks == old(jwks);
        ensures option::is_none(result) ==>
            (forall j in 0..len(old(jwks).jwks): spec_jwk_id(old(jwks).jwks[j]) != jwk_id);
        ensures option::is_some(result) ==> len(jwks.jwks) + 1 == len(old(jwks).jwks)
            && vector::spec_contains(old(jwks).jwks, option::borrow(result));
        // The first JWK with the requested id is removed; the others keep their order.
        ensures option::is_some(result) ==> spec_jwk_id(option::borrow(result)) == jwk_id;
        ensures option::is_some(result) ==> (exists i in 0..len(old(jwks).jwks):
            old(jwks).jwks[i] == option::borrow(result)
                && (forall j in 0..i: spec_jwk_id(old(jwks).jwks[j]) != jwk_id)
                && jwks.jwks == concat(old(jwks).jwks[0..i], old(jwks).jwks[i + 1..len(old(jwks).jwks)]));
    }

    /// The id that `get_jwk_id` returns for a JWK of a known variant.
    spec fun spec_jwk_id(jwk: JWK): vector<u8> {
        if (jwk.variant.type_name.bytes == b"0x1::jwks::RSA_JWK") {
            aptos_std::from_bcs::deserialize<RSA_JWK>(jwk.variant.data).kid.bytes
        } else {
            aptos_std::from_bcs::deserialize<UnsupportedJWK>(jwk.variant.data).id
        }
    }

    spec apply_patch(jwks: &mut AllProvidersJWKs, patch: Patch) {
    }

    spec try_get_patched_jwk(issuer: vector<u8>, jwk_id: vector<u8>): Option<JWK> {
        // Partial: the aborts of decoding JWK ids are not specified.
        pragma aborts_if_is_partial;
        aborts_if !exists<PatchedJWKs>(@aptos_framework);
    }

    spec set_patches(fx: &signer, patches: vector<Patch>) {
        // Partial: the aborts of applying the patches are not specified.
        pragma aborts_if_is_partial;
        aborts_if std::signer::address_of(fx) != @aptos_framework;
        aborts_if !exists<Patches>(@aptos_framework);
        aborts_if !exists<ObservedJWKs>(@aptos_framework);
        aborts_if !exists<PatchedJWKs>(@aptos_framework);
        ensures global<Patches>(@aptos_framework).patches == patches;
    }

    spec remove_issuer_from_observed_jwks(fx: &signer, issuer: vector<u8>): Option<ProviderJWKs> {
        use std::option;
        use std::vector;
        // Partial: the aborts of regenerating the patched JWKs are not specified.
        pragma aborts_if_is_partial;
        aborts_if std::signer::address_of(fx) != @aptos_framework;
        aborts_if !exists<ObservedJWKs>(@aptos_framework);
        ensures option::is_some(result) ==> vector::spec_contains(
            old(global<ObservedJWKs>(@aptos_framework)).jwks.entries, option::borrow(result));
    }


    spec try_get_jwk_by_id(provider_jwks: &ProviderJWKs, jwk_id: vector<u8>): Option<JWK> {
        use std::option;
        use std::vector;
        // Partial: the aborts of decoding JWK ids are not specified.
        pragma aborts_if_is_partial;
        ensures option::is_some(result) ==> vector::spec_contains(provider_jwks.jwks, option::borrow(result));
    }

    spec get_jwk_id(jwk: &JWK): vector<u8> {
        use aptos_std::from_bcs;
        let name = jwk.variant.type_name.bytes;
        let data = jwk.variant.data;
        aborts_if name != b"0x1::jwks::RSA_JWK" && name != b"0x1::jwks::UnsupportedJWK"
            with error::INVALID_ARGUMENT;
        aborts_if name == b"0x1::jwks::RSA_JWK" && !from_bcs::deserializable<RSA_JWK>(data);
        aborts_if name == b"0x1::jwks::UnsupportedJWK" && !from_bcs::deserializable<UnsupportedJWK>(data);
        ensures name == b"0x1::jwks::RSA_JWK" ==> result == from_bcs::deserialize<RSA_JWK>(data).kid.bytes;
        ensures name == b"0x1::jwks::UnsupportedJWK" ==> result == from_bcs::deserialize<UnsupportedJWK>(data).id;
    }

    /// Either an existing entry is replaced and returned, or a new entry is inserted.
    spec upsert_provider_jwks(jwks: &mut AllProvidersJWKs, provider_jwks: ProviderJWKs): Option<ProviderJWKs> {
        use std::option;
        use std::vector;
        aborts_if false;
        ensures option::is_none(result) ==> len(jwks.entries) == len(old(jwks).entries) + 1;
        ensures option::is_some(result) ==> len(jwks.entries) == len(old(jwks).entries)
            && vector::spec_contains(old(jwks).entries, option::borrow(result));
        // The entry is inserted, or replaces the one with the same issuer, after all entries with a smaller issuer.
        // An insertion lands at the end or before an entry with a greater issuer.
        ensures option::is_none(result) ==> (exists i in 0..len(old(jwks).entries) + 1:
            spec_issuers_below(old(jwks).entries, provider_jwks.issuer, i)
                && (i == len(old(jwks).entries)
                    || aptos_std::comparator::spec_compare_u8_vector(
                        provider_jwks.issuer, old(jwks).entries[i].issuer
                    ).is_smaller_than())
                && jwks.entries == concat(
                    old(jwks).entries[0..i],
                    concat(vec(provider_jwks), old(jwks).entries[i..len(old(jwks).entries)])
                ));
        ensures option::is_some(result) ==> (exists i in 0..len(old(jwks).entries):
            old(jwks).entries[i] == option::borrow(result)
                && old(jwks).entries[i].issuer == provider_jwks.issuer
                && spec_issuers_below(old(jwks).entries, provider_jwks.issuer, i)
                && jwks.entries == update(old(jwks).entries, i, provider_jwks));
    }

    /// The issuers of `entries[0..i]` all compare less than `issuer`.
    spec fun spec_issuers_below(entries: vector<ProviderJWKs>, issuer: vector<u8>, i: u64): bool {
        forall j in 0..i: aptos_std::comparator::spec_compare_u8_vector(issuer, entries[j].issuer).is_greater_than()
    }

    /// Either an existing JWK is replaced and returned, or a new JWK is inserted; issuer and version are unchanged.
    spec upsert_jwk(set: &mut ProviderJWKs, jwk: JWK): Option<JWK> {
        use std::option;
        use std::vector;
        // Aborts are not specified: the search loop has no invariant bounding the index.
        pragma aborts_if_is_partial;
        ensures set.issuer == old(set).issuer;
        ensures set.version == old(set).version;
        ensures option::is_none(result) ==> len(set.jwks) == len(old(set).jwks) + 1;
        ensures option::is_some(result) ==> len(set.jwks) == len(old(set).jwks)
            && vector::spec_contains(old(set).jwks, option::borrow(result));
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
