/// JWK functions and structs.
module aptos_framework::jwks {
    use std::error;
    use std::error::invalid_argument;
    use std::option;
    use std::option::Option;
    use std::string;
    use std::string::{String, utf8};
    use std::vector;
    use aptos_std::comparator::{compare_u8_vector, is_greater_than, is_equal};
    use aptos_std::copyable_any;
    use aptos_std::copyable_any::{Any, pack};
    use aptos_framework::event::emit;
    use aptos_framework::reconfiguration;
    use aptos_framework::system_addresses;
    #[test_only]
    use aptos_framework::account::create_account_for_test;

    friend aptos_framework::genesis;

    const EUNEXPECTED_EPOCH: u64 = 1;
    const EUNEXPECTED_VERSION: u64 = 2;
    const EUNKNOWN_JWKPATCH_VARIANT: u64 = 3;
    const EUNKNOWN_JWK_VARIANT: u64 = 4;
    const EISSUER_NOT_FOUND: u64 = 5;
    const EJWK_ID_NOT_FOUND: u64 = 6;

    /// An OIDC provider.
    struct OIDCProvider has drop, store {
        /// The utf-8 encoded issuer string. E.g., b"https://www.facebook.com".
        name: vector<u8>,

        /// The ut8-8 encoded OpenID configuration URL of the provider.
        /// E.g., b"https://www.facebook.com/.well-known/openid-configuration/".
        config_url: vector<u8>,
    }

    /// A list of OIDC providers whose JWKs should be watched by validators. Maintained by governance proposals.
    struct SupportedOIDCProviders has key {
        providers: vector<OIDCProvider>,
    }

    /// An JWK variant that represents the JWKs which were observed but not yet supported by Aptos.
    /// Observing `UnsupportedJWK`s means the providers adopted a new key type/format, and the system should be updated.
    struct UnsupportedJWK has copy, drop, store {
        id: vector<u8>,
        payload: vector<u8>,
    }

    /// A JWK variant where `kty` is `RSA`.
    struct RSA_JWK has copy, drop, store {
        kid: String,
        kty: String,
        alg: String,
        e: String,
        n: String,
    }

    /// A JSON web key.
    struct JWK has copy, drop, store {
        /// A `JWK` variant packed as an `Any`.
        /// Currently the variant type is one of the following.
        /// - `RSA_JWK`
        /// - `UnsupportedJWK`
        variant: Any,
    }

    /// A provider and its `JWK`s.
    struct ProviderJWKs has copy, drop, store {
        /// The utf-8 encoding of the issuer string (e.g., "https://www.facebook.com").
        issuer: vector<u8>,

        /// Bumped every time the JWKs for the current issuer is updated.
        version: u64,

        /// The `jwks` each has a unique `id` and are sorted by `id` in dictionary order.
        jwks: vector<JWK>,
    }

    /// Multiple `ProviderJWKs`s, indexed by issuer and key ID.
    struct JWKs has copy, drop, store {
        /// Entries each has a unique `issuer`, and are sorted by `issuer` in dictionary order.
        entries: vector<ProviderJWKs>,
    }

    /// The `JWKs` that validators observed and agreed on.
    struct ObservedJWKs has copy, drop, key, store {
        jwks: JWKs,
    }

    #[event]
    /// When the `ObservedJWKs` is updated, this event is sent to resync the JWK consensus state in all validators.
    struct ObservedJWKsUpdated has drop, store {
        epoch: u64,
        jwks: JWKs,
    }

    /// A small edit that can be applied to a `JWKs`.
    struct JWKPatch has copy, drop, store {
        /// A `JWKPatch` variant packed as an `Any`.
        /// Currently the variant type is one of the following.
        /// - `JWKPatchRemoveAll`
        /// - `JWKPatchRemoveIssuer`
        /// - `JWKPatchRemoveJWK`
        /// - `JWKPatchUpsertJWK`
        variant: Any,
    }

