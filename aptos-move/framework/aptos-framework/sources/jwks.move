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
    use aptos_std::debug;

    friend aptos_framework::genesis;

    const EUNEXPECTED_EPOCH: u64 = 1;
    const EUNEXPECTED_VERSION: u64 = 2;
    const EUNKNOWN_JWKPATCH_VARIANT: u64 = 3;
    const EUNKNOWN_JWK_VARIANT: u64 = 4;

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

        /// The `jwks` each has a unique `id` and are sorted by `id` in dictionary order.
        jwks: vector<JWK>,
    }

    /// Multiple `JWK`s indexed by issuer then JWK ID.
    struct JWKs has copy, drop, store {
        /// Entries each has a unique `issuer`, and are sorted by `issuer` in dictionary order.
        entries: vector<ProviderJWKs>,
    }

    /// The `JWKs` that validators observed and agreed on.
    struct ObservedJWKs has copy, drop, key, store {
        version: u64,
        jwks: JWKs,
    }

    #[event]
    /// When the `ObservedJWKs` is updated, this event is sent to reset the JWK consensus state in all validators.
    struct ObservedJWKsUpdated has drop, store {
        epoch: u64,
        version: u64,
        jwks: JWKs,
    }

    /// A small edit that can be applied to a `JWKs`.
    struct JWKPatch has drop, store {
        /// A `JWKPatch` variant packed as an `Any`.
        /// Currently the variant type is one of the following.
        /// - `JWKPatchDeleteAll`
        /// - `JWKPatchDeleteIssuer`
        /// - `JWKPatchDeleteJWK`
        /// - `JWKPatchUpsertJWK`
        variant: Any,
    }

    /// A `JWKPatch` variant to delete all JWKs.
    struct JWKPatchDeleteAll has copy, drop, store {}

    /// A `JWKPatch` variant to delete all JWKs from an issuer.
    struct JWKPatchDeleteIssuer has copy, drop, store {
        issuer: vector<u8>,
    }

    /// A `JWKPatch` variant to delete a JWK.
    struct JWKPatchDeleteJWK has copy, drop, store {
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
    struct FinalJWKs {
        jwks: JWKs,
    }

    //
    // Structs end.
    // Functions begin.
    //

    /// Initialize some JWK resources. Should only be invoked by genesis.
    public(friend) fun initialize(account: &signer) {
        system_addresses::assert_aptos_framework(account);
        move_to(account, SupportedOIDCProviders { providers: vector[] });
        move_to(account, ObservedJWKs { version: 0, jwks: JWKs { entries: vector[] } });
        move_to(account, JWKPatches { patches: vector[] });
    }

    /// (1) Remove the entry for a provider of a given name from the provider set, if it exists.
    /// (2) Add a new entry for the provider with the new config endpoint, if provided.
    /// (3) Return the removed config endpoint in (1), if it happened.
    ///
    /// Designed to be used only in governance proposal-only.
    public fun update_oidc_provider(account: &signer, name: vector<u8>, new_config_url: Option<vector<u8>>): Option<vector<u8>> acquires SupportedOIDCProviders {
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

        if (option::is_some(&new_config_url)) {
            let config_endpoint = option::extract(&mut new_config_url);
            vector::push_back(&mut provider_set.providers, OIDCProvider { name, config_url: config_endpoint });
        };

        old_config_endpoint
    }

    /// Update the JWK set. Should only be invoked by validator transactions/governance proposals.
    public fun set_observed_jwks(account: signer, epoch: u64, version: u64, jwks: JWKs) acquires ObservedJWKs {
        system_addresses::assert_aptos_framework(&account);

        // Epoch check.
        assert!(reconfiguration::current_epoch() == epoch, invalid_argument(EUNEXPECTED_EPOCH));

        let observed_jwks = borrow_global_mut<ObservedJWKs>(@aptos_framework);

        // Version check.
        assert!(observed_jwks.version + 1 == version, invalid_argument(EUNEXPECTED_VERSION));

        // Replace.
        *observed_jwks = ObservedJWKs { version, jwks: jwks };
        emit(ObservedJWKsUpdated { epoch, version, jwks: jwks });
    }

    /// Update the `JWKPatches`. This is governance proposal-only.
    public fun set_jwk_patches(aptos_framework: &signer, patches: vector<JWKPatch>) acquires JWKPatches {
        system_addresses::assert_aptos_framework(aptos_framework);
        borrow_global_mut<JWKPatches>(@aptos_framework).patches = patches;
    }

    /// Create a `JWKPatch` that deletes all entries.
    public fun new_jwk_patch_del_all(): JWKPatch {
        JWKPatch {
            variant: pack(JWKPatchDeleteAll {}),
        }
    }

    /// Create a `JWKPatch` that deletes the entry of a given issuer, if exists.
    public fun new_jwk_patch_del_issuer(issuer: vector<u8>): JWKPatch {
        JWKPatch {
            variant: pack(JWKPatchDeleteIssuer { issuer }),
        }
    }

    /// Create a `JWKPatch` that deletes the entry of a given issuer, if exists.
    public fun new_jwk_patch_del_jwk(issuer: vector<u8>, jwk_id: vector<u8>): JWKPatch {
        JWKPatch {
            variant: pack(JWKPatchDeleteJWK { issuer, jwk_id })
        }
    }

    /// Create a `JWKPatch` that upserts a JWK into an issuer's JWK set.
    public fun new_jwk_patch_upsert_jwk(issuer: vector<u8>, jwk: JWK): JWKPatch {
        JWKPatch {
            variant: pack(JWKPatchUpsertJWK { issuer, jwk })
        }
    }

    /// Get the ID of a JWK.
    public fun get_jwk_id(jwk: &JWK): vector<u8> {
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
    public fun upsert_into_jwks(jwks: &mut JWKs, provider_jwks: ProviderJWKs): Option<ProviderJWKs> {
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

        // Now if `found == true`, `index` points to the JWK we want to update/delete; otherwise, `index` points to where we want to insert.

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

        // Now if `found == true`, `index` points to the JWK we want to update/delete; otherwise, `index` points to where we want to insert.

        let ret = if (found) {
            option::some(vector::remove(&mut jwks.jwks, index))
        } else {
            option::none()
        };

        ret
    }

    /// Create a `JWK` of variant `RSA_JWK`.
    fun new_rsa_jwk(kid: String, alg: String, e: String, n: String): JWK {
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
    fun new_unsupported_jwk(id: vector<u8>, payload: vector<u8>): JWK {
        JWK {
            variant: copyable_any::pack(UnsupportedJWK { id, payload })
        }
    }

    /// Modify a `JWKs`.
    fun apply_patch_to_jwks(jwks: &mut JWKs, patch: JWKPatch) {
        let variant_type_name = *string::bytes(copyable_any::type_name(&patch.variant));
        if (variant_type_name == b"0x1::jwks::JWKPatchDeleteAll") {
            jwks.entries = vector[];
        } else if (variant_type_name == b"0x1::jwks::JWKPatchDeleteIssuer") {
            let cmd = copyable_any::unpack<JWKPatchDeleteIssuer>(patch.variant);
            let (found, index) = vector::find(&jwks.entries, |obj| {
                let provider_jwk_set: &ProviderJWKs = obj;
                provider_jwk_set.issuer == cmd.issuer
            });
            if (found) {
                vector::swap_remove(&mut jwks.entries, index);
            };
        } else if (variant_type_name == b"0x1::jwks::JWKPatchDeleteJWK") {
            let cmd = copyable_any::unpack<JWKPatchDeleteJWK>(patch.variant);
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

    #[test]
    fun test_apply_patch_to_jwks() {
        let jwks = JWKs {
            entries: vector[
                ProviderJWKs {
                    issuer: b"alice",
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
                    jwks: vector[
                        new_unsupported_jwk(b"key_id_1", b"key_content_1"),
                        new_unsupported_jwk(b"key_id_2", b"key_content_2"),
                    ],
                },
            ],
        };

        let patch = new_jwk_patch_del_issuer(b"alice");
        apply_patch_to_jwks(&mut jwks, patch);
        assert!(jwks == JWKs {
            entries: vector[
                ProviderJWKs {
                    issuer: b"bob",
                    jwks: vector[
                        new_unsupported_jwk(b"key_id_1", b"key_content_1"),
                        new_unsupported_jwk(b"key_id_2", b"key_content_2"),
                    ],
                },
            ],
        }, 1);

        let patch = new_jwk_patch_del_jwk(b"bob", b"key_id_1");
        apply_patch_to_jwks(&mut jwks, patch);
        assert!(jwks == JWKs {
            entries: vector[
                ProviderJWKs {
                    issuer: b"bob",
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
        debug::print(&jwks);
        assert!(jwks == JWKs {
            entries: vector[
                ProviderJWKs {
                    issuer: b"alice",
                    jwks: vector[
                        new_unsupported_jwk(b"key_id_0", b"key_content_0b"),
                        new_unsupported_jwk(b"key_id_3", b"key_content_3"),
                    ],
                },
                ProviderJWKs {
                    issuer: b"bob",
                    jwks: vector[
                        new_unsupported_jwk(b"key_id_2", b"key_content_2b"),
                    ],
                },
                ProviderJWKs {
                    issuer: b"carl",
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

        let patch = new_jwk_patch_del_all();
        apply_patch_to_jwks(&mut jwks, patch);
        assert!(jwks == JWKs { entries: vector[] }, 1);
    }
}
