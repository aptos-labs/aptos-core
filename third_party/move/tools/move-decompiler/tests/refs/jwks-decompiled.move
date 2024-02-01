module 0x1::jwks {
    struct AllProvidersJWKs has copy, drop, store {
        entries: vector<ProviderJWKs>,
    }
    
    struct JWK has copy, drop, store {
        variant: 0x1::copyable_any::Any,
    }
    
    struct OIDCProvider has drop, store {
        name: vector<u8>,
        config_url: vector<u8>,
    }
    
    struct ObservedJWKs has copy, drop, store, key {
        jwks: AllProvidersJWKs,
    }
    
    struct ObservedJWKsUpdated has drop, store {
        epoch: u64,
        jwks: AllProvidersJWKs,
    }
    
    struct Patch has copy, drop, store {
        variant: 0x1::copyable_any::Any,
    }
    
    struct PatchRemoveAll has copy, drop, store {
        dummy_field: bool,
    }
    
    struct PatchRemoveIssuer has copy, drop, store {
        issuer: vector<u8>,
    }
    
    struct PatchRemoveJWK has copy, drop, store {
        issuer: vector<u8>,
        jwk_id: vector<u8>,
    }
    
    struct PatchUpsertJWK has copy, drop, store {
        issuer: vector<u8>,
        jwk: JWK,
    }
    
    struct PatchedJWKs has drop, key {
        jwks: AllProvidersJWKs,
    }
    
    struct Patches has key {
        patches: vector<Patch>,
    }
    
    struct ProviderJWKs has copy, drop, store {
        issuer: vector<u8>,
        version: u64,
        jwks: vector<JWK>,
    }
    
    struct RSA_JWK has copy, drop, store {
        kid: 0x1::string::String,
        kty: 0x1::string::String,
        alg: 0x1::string::String,
        e: 0x1::string::String,
        n: 0x1::string::String,
    }
    
    struct SupportedOIDCProviders has key {
        providers: vector<OIDCProvider>,
    }
    
    struct UnsupportedJWK has copy, drop, store {
        id: vector<u8>,
        payload: vector<u8>,
    }
    
    fun apply_patch(arg0: &mut AllProvidersJWKs, arg1: Patch) {
        let v0 = *0x1::string::bytes(0x1::copyable_any::type_name(&arg1.variant));
        if (v0 == b"0x1::jwks::PatchRemoveAll") {
            arg0.entries = 0x1::vector::empty<ProviderJWKs>();
        } else {
            if (v0 == b"0x1::jwks::PatchRemoveIssuer") {
                let v1 = 0x1::copyable_any::unpack<PatchRemoveIssuer>(arg1.variant);
                remove_issuer(arg0, v1.issuer);
            } else {
                if (v0 == b"0x1::jwks::PatchRemoveJWK") {
                    let v2 = 0x1::copyable_any::unpack<PatchRemoveJWK>(arg1.variant);
                    let v3 = remove_issuer(arg0, v2.issuer);
                    if (0x1::option::is_some<ProviderJWKs>(&v3)) {
                        let v4 = 0x1::option::extract<ProviderJWKs>(&mut v3);
                        remove_jwk(&mut v4, v2.jwk_id);
                        upsert_provider_jwks(arg0, v4);
                    };
                } else {
                    assert!(v0 == b"0x1::jwks::PatchUpsertJWK", 0x1::error::invalid_argument(3));
                    let v5 = 0x1::copyable_any::unpack<PatchUpsertJWK>(arg1.variant);
                    let v6 = remove_issuer(arg0, v5.issuer);
                    let v7 = if (0x1::option::is_some<ProviderJWKs>(&v6)) {
                        0x1::option::extract<ProviderJWKs>(&mut v6)
                    } else {
                        ProviderJWKs{issuer: v5.issuer, version: 0, jwks: 0x1::vector::empty<JWK>()}
                    };
                    let v8 = v7;
                    upsert_jwk(&mut v8, v5.jwk);
                    upsert_provider_jwks(arg0, v8);
                };
            };
        };
        return
        abort 0x1::error::invalid_argument(3)
    }
    
    fun get_jwk_id(arg0: &JWK) : vector<u8> {
        let v0 = *0x1::string::bytes(0x1::copyable_any::type_name(&arg0.variant));
        if (v0 == b"0x1::jwks::RSA_JWK") {
            let v2 = 0x1::copyable_any::unpack<RSA_JWK>(arg0.variant);
            *0x1::string::bytes(&v2.kid)
        } else {
            assert!(v0 == b"0x1::jwks::UnsupportedJWK", 0x1::error::invalid_argument(4));
            let v3 = 0x1::copyable_any::unpack<UnsupportedJWK>(arg0.variant);
            v3.id
        }
    }
    
    public fun get_patched_jwk(arg0: vector<u8>, arg1: vector<u8>) : JWK acquires PatchedJWKs {
        let v0 = try_get_patched_jwk(arg0, arg1);
        0x1::option::extract<JWK>(&mut v0)
    }
    
    public(friend) fun initialize(arg0: &signer) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = SupportedOIDCProviders{providers: 0x1::vector::empty<OIDCProvider>()};
        move_to<SupportedOIDCProviders>(arg0, v0);
        let v1 = AllProvidersJWKs{entries: 0x1::vector::empty<ProviderJWKs>()};
        let v2 = ObservedJWKs{jwks: v1};
        move_to<ObservedJWKs>(arg0, v2);
        let v3 = Patches{patches: 0x1::vector::empty<Patch>()};
        move_to<Patches>(arg0, v3);
        let v4 = AllProvidersJWKs{entries: 0x1::vector::empty<ProviderJWKs>()};
        let v5 = PatchedJWKs{jwks: v4};
        move_to<PatchedJWKs>(arg0, v5);
    }
    
    public fun new_patch_remove_all() : Patch {
        let v0 = PatchRemoveAll{dummy_field: false};
        Patch{variant: 0x1::copyable_any::pack<PatchRemoveAll>(v0)}
    }
    
    public fun new_patch_remove_issuer(arg0: vector<u8>) : Patch {
        let v0 = PatchRemoveIssuer{issuer: arg0};
        Patch{variant: 0x1::copyable_any::pack<PatchRemoveIssuer>(v0)}
    }
    
    public fun new_patch_remove_jwk(arg0: vector<u8>, arg1: vector<u8>) : Patch {
        let v0 = PatchRemoveJWK{
            issuer : arg0, 
            jwk_id : arg1,
        };
        Patch{variant: 0x1::copyable_any::pack<PatchRemoveJWK>(v0)}
    }
    
    public fun new_patch_upsert_jwk(arg0: vector<u8>, arg1: JWK) : Patch {
        let v0 = PatchUpsertJWK{
            issuer : arg0, 
            jwk    : arg1,
        };
        Patch{variant: 0x1::copyable_any::pack<PatchUpsertJWK>(v0)}
    }
    
    public fun new_rsa_jwk(arg0: 0x1::string::String, arg1: 0x1::string::String, arg2: 0x1::string::String, arg3: 0x1::string::String) : JWK {
        let v0 = RSA_JWK{
            kid : arg0, 
            kty : 0x1::string::utf8(b"RSA"), 
            alg : arg1, 
            e   : arg2, 
            n   : arg3,
        };
        JWK{variant: 0x1::copyable_any::pack<RSA_JWK>(v0)}
    }
    
    public fun new_unsupported_jwk(arg0: vector<u8>, arg1: vector<u8>) : JWK {
        let v0 = UnsupportedJWK{
            id      : arg0, 
            payload : arg1,
        };
        JWK{variant: 0x1::copyable_any::pack<UnsupportedJWK>(v0)}
    }
    
    fun regenerate_patched_jwks() acquires ObservedJWKs, PatchedJWKs, Patches {
        let v0 = borrow_global<ObservedJWKs>(@0x1).jwks;
        let v1 = &borrow_global<Patches>(@0x1).patches;
        let v2 = 0;
        while (v2 < 0x1::vector::length<Patch>(v1)) {
            apply_patch(&mut v0, *0x1::vector::borrow<Patch>(v1, v2));
            v2 = v2 + 1;
        };
        let v3 = PatchedJWKs{jwks: v0};
        *borrow_global_mut<PatchedJWKs>(@0x1) = v3;
    }
    
    fun remove_issuer(arg0: &mut AllProvidersJWKs, arg1: vector<u8>) : 0x1::option::Option<ProviderJWKs> {
        let v0 = &arg0.entries;
        let v1 = false;
        let v2 = 0;
        let v3 = 0;
        while (v3 < 0x1::vector::length<ProviderJWKs>(v0)) {
            if (0x1::vector::borrow<ProviderJWKs>(v0, v3).issuer == arg1) {
                v1 = true;
                v2 = v3;
                break
            };
            v3 = v3 + 1;
        };
        if (v1) {
            0x1::option::some<ProviderJWKs>(0x1::vector::remove<ProviderJWKs>(&mut arg0.entries, v2))
        } else {
            0x1::option::none<ProviderJWKs>()
        }
    }
    
    fun remove_jwk(arg0: &mut ProviderJWKs, arg1: vector<u8>) : 0x1::option::Option<JWK> {
        let v0 = &arg0.jwks;
        let v1 = false;
        let v2 = 0;
        let v3 = 0;
        while (v3 < 0x1::vector::length<JWK>(v0)) {
            if (arg1 == get_jwk_id(0x1::vector::borrow<JWK>(v0, v3))) {
                v1 = true;
                v2 = v3;
                break
            };
            v3 = v3 + 1;
        };
        if (v1) {
            0x1::option::some<JWK>(0x1::vector::remove<JWK>(&mut arg0.jwks, v2))
        } else {
            0x1::option::none<JWK>()
        }
    }
    
    public fun remove_oidc_provider(arg0: &signer, arg1: vector<u8>) : 0x1::option::Option<vector<u8>> acquires SupportedOIDCProviders {
        0x1::system_addresses::assert_aptos_framework(arg0);
        remove_oidc_provider_internal(borrow_global_mut<SupportedOIDCProviders>(@0x1), arg1)
    }
    
    fun remove_oidc_provider_internal(arg0: &mut SupportedOIDCProviders, arg1: vector<u8>) : 0x1::option::Option<vector<u8>> {
        let v0 = &arg0.providers;
        let v1 = false;
        let v2 = 0;
        let v3 = 0;
        while (v3 < 0x1::vector::length<OIDCProvider>(v0)) {
            if (0x1::vector::borrow<OIDCProvider>(v0, v3).name == arg1) {
                v1 = true;
                v2 = v3;
                break
            };
            v3 = v3 + 1;
        };
        if (v1) {
            let v5 = 0x1::vector::swap_remove<OIDCProvider>(&mut arg0.providers, v2);
            0x1::option::some<vector<u8>>(v5.config_url)
        } else {
            0x1::option::none<vector<u8>>()
        }
    }
    
    public fun set_patches(arg0: &signer, arg1: vector<Patch>) acquires ObservedJWKs, PatchedJWKs, Patches {
        0x1::system_addresses::assert_aptos_framework(arg0);
        borrow_global_mut<Patches>(@0x1).patches = arg1;
        regenerate_patched_jwks();
    }
    
    fun try_get_jwk_by_id(arg0: &ProviderJWKs, arg1: vector<u8>) : 0x1::option::Option<JWK> {
        let v0 = &arg0.jwks;
        let v1 = false;
        let v2 = 0;
        let v3 = 0;
        while (v3 < 0x1::vector::length<JWK>(v0)) {
            if (arg1 == get_jwk_id(0x1::vector::borrow<JWK>(v0, v3))) {
                v1 = true;
                v2 = v3;
                break
            };
            v3 = v3 + 1;
        };
        if (v1) {
            0x1::option::some<JWK>(*0x1::vector::borrow<JWK>(&arg0.jwks, v2))
        } else {
            0x1::option::none<JWK>()
        }
    }
    
    fun try_get_jwk_by_issuer(arg0: &AllProvidersJWKs, arg1: vector<u8>, arg2: vector<u8>) : 0x1::option::Option<JWK> {
        let v0 = &arg0.entries;
        let v1 = false;
        let v2 = 0;
        let v3 = 0;
        while (v3 < 0x1::vector::length<ProviderJWKs>(v0)) {
            if (arg1 == 0x1::vector::borrow<ProviderJWKs>(v0, v3).issuer) {
                v1 = true;
                v2 = v3;
                break
            };
            v3 = v3 + 1;
        };
        if (v1) {
            try_get_jwk_by_id(0x1::vector::borrow<ProviderJWKs>(&arg0.entries, v2), arg2)
        } else {
            0x1::option::none<JWK>()
        }
    }
    
    public fun try_get_patched_jwk(arg0: vector<u8>, arg1: vector<u8>) : 0x1::option::Option<JWK> acquires PatchedJWKs {
        try_get_jwk_by_issuer(&borrow_global<PatchedJWKs>(@0x1).jwks, arg0, arg1)
    }
    
    public fun upsert_into_observed_jwks(arg0: &signer, arg1: vector<ProviderJWKs>) acquires ObservedJWKs, PatchedJWKs, Patches {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = borrow_global_mut<ObservedJWKs>(@0x1);
        let v1 = arg1;
        0x1::vector::reverse<ProviderJWKs>(&mut v1);
        let v2 = v1;
        let v3 = 0x1::vector::length<ProviderJWKs>(&v2);
        while (v3 > 0) {
            upsert_provider_jwks(&mut v0.jwks, 0x1::vector::pop_back<ProviderJWKs>(&mut v2));
            v3 = v3 - 1;
        };
        0x1::vector::destroy_empty<ProviderJWKs>(v2);
        let v4 = ObservedJWKsUpdated{
            epoch : 0x1::reconfiguration::current_epoch(), 
            jwks  : v0.jwks,
        };
        0x1::event::emit<ObservedJWKsUpdated>(v4);
        regenerate_patched_jwks();
    }
    
    fun upsert_jwk(arg0: &mut ProviderJWKs, arg1: JWK) : 0x1::option::Option<JWK> {
        let v0 = false;
        let v1 = 0;
        while (v1 < 0x1::vector::length<JWK>(&arg0.jwks)) {
            let v2 = get_jwk_id(0x1::vector::borrow<JWK>(&arg0.jwks, v1));
            let v3 = 0x1::comparator::compare_u8_vector(get_jwk_id(&arg1), v2);
            if (0x1::comparator::is_greater_than(&v3)) {
                v1 = v1 + 1;
            } else {
                v0 = 0x1::comparator::is_equal(&v3);
                break
            };
        };
        if (v0) {
            let v5 = 0x1::vector::borrow_mut<JWK>(&mut arg0.jwks, v1);
            *v5 = arg1;
            0x1::option::some<JWK>(*v5)
        } else {
            0x1::vector::insert<JWK>(&mut arg0.jwks, v1, arg1);
            0x1::option::none<JWK>()
        }
    }
    
    public fun upsert_oidc_provider(arg0: &signer, arg1: vector<u8>, arg2: vector<u8>) : 0x1::option::Option<vector<u8>> acquires SupportedOIDCProviders {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = borrow_global_mut<SupportedOIDCProviders>(@0x1);
        let v1 = OIDCProvider{
            name       : arg1, 
            config_url : arg2,
        };
        0x1::vector::push_back<OIDCProvider>(&mut v0.providers, v1);
        remove_oidc_provider_internal(v0, arg1)
    }
    
    fun upsert_provider_jwks(arg0: &mut AllProvidersJWKs, arg1: ProviderJWKs) : 0x1::option::Option<ProviderJWKs> {
        let v0 = false;
        let v1 = 0;
        while (v1 < 0x1::vector::length<ProviderJWKs>(&arg0.entries)) {
            let v2 = 0x1::vector::borrow<ProviderJWKs>(&arg0.entries, v1).issuer;
            let v3 = 0x1::comparator::compare_u8_vector(arg1.issuer, v2);
            if (0x1::comparator::is_greater_than(&v3)) {
                v1 = v1 + 1;
            } else {
                v0 = 0x1::comparator::is_equal(&v3);
                break
            };
        };
        if (v0) {
            let v5 = 0x1::vector::borrow_mut<ProviderJWKs>(&mut arg0.entries, v1);
            *v5 = arg1;
            0x1::option::some<ProviderJWKs>(*v5)
        } else {
            0x1::vector::insert<ProviderJWKs>(&mut arg0.entries, v1, arg1);
            0x1::option::none<ProviderJWKs>()
        }
    }
    
    // decompiled from Move bytecode v6
}