    /// A `JWKPatch` variant to remove all JWKs.
    struct JWKPatchRemoveAll has copy, drop, store {}

    /// A `JWKPatch` variant to remove all JWKs from an issuer.
    struct JWKPatchRemoveIssuer has copy, drop, store {
        issuer: vector<u8>,
    }

    /// A `JWKPatch` variant to remove a JWK.
    struct JWKPatchRemoveJWK has copy, drop, store {
        issuer: vector<u8>,
        jwk_id: vector<u8>,
    }

    /// A `JWKPatch` variant to upsert a JWK.
    struct JWKPatchUpsertJWK has copy, drop, store {
        issuer: vector<u8>,
        jwk: JWK,
    }

    /// A sequence of `JWKPatch` that needs to be applied *one by one* to the `ObservedJWKs`.
    ///
    /// Maintained by governance proposals.
    struct JWKPatches has key {
        patches: vector<JWKPatch>,
    }

    /// The result of applying the `JWKPatches` to the `ObservedJWKs`.
    /// This is what applications should consume.
    struct FinalJWKs has drop, key {
        jwks: JWKs,
    }

    //
    // Structs end.
    // Functions begin.
    //

    /// Return whether a JWK can be found by issuer and key ID in the `FinalJWKs`.
    public fun exists_in_final_jwks(issuer: vector<u8>, jwk_id: vector<u8>): bool acquires FinalJWKs {
        let jwks = &borrow_global<FinalJWKs>(@aptos_framework).jwks;
        exists_in_jwks(jwks, issuer, jwk_id)
    }

    /// Get a JWK by issuer and key ID from the `FinalJWKs`.
    /// Abort if such a JWK does not exist.
    public fun get_final_jwk(issuer: vector<u8>, jwk_id: vector<u8>): JWK acquires FinalJWKs {
        let jwks = &borrow_global<FinalJWKs>(@aptos_framework).jwks;
        get_jwk_from_jwks(jwks, issuer, jwk_id)
    }

    /// Get a JWK by issuer and key ID from the `FinalJWKs`, if it exists.
    public fun try_get_final_jwk(issuer: vector<u8>, jwk_id: vector<u8>): Option<JWK> acquires FinalJWKs {
        let jwks = &borrow_global<FinalJWKs>(@aptos_framework).jwks;
        try_get_jwk_from_jwks(jwks, issuer, jwk_id)
    }

    /// Upsert an OIDC provider metadata into the `SupportedOIDCProviders`.
    /// Can only be called in a governance proposal.
    public fun upsert_into_supported_oidc_providers(account: &signer, name: vector<u8>, config_url: vector<u8>): Option<vector<u8>> acquires SupportedOIDCProviders {
        system_addresses::assert_aptos_framework(account);

        let provider_set = borrow_global_mut<SupportedOIDCProviders>(@aptos_framework);

        let (name_exists, idx) = vector::find(&provider_set.providers, |obj| {
            let provider: &OIDCProvider = obj;
            provider.name == name
        });

        let old_config_endpoint = if (name_exists) {
            let old_provider_info = vector::swap_remove(&mut provider_set.providers, idx);
            option::some(old_provider_info.config_url)
        } else {
            option::none()
        };

        vector::push_back(&mut provider_set.providers, OIDCProvider { name, config_url });

        old_config_endpoint
    }

    /// Remove an OIDC provider from the `SupportedOIDCProviders`.
    /// Can only be called in a governance proposal.
    public fun remove_from_supported_oidc_providers(account: &signer, name: vector<u8>): Option<vector<u8>> acquires SupportedOIDCProviders {
        system_addresses::assert_aptos_framework(account);

        let provider_set = borrow_global_mut<SupportedOIDCProviders>(@aptos_framework);

        let (name_exists, idx) = vector::find(&provider_set.providers, |obj| {
            let provider: &OIDCProvider = obj;
            provider.name == name
        });

        let old_config_endpoint = if (name_exists) {
            let old_provider_info = vector::swap_remove(&mut provider_set.providers, idx);
            option::some(old_provider_info.config_url)
        } else {
            option::none()
        };

        old_config_endpoint
    }

    /// Set the `JWKPatches`. Only called in governance proposals.
    public fun set_jwk_patches(aptos_framework: &signer, patches: vector<JWKPatch>) acquires JWKPatches, FinalJWKs, ObservedJWKs {
        system_addresses::assert_aptos_framework(aptos_framework);
        borrow_global_mut<JWKPatches>(@aptos_framework).patches = patches;
        regenerate_final_jwks();
    }

    /// Create a `JWKPatch` that removes all entries.
    public fun new_jwk_patch_remove_all(): JWKPatch {
        JWKPatch {
            variant: pack(JWKPatchRemoveAll {}),
        }
    }

    /// Create a `JWKPatch` that removes the entry of a given issuer, if exists.
    public fun new_jwk_patch_remove_issuer(issuer: vector<u8>): JWKPatch {
        JWKPatch {
            variant: pack(JWKPatchRemoveIssuer { issuer }),
        }
    }

    /// Create a `JWKPatch` that removes the entry of a given issuer, if exists.
    public fun new_jwk_patch_remove_jwk(issuer: vector<u8>, jwk_id: vector<u8>): JWKPatch {
        JWKPatch {
            variant: pack(JWKPatchRemoveJWK { issuer, jwk_id })
        }
    }

    /// Create a `JWKPatch` that upserts a JWK into an issuer's JWK set.
    public fun new_jwk_patch_upsert_jwk(issuer: vector<u8>, jwk: JWK): JWKPatch {
        JWKPatch {
            variant: pack(JWKPatchUpsertJWK { issuer, jwk })
        }
    }

    /// Create a `JWK` of variant `RSA_JWK`.
    public fun new_rsa_jwk(kid: String, alg: String, e: String, n: String): JWK {
        JWK {
            variant: copyable_any::pack(RSA_JWK {
                kid,
                kty: utf8(b"RSA"),
                e,
                n,
                alg,
            }),
        }
    }

    /// Create a `JWK` of variant `UnsupportedJWK`.
    public fun new_unsupported_jwk(id: vector<u8>, payload: vector<u8>): JWK {
        JWK {
            variant: copyable_any::pack(UnsupportedJWK { id, payload })
        }
    }

    /// Initialize some JWK resources. Should only be invoked by genesis.
    public(friend) fun initialize(account: &signer) {
        system_addresses::assert_aptos_framework(account);
        move_to(account, SupportedOIDCProviders { providers: vector[] });
        move_to(account, ObservedJWKs { jwks: JWKs { entries: vector[] } });
        move_to(account, JWKPatches { patches: vector[] });
        move_to(account, FinalJWKs { jwks: JWKs { entries: vector [] } });
    }

    /// Only used by validators to publish their observed JWK update.
    ///
    /// NOTE: It is assumed verification has been done to ensure each update is quorum-certified,
    /// and its `version` equals to the on-chain version + 1.
    fun upsert_into_observed_jwks(aptos_framework: &signer, provider_jwks_vec: vector<ProviderJWKs>) acquires ObservedJWKs, FinalJWKs, JWKPatches {
        system_addresses::assert_aptos_framework(aptos_framework);
        let observed_jwks = borrow_global_mut<ObservedJWKs>(@aptos_framework);
        vector::for_each(provider_jwks_vec, |obj| {
            let provider_jwks: ProviderJWKs = obj;
            upsert_into_jwks(&mut observed_jwks.jwks, provider_jwks);
        });

        let epoch = reconfiguration::current_epoch();
        emit(ObservedJWKsUpdated { epoch, jwks: observed_jwks.jwks });
        regenerate_final_jwks();
    }

    /// Regenerate `FinalJWKs` from `ObservedJWKs` and `JWKPatches` and save the result.
    fun regenerate_final_jwks() acquires FinalJWKs, JWKPatches, ObservedJWKs {
        let jwks = borrow_global<ObservedJWKs>(@aptos_framework).jwks;
        let patches = borrow_global<JWKPatches>(@aptos_framework);
        vector::for_each_ref(&patches.patches, |obj|{
            let patch: &JWKPatch = obj;
            apply_patch_to_jwks(&mut jwks, *patch);
        });
        *borrow_global_mut<FinalJWKs>(@aptos_framework) = FinalJWKs { jwks };
    }

    /// Return whether a JWK can be found by issuer and key ID in a `JWKs`.
    fun exists_in_jwks(jwks: &JWKs, issuer: vector<u8>, jwk_id: vector<u8>): bool {
        let (issuer_found, index) = vector::find(&jwks.entries, |obj| {
            let provider_jwks: &ProviderJWKs = obj;
            !is_greater_than(&compare_u8_vector(issuer, provider_jwks.issuer))
        });

        issuer_found && exists_in_provider_jwks(vector::borrow(&jwks.entries, index), jwk_id)
    }

    /// Return whether a JWK can be found by key ID in a `ProviderJWKs`.
    fun exists_in_provider_jwks(provider_jwks: &ProviderJWKs, jwk_id: vector<u8>): bool {
        vector::any(&provider_jwks.jwks, |obj| {
            let jwk: &JWK = obj;
            jwk_id == get_jwk_id(jwk)
        })
    }

    /// Get a JWK by issuer and key ID from a `JWKs`.
    /// Abort if such a JWK does not exist.
    fun get_jwk_from_jwks(jwks: &JWKs, issuer: vector<u8>, jwk_id: vector<u8>): JWK {
        let (issuer_found, index) = vector::find(&jwks.entries, |obj| {
            let provider_jwks: &ProviderJWKs = obj;
            !is_greater_than(&compare_u8_vector(issuer, provider_jwks.issuer))
        });

        assert!(issuer_found, invalid_argument(EISSUER_NOT_FOUND));
        get_jwk_from_provider_jwks(vector::borrow(&jwks.entries, index), jwk_id)

    }

    /// Get a JWK by key ID from a `ProviderJWKs`.
    /// Abort if such a JWK does not exist.
    fun get_jwk_from_provider_jwks(provider_jwks: &ProviderJWKs, jwk_id: vector<u8>): JWK {
        let (jwk_id_found, index) = vector::find(&provider_jwks.jwks, |obj|{
            let jwk: &JWK = obj;
            !is_greater_than(&compare_u8_vector(jwk_id, get_jwk_id(jwk)))
        });

        assert!(jwk_id_found, error::invalid_argument(EJWK_ID_NOT_FOUND));
        *vector::borrow(&provider_jwks.jwks, index)
    }

    /// Get a JWK by issuer and key ID from a `JWKs`, if it exists.
    fun try_get_jwk_from_jwks(jwks: &JWKs, issuer: vector<u8>, jwk_id: vector<u8>): Option<JWK> {
        let (issuer_found, index) = vector::find(&jwks.entries, |obj| {
            let provider_jwks: &ProviderJWKs = obj;
            !is_greater_than(&compare_u8_vector(issuer, provider_jwks.issuer))
        });

        if (issuer_found) {
            try_get_jwk_from_provider_jwks(vector::borrow(&jwks.entries, index), jwk_id)
        } else {
            option::none()
        }

    }

    /// Get a JWK by key ID from a `ProviderJWKs`, if it exists.
    fun try_get_jwk_from_provider_jwks(provider_jwks: &ProviderJWKs, jwk_id: vector<u8>): Option<JWK> {
        let (jwk_id_found, index) = vector::find(&provider_jwks.jwks, |obj|{
            let jwk: &JWK = obj;
            !is_greater_than(&compare_u8_vector(jwk_id, get_jwk_id(jwk)))
        });

        if (jwk_id_found) {
            option::some(*vector::borrow(&provider_jwks.jwks, index))
        } else {
            option::none()
        }
    }

    /// Get the ID of a JWK.
    fun get_jwk_id(jwk: &JWK): vector<u8> {
        let variant_type_name = *string::bytes(copyable_any::type_name(&jwk.variant));
        if (variant_type_name == b"0x1::jwks::RSA_JWK") {
            let rsa = copyable_any::unpack<RSA_JWK>(jwk.variant);
            *string::bytes(&rsa.kid)
        } else if (variant_type_name == b"0x1::jwks::UnsupportedJWK") {
            let unsupported = copyable_any::unpack<UnsupportedJWK>(jwk.variant);
            unsupported.id
        } else {
            abort(error::invalid_argument(EUNKNOWN_JWK_VARIANT))
        }
    }

    /// Upsert a `ProviderJWKs` into a `JWKs`. If this upsert replaced an existing entry, return it.
    fun upsert_into_jwks(jwks: &mut JWKs, provider_jwks: ProviderJWKs): Option<ProviderJWKs> {
        let found = false;
        let index = 0;
        let num_entries = vector::length(&jwks.entries);
        while (index < num_entries) {
            let cur_entry = vector::borrow(&jwks.entries, index);
            let comparison = compare_u8_vector(provider_jwks.issuer, cur_entry.issuer);
            if (is_greater_than(&comparison)) {
                index = index + 1;
            } else {
                found = is_equal(&comparison);
                break
            }
        };

        let ret = if (found) {
            option::some(vector::remove(&mut jwks.entries, index))
        } else {
            option::none()
        };

        vector::insert(&mut jwks.entries, index, provider_jwks);

        ret
    }

    /// Remove the entry of an issuer from a `JWKs` and return the entry, if exists.
    fun remove_from_jwks(jwks: &mut JWKs, issuer: vector<u8>): Option<ProviderJWKs> {
        let found = false;
        let index = 0;
        let num_entries = vector::length(&jwks.entries);
        while (index < num_entries) {
            let cur_entry = vector::borrow(&jwks.entries, index);
            let comparison = compare_u8_vector(issuer, cur_entry.issuer);
            if (is_greater_than(&comparison)) {
                index = index + 1;
            } else {
                found = is_equal(&comparison);
                break
            }
        };

        let ret = if (found) {
            option::some(vector::remove(&mut jwks.entries, index))
        } else {
            option::none()
        };

        ret
    }

    /// Upsert a `JWK` into a `ProviderJWKs`. If this upsert replaced an existing entry, return it.
    fun upsert_into_provider_jwks(set: &mut ProviderJWKs, jwk: JWK): Option<JWK> {
        let found = false;
        let index = 0;
        let num_entries = vector::length(&set.jwks);
        while (index < num_entries) {
            let cur_entry = vector::borrow(&set.jwks, index);
            let comparison = compare_u8_vector(get_jwk_id(&jwk), get_jwk_id(cur_entry));
            if (is_greater_than(&comparison)) {
                index = index + 1;
            } else {
                found = is_equal(&comparison);
                break
            }
        };

        // Now if `found == true`, `index` points to the JWK we want to update/remove; otherwise, `index` points to where we want to insert.

        let ret = if (found) {
            option::some(vector::remove(&mut set.jwks, index))
        } else {
            option::none()
        };

        vector::insert(&mut set.jwks, index, jwk);

        ret
    }

    /// Remove the entry of a key ID from a `ProviderJWKs` and return the entry, if exists.
    fun remove_from_provider_jwks(jwks: &mut ProviderJWKs, jwk_id: vector<u8>): Option<JWK> {
        let found = false;
        let index = 0;
        let num_entries = vector::length(&jwks.jwks);
        while (index < num_entries) {
            let cur_entry = vector::borrow(&jwks.jwks, index);
            let comparison = compare_u8_vector(jwk_id, get_jwk_id(cur_entry));
            if (is_greater_than(&comparison)) {
                index = index + 1;
            } else {
                found = is_equal(&comparison);
                break
            }
        };

        // Now if `found == true`, `index` points to the JWK we want to update/remove; otherwise, `index` points to where we want to insert.

        let ret = if (found) {
            option::some(vector::remove(&mut jwks.jwks, index))
        } else {
            option::none()
        };

        ret
    }

    /// Modify a `JWKs`.
    fun apply_patch_to_jwks(jwks: &mut JWKs, patch: JWKPatch) {
        let variant_type_name = *string::bytes(copyable_any::type_name(&patch.variant));
        if (variant_type_name == b"0x1::jwks::JWKPatchRemoveAll") {
            jwks.entries = vector[];
        } else if (variant_type_name == b"0x1::jwks::JWKPatchRemoveIssuer") {
            let cmd = copyable_any::unpack<JWKPatchRemoveIssuer>(patch.variant);
            let (found, index) = vector::find(&jwks.entries, |obj| {
                let provider_jwk_set: &ProviderJWKs = obj;
                provider_jwk_set.issuer == cmd.issuer
            });
            if (found) {
                vector::swap_remove(&mut jwks.entries, index);
            };
        } else if (variant_type_name == b"0x1::jwks::JWKPatchRemoveJWK") {
            let cmd = copyable_any::unpack<JWKPatchRemoveJWK>(patch.variant);
            let existing_jwk_set = remove_from_jwks(jwks, cmd.issuer);
            if (option::is_some(&existing_jwk_set)) {
                let jwk_set = option::extract(&mut existing_jwk_set);
                remove_from_provider_jwks(&mut jwk_set, cmd.jwk_id);
                upsert_into_jwks(jwks, jwk_set);
            };
        } else if (variant_type_name == b"0x1::jwks::JWKPatchUpsertJWK") {
            let cmd = copyable_any::unpack<JWKPatchUpsertJWK>(patch.variant);
            let existing_jwk_set = remove_from_jwks(jwks, cmd.issuer);
            let jwk_set = if (option::is_some(&existing_jwk_set)) {
                option::extract(&mut existing_jwk_set)
            } else {
                ProviderJWKs {
                    version: 0,
                    issuer: cmd.issuer,
                    jwks: vector[],
                }
            };
            upsert_into_provider_jwks(&mut jwk_set, cmd.jwk);
            upsert_into_jwks(jwks, jwk_set);
        } else {
            abort(std::error::invalid_argument(EUNKNOWN_JWKPATCH_VARIANT))
        }
    }

    //
    // Functions end.
    // Tests begin.
    //

    #[test_only]
    fun initialize_for_test(aptos_framework: &signer) {
        create_account_for_test(@aptos_framework);
        reconfiguration::initialize_for_test(aptos_framework);
        initialize(aptos_framework);
    }

    #[test]
    fun test_apply_patch_to_jwks() {
        let jwks = JWKs {
            entries: vector[
                ProviderJWKs {
                    issuer: b"alice",
                    version: 111,
                    jwks: vector[
                        new_rsa_jwk(
                            utf8(b"e4adfb436b9e197e2e1106af2c842284e4986aff"), // kid
                            utf8(b"RS256"), // alg
                            utf8(b"AQAB"), // e
                            utf8(b"psply8S991RswM0JQJwv51fooFFvZUtYdL8avyKObshyzj7oJuJD8vkf5DKJJF1XOGi6Wv2D-U4b3htgrVXeOjAvaKTYtrQVUG_Txwjebdm2EvBJ4R6UaOULjavcSkb8VzW4l4AmP_yWoidkHq8n6vfHt9alDAONILi7jPDzRC7NvnHQ_x0hkRVh_OAmOJCpkgb0gx9-U8zSBSmowQmvw15AZ1I0buYZSSugY7jwNS2U716oujAiqtRkC7kg4gPouW_SxMleeo8PyRsHpYCfBME66m-P8Zr9Fh1Qgmqg4cWdy_6wUuNc1cbVY_7w1BpHZtZCNeQ56AHUgUFmo2LAQQ"), // n
                        ),
                        new_unsupported_jwk(b"key_id_0", b"key_content_0"),
                    ],
                },
                ProviderJWKs {
                    issuer: b"bob",
                    version: 222,
                    jwks: vector[
                        new_unsupported_jwk(b"key_id_1", b"key_content_1"),
                        new_unsupported_jwk(b"key_id_2", b"key_content_2"),
                    ],
                },
            ],
        };

        let patch = new_jwk_patch_remove_issuer(b"alice");
        apply_patch_to_jwks(&mut jwks, patch);
        assert!(jwks == JWKs {
            entries: vector[
                ProviderJWKs {
                    issuer: b"bob",
                    version: 222,
                    jwks: vector[
                        new_unsupported_jwk(b"key_id_1", b"key_content_1"),
                        new_unsupported_jwk(b"key_id_2", b"key_content_2"),
                    ],
                },
            ],
        }, 1);

        let patch = new_jwk_patch_remove_jwk(b"bob", b"key_id_1");
        apply_patch_to_jwks(&mut jwks, patch);
        assert!(jwks == JWKs {
            entries: vector[
                ProviderJWKs {
                    issuer: b"bob",
                    version: 222,
                    jwks: vector[
                        new_unsupported_jwk(b"key_id_2", b"key_content_2"),
                    ],
                },
            ],
        }, 1);

        let patch = new_jwk_patch_upsert_jwk(b"carl", new_rsa_jwk(
            utf8(b"0ad1fec78504f447bae65bcf5afaedb65eec9e81"), // kid
            utf8(b"RS256"), // alg
            utf8(b"AQAB"), // e
            utf8(b"sm72oBH-R2Rqt4hkjp66tz5qCtq42TMnVgZg2Pdm_zs7_-EoFyNs9sD1MKsZAFaBPXBHDiWywyaHhLgwETLN9hlJIZPzGCEtV3mXJFSYG-8L6t3kyKi9X1lUTZzbmNpE0tf-eMW-3gs3VQSBJQOcQnuiANxbSXwS3PFmi173C_5fDSuC1RoYGT6X3JqLc3DWUmBGucuQjPaUF0w6LMqEIy0W_WYbW7HImwANT6dT52T72md0JWZuAKsRRnRr_bvaUX8_e3K8Pb1K_t3dD6WSLvtmEfUnGQgLynVl3aV5sRYC0Hy_IkRgoxl2fd8AaZT1X_rdPexYpx152Pl_CHJ79Q"), // n
        ));
        apply_patch_to_jwks(&mut jwks, patch);
        let edit = new_jwk_patch_upsert_jwk(b"bob", new_unsupported_jwk(b"key_id_2", b"key_content_2b"));
        apply_patch_to_jwks(&mut jwks, edit);
        let edit = new_jwk_patch_upsert_jwk(b"alice", new_unsupported_jwk(b"key_id_3", b"key_content_3"));
        apply_patch_to_jwks(&mut jwks, edit);
        let edit = new_jwk_patch_upsert_jwk(b"alice", new_unsupported_jwk(b"key_id_0", b"key_content_0b"));
        apply_patch_to_jwks(&mut jwks, edit);
        assert!(jwks == JWKs {
            entries: vector[
                ProviderJWKs {
                    issuer: b"alice",
                    version: 0,
                    jwks: vector[
                        new_unsupported_jwk(b"key_id_0", b"key_content_0b"),
                        new_unsupported_jwk(b"key_id_3", b"key_content_3"),
                    ],
                },
                ProviderJWKs {
                    issuer: b"bob",
                    version: 222,
                    jwks: vector[
                        new_unsupported_jwk(b"key_id_2", b"key_content_2b"),
                    ],
                },
                ProviderJWKs {
                    issuer: b"carl",
                    version: 0,
                    jwks: vector[
                        new_rsa_jwk(
                            utf8(b"0ad1fec78504f447bae65bcf5afaedb65eec9e81"), // kid
                            utf8(b"RS256"), // alg
                            utf8(b"AQAB"), // e
                            utf8(b"sm72oBH-R2Rqt4hkjp66tz5qCtq42TMnVgZg2Pdm_zs7_-EoFyNs9sD1MKsZAFaBPXBHDiWywyaHhLgwETLN9hlJIZPzGCEtV3mXJFSYG-8L6t3kyKi9X1lUTZzbmNpE0tf-eMW-3gs3VQSBJQOcQnuiANxbSXwS3PFmi173C_5fDSuC1RoYGT6X3JqLc3DWUmBGucuQjPaUF0w6LMqEIy0W_WYbW7HImwANT6dT52T72md0JWZuAKsRRnRr_bvaUX8_e3K8Pb1K_t3dD6WSLvtmEfUnGQgLynVl3aV5sRYC0Hy_IkRgoxl2fd8AaZT1X_rdPexYpx152Pl_CHJ79Q"), // n
                        )
                    ],
                },
            ],
        }, 1);

        let patch = new_jwk_patch_remove_all();
        apply_patch_to_jwks(&mut jwks, patch);
        assert!(jwks == JWKs { entries: vector[] }, 1);
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_final_jwks(aptos_framework: signer) acquires ObservedJWKs, FinalJWKs, JWKPatches {
        initialize_for_test(&aptos_framework);
        let jwk_0 = new_unsupported_jwk(b"key_id_0", b"key_payload_0");
        let jwk_1 = new_unsupported_jwk(b"key_id_1", b"key_payload_1");
        let jwk_2 = new_unsupported_jwk(b"key_id_2", b"key_payload_2");
        let jwk_3 = new_unsupported_jwk(b"key_id_3", b"key_payload_3");
        let jwk_3b = new_unsupported_jwk(b"key_id_3", b"key_payload_3b");

        // Fake observation from validators.
        upsert_into_observed_jwks(&aptos_framework, vector [
            ProviderJWKs {
                issuer: b"alice",
                version: 111,
                jwks: vector[jwk_0, jwk_1],
            },
            ProviderJWKs{
                issuer: b"bob",
                version: 222,
                jwks: vector[jwk_2, jwk_3],
            },
        ]);
        assert!(jwk_3 == get_final_jwk(b"bob", b"key_id_3"), 1);
        assert!(exists_in_final_jwks(b"bob", b"key_id_3"), 1);
        assert!(option::some(jwk_3) == try_get_final_jwk(b"bob", b"key_id_3"), 1);

        // Ignore all Bob's keys.
        set_jwk_patches(&aptos_framework, vector[
            new_jwk_patch_remove_issuer(b"bob"),
        ]);
        assert!(!exists_in_final_jwks(b"bob", b"key_id_3"), 1);
        assert!(option::none() == try_get_final_jwk(b"bob", b"key_id_3"), 1);

        // Update one of Bob's key..
        set_jwk_patches(&aptos_framework, vector[
            new_jwk_patch_upsert_jwk(b"bob", jwk_3b),
        ]);
        assert!(jwk_3b == get_final_jwk(b"bob", b"key_id_3"), 1);
        assert!(exists_in_final_jwks(b"bob", b"key_id_3"), 1);
        assert!(option::some(jwk_3b) == try_get_final_jwk(b"bob", b"key_id_3"), 1);

        // Wipe everything, then add some keys back.
        set_jwk_patches(&aptos_framework, vector[
            new_jwk_patch_remove_all(),
            new_jwk_patch_upsert_jwk(b"alice", jwk_1),
            new_jwk_patch_upsert_jwk(b"bob", jwk_3),
        ]);
        assert!(jwk_3 == get_final_jwk(b"bob", b"key_id_3"), 1);
        assert!(exists_in_final_jwks(b"bob", b"key_id_3"), 1);
        assert!(option::some(jwk_3) == try_get_final_jwk(b"bob", b"key_id_3"), 1);
    }
}
