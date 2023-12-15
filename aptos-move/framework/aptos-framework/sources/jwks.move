/// JWK functions and structs.
module aptos_framework::jwks {
    use std::error::invalid_argument;
    use std::option;
    use std::option::Option;
    use std::string::String;
    use std::vector;
    use aptos_std::copyable_any::{Any, pack};
    use aptos_framework::event::emit;
    use aptos_framework::reconfiguration;
    use aptos_framework::system_addresses;
    #[test_only]
    use std::string;
    #[test_only]
    use std::string::utf8;
    #[test_only]
    use aptos_std::comparator::{compare_u8_vector, is_greater_than, is_equal};
    #[test_only]
    use aptos_std::copyable_any;

    friend aptos_framework::genesis;

    const EUNEXPECTED_EPOCH: u64 = 1;
    const EUNEXPECTED_VERSION: u64 = 2;
    const EINVALID_SIG: u64 = 3;
    const EUNKNOWN_JWK_MAP_EDIT: u64 = 4;

    /// An OIDC provider.
    struct OIDCProvider has drop, store {
        name: vector<u8>, // e.g., b"https://www.facebook.com"
        config_url: vector<u8>, // e.g., b"https://www.facebook.com/.well-known/openid-configuration/"
    }

    /// The OIDC provider set. Maintained by governance proposals.
    struct OIDCProviderSet has key {
        providers: vector<OIDCProvider>, // The order does not matter.
    }

    /// Some extra configs that controls JWK consensus behavior. Maintained by governance proposals.
    ///
    /// Currently supported `content` types:
    /// - `JWKConsensusConfigV0`
    struct JWKConsensusConfig has drop, key {
        content: Any,
    }


    struct JWKConsensusConfigV0 has copy, drop, store {
        observation_interval_ms: u64,
    }

    /// An observed but not yet supported JWK.
    struct UnsupportedJWK has copy, drop, store {
        payload: vector<u8>,
    }

    /// A JWK where `kty` is `RSA`.
    struct RSA_JWK has copy, drop, store {
        kid: String,
        kty: String,
        alg: String,
        e: String,
        n: String,
    }

    /// A JWK.
    ///
    /// Currently supported `content` types:
    /// - `RSA_JWK`
    /// - `UnsupportedJWK`
    struct JWK has copy, drop, store {
        id: vector<u8>,
        content: Any,
    }

    /// A provider and its JWKs.
    struct ProviderJWKSet has copy, drop, store {
        /// The utf-8 encoding of the issuer string (e.g., "https://accounts.google.com").
        issuer: vector<u8>,

        /// The `jwks` should each have a unique `id`, and should be sorted by `id` in alphabetical order.
        jwks: vector<JWK>,
    }

    /// All OIDC providers and their JWK sets.
    struct JWKMap has copy, drop, store {
        /// Entries should each have a unique `issuer`, and should be sorted by `issuer` in an alphabetical order.
        entries: vector<ProviderJWKSet>,
    }

    /// A `JWKMap` maintained by JWK consensus.
    struct OnChainJWKMap has copy, drop, key, store {
        version: u64,
        jwk_map: JWKMap,
    }

    #[event]
    /// When an on-chain JWK set update is done, this event is sent to reset the JWK consensus state in all validators.
    struct OnChainJWKMapUpdated has drop, store {
        epoch: u64,
        version: u64,
        jwk_map: JWKMap,
    }

    /// A small edit that can be applied to a `JWKMap`.
    ///
    /// Currently supported `content` types:
    /// - `JWKMapEditCmdDelAll`
    /// - `JWKMapEditCmdDelIssuer`
    /// - `JWKMapEditCmdDelJwk`
    /// - `JWKMapEditCmdPutJwk`
    struct JWKMapEdit has drop, store {
        content: Any,
    }

    struct JWKMapEditCmdDelAll has copy, drop, store {}

    struct JWKMapEditCmdDelIssuer has copy, drop, store {
        issuer: vector<u8>,
    }

    struct JWKMapEditCmdDelJwk has copy, drop, store {
        issuer: vector<u8>,
        jwk_id: vector<u8>,
    }

    struct JWKMapEditCmdPutJwk has copy, drop, store {
        issuer: vector<u8>,
        jwk: JWK,
    }

    /// A sequence of `JWKMapEdit` that needs to be applied *one by one* to the JWK consensus-maintained `JWKMap` before being used.
    ///
    /// Maintained by governance proposals.
    struct JWKMapPatch has key {
        edits: vector<JWKMapEdit>,
    }

    //
    // Structs end.
    // Public functions begin.
    //

    /// Initialize some JWK resources. Should only be invoked by genesis.
    public(friend) fun initialize(account: &signer) {
        system_addresses::assert_aptos_framework(account);
        move_to(account, OIDCProviderSet { providers: vector[] });
        move_to(account, jwk_consensus_config_v0(10000));
        move_to(account, OnChainJWKMap { version: 0, jwk_map: JWKMap { entries: vector[] } });
        move_to(account, JWKMapPatch { edits: vector[] });
    }

    /// (1) Remove the entry for a provider of a given name from the provider set, if it exists.
    /// (2) Add a new entry for the provider with the new config endpoint, if provided.
    /// (3) Return the removed config endpoint in (1), if it happened.
    ///
    /// Designed to be used only in governance proposal-only.
    public fun update_oidc_provider(account: &signer, name: vector<u8>, new_config_url: Option<vector<u8>>): Option<vector<u8>> acquires OIDCProviderSet {
        system_addresses::assert_aptos_framework(account);

        let provider_set = borrow_global_mut<OIDCProviderSet>(@aptos_framework);

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

    /// Create a `JWKConsensusConfig` with content type `JWKConsensusConfigV0`.
    public fun jwk_consensus_config_v0(observation_interval_ms: u64): JWKConsensusConfig {
        let v0 = JWKConsensusConfigV0 { observation_interval_ms };
        JWKConsensusConfig {
            content: pack(v0),
        }
    }

    /// Update JWK consensus config. Should only be invoked by governance proposals.
    public fun update_jwk_consensus_config(account: &signer, config: JWKConsensusConfig) acquires JWKConsensusConfig {
        system_addresses::assert_aptos_framework(account);
        *borrow_global_mut<JWKConsensusConfig>(@aptos_framework) = config;
    }

    //
    // Public functions end.
    // Private functions begin.
    //

    /// Update the JWK set. Should only be invoked by validator transactions/governance proposals.
    public fun update_onchain_jwk_map(account: signer, epoch: u64, version: u64, jwk_map: JWKMap) acquires OnChainJWKMap {
        system_addresses::assert_aptos_framework(&account);

        // Epoch check.
        assert!(reconfiguration::current_epoch() == epoch, invalid_argument(EUNEXPECTED_EPOCH));

        let onchain_jwk_map = borrow_global_mut<OnChainJWKMap>(@aptos_framework);

        // Version check.
        assert!(onchain_jwk_map.version + 1 == version, invalid_argument(EUNEXPECTED_VERSION));

        // Replace.
        *onchain_jwk_map = OnChainJWKMap { version, jwk_map };
        emit(OnChainJWKMapUpdated{ epoch, version, jwk_map });
    }

    /// Update the system `JWKMapPatch`. This is governance proposal-only.
    public fun update_jwk_map_patch(aptos_framework: &signer, edits: vector<JWKMapEdit>) acquires JWKMapPatch {
        system_addresses::assert_aptos_framework(aptos_framework);
        let patch = borrow_global_mut<JWKMapPatch>(@aptos_framework);
        patch.edits = edits;
    }

    /// Create a JWKMap edit command that deletes all entries.
    public fun jwk_map_edit_del_all(): JWKMapEdit {
        JWKMapEdit {
            content: pack(JWKMapEditCmdDelAll {}),
        }
    }

    /// Create a JWKMap edit command that deletes the entry for a given issuer, if exists.
    public fun jwk_map_edit_del_issuer(issuer: vector<u8>): JWKMapEdit {
        JWKMapEdit {
            content: pack(JWKMapEditCmdDelIssuer { issuer }),
        }
    }

    /// Create a JWKMap edit command that deletes the entry for a given issuer, if exists.
    public fun jwk_map_edit_del_jwk(issuer: vector<u8>, jwk_id: vector<u8>): JWKMapEdit {
        JWKMapEdit {
            content: pack(JWKMapEditCmdDelJwk { issuer, jwk_id })
        }
    }

    /// Create a JWKMap edit command that upserts a JWK into an issuer's JWK set.
    public fun jwk_map_edit_put_jwk(issuer: vector<u8>, jwk: JWK): JWKMapEdit {
        JWKMapEdit {
            content: pack(JWKMapEditCmdPutJwk { issuer, jwk })
        }
    }

    //
    // Private functions end.
    // Tests begin.
    //

    #[test_only]
    /// Insert a JWK set if `issuer` does not exist and `jwk_set` is some.
    /// Update an existing JWK set if `issuer` exists and `jwk_set` is some.
    /// Delete a JWK set if `issuer` exists but `jwk_set` is none.
    fun update_jwk_map(map: &mut JWKMap, issuer: vector<u8>, jwk_set: Option<ProviderJWKSet>): Option<ProviderJWKSet> {
        let found = false;
        let index = 0;
        let num_entries = vector::length(&map.entries);
        while (index < num_entries) {
            let cur_entry = vector::borrow(&map.entries, index);
            let comparison = compare_u8_vector(issuer, cur_entry.issuer);
            if (is_greater_than(&comparison)) {
                index = index + 1;
            } else {
                found = is_equal(&comparison);
                break
            }
        };

        let ret = if (found) {
            option::some(vector::remove(&mut map.entries, index))
        } else {
            option::none()
        };

        if (option::is_some(&jwk_set)) {
            let jwk_set = option::extract(&mut jwk_set);
            vector::insert(&mut map.entries, index, jwk_set);
        };

        ret
    }

    #[test_only]
    /// Insert a JWK if `jwk_id` does not exist and `jwk` is some.
    /// Update an existing JWK if `jwk_id` exists and `jwk` is some.
    /// Delete a JWK if `jwk_id` exists but `jwk` is none.
    fun update_jwk_set(set: &mut ProviderJWKSet, jwk_id: vector<u8>, jwk: Option<JWK>): Option<JWK> {
        let found = false;
        let index = 0;
        let num_entries = vector::length(&set.jwks);
        while (index < num_entries) {
            let cur_entry = vector::borrow(&set.jwks, index);
            let comparison = compare_u8_vector(jwk_id, cur_entry.id);
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

        if (option::is_some(&jwk)) {
            let jwk = option::extract(&mut jwk);
            vector::insert(&mut set.jwks, index, jwk);
        };

        ret
    }

    #[test_only]
    fun jwk_rsa(rsa: RSA_JWK): JWK {
        JWK {
            id: *string::bytes(&rsa.kid),
            content: copyable_any::pack(rsa),
        }
    }

    #[test_only]
    fun jwk_unsupported(id: vector<u8>, payload: vector<u8>): JWK {
        JWK {
            id,
            content: copyable_any::pack(UnsupportedJWK { payload })
        }
    }

    #[test_only]
    /// Apply a small edit to a `JWKMap`.
    fun apply_edit_to_jwk_map(map: &mut JWKMap, edit: JWKMapEdit) {
        let variant_type_name = *string::bytes(copyable_any::type_name(&edit.content));
        if (variant_type_name == b"0x1::jwks::JWKMapEditCmdDelAll") {
            map.entries = vector[];
        } else if (variant_type_name == b"0x1::jwks::JWKMapEditCmdDelIssuer") {
            let cmd = copyable_any::unpack<JWKMapEditCmdDelIssuer>(edit.content);
            let (found, index) = vector::find(&map.entries, |obj| {
                let provider_jwk_set: &ProviderJWKSet = obj;
                provider_jwk_set.issuer == cmd.issuer
            });
            if (found) {
                vector::swap_remove(&mut map.entries, index);
            };
        } else if (variant_type_name == b"0x1::jwks::JWKMapEditCmdDelJwk") {
            let cmd = copyable_any::unpack<JWKMapEditCmdDelJwk>(edit.content);
            let existing_jwk_set = update_jwk_map(map, cmd.issuer, option::none());
            if (option::is_some(&existing_jwk_set)) {
                let jwk_set = option::extract(&mut existing_jwk_set);
                update_jwk_set(&mut jwk_set, cmd.jwk_id, option::none());
                update_jwk_map(map, cmd.issuer, option::some(jwk_set));
            };
        } else if (variant_type_name == b"0x1::jwks::JWKMapEditCmdPutJwk") {
            let cmd = copyable_any::unpack<JWKMapEditCmdPutJwk>(edit.content);
            let existing_jwk_set = update_jwk_map(map, cmd.issuer, option::none());
            let jwk_set = if (option::is_some(&existing_jwk_set)) {
                option::extract(&mut existing_jwk_set)
            } else {
                ProviderJWKSet {
                    issuer: cmd.issuer,
                    jwks: vector[],
                }
            };
            update_jwk_set(&mut jwk_set, cmd.jwk.id, option::some(cmd.jwk));
            update_jwk_map(map, jwk_set.issuer, option::some(jwk_set));
        } else {
            abort(std::error::invalid_argument(EUNKNOWN_JWK_MAP_EDIT))
        }
    }

    #[test]
    fun test_jwk_map_apply_edit() {
        let jwk_map = JWKMap {
            entries: vector[
                ProviderJWKSet {
                    issuer: b"alice",
                    jwks: vector[
                        jwk_rsa(RSA_JWK {
                            kid: utf8(b"e4adfb436b9e197e2e1106af2c842284e4986aff"),
                            kty: utf8(b"RSA"),
                            alg: utf8(b"RS256"),
                            e: utf8(b"AQAB"),
                            n: utf8(b"psply8S991RswM0JQJwv51fooFFvZUtYdL8avyKObshyzj7oJuJD8vkf5DKJJF1XOGi6Wv2D-U4b3htgrVXeOjAvaKTYtrQVUG_Txwjebdm2EvBJ4R6UaOULjavcSkb8VzW4l4AmP_yWoidkHq8n6vfHt9alDAONILi7jPDzRC7NvnHQ_x0hkRVh_OAmOJCpkgb0gx9-U8zSBSmowQmvw15AZ1I0buYZSSugY7jwNS2U716oujAiqtRkC7kg4gPouW_SxMleeo8PyRsHpYCfBME66m-P8Zr9Fh1Qgmqg4cWdy_6wUuNc1cbVY_7w1BpHZtZCNeQ56AHUgUFmo2LAQQ"),
                        }),
                        jwk_unsupported(b"key_id_0", b"key_content_0"),
                    ],
                },
                ProviderJWKSet {
                    issuer: b"bob",
                    jwks: vector[
                        jwk_unsupported(b"key_id_1", b"key_content_1"),
                        jwk_unsupported(b"key_id_2", b"key_content_2"),
                    ],
                },
            ],
        };

        let edit = jwk_map_edit_del_issuer(b"alice");
        apply_edit_to_jwk_map(&mut jwk_map, edit);
        assert!(jwk_map == JWKMap {
            entries: vector[
                ProviderJWKSet {
                    issuer: b"bob",
                    jwks: vector[
                        jwk_unsupported(b"key_id_1", b"key_content_1"),
                        jwk_unsupported(b"key_id_2", b"key_content_2"),
                    ],
                },
            ],
        }, 1);

        let edit = jwk_map_edit_del_jwk(b"bob", b"key_id_1");
        apply_edit_to_jwk_map(&mut jwk_map, edit);
        assert!(jwk_map == JWKMap {
            entries: vector[
                ProviderJWKSet {
                    issuer: b"bob",
                    jwks: vector[
                        jwk_unsupported(b"key_id_2", b"key_content_2"),
                    ],
                },
            ],
        }, 1);

        let edit = jwk_map_edit_put_jwk(b"carl", jwk_rsa(RSA_JWK {
            kid: utf8(b"0ad1fec78504f447bae65bcf5afaedb65eec9e81"),
            kty: utf8(b"RSA"),
            alg: utf8(b"RS256"),
            e: utf8(b"AQAB"),
            n: utf8(b"sm72oBH-R2Rqt4hkjp66tz5qCtq42TMnVgZg2Pdm_zs7_-EoFyNs9sD1MKsZAFaBPXBHDiWywyaHhLgwETLN9hlJIZPzGCEtV3mXJFSYG-8L6t3kyKi9X1lUTZzbmNpE0tf-eMW-3gs3VQSBJQOcQnuiANxbSXwS3PFmi173C_5fDSuC1RoYGT6X3JqLc3DWUmBGucuQjPaUF0w6LMqEIy0W_WYbW7HImwANT6dT52T72md0JWZuAKsRRnRr_bvaUX8_e3K8Pb1K_t3dD6WSLvtmEfUnGQgLynVl3aV5sRYC0Hy_IkRgoxl2fd8AaZT1X_rdPexYpx152Pl_CHJ79Q"),
        }));
        apply_edit_to_jwk_map(&mut jwk_map, edit);
        let edit = jwk_map_edit_put_jwk(b"bob", jwk_unsupported(b"key_id_2", b"key_content_2b"));
        apply_edit_to_jwk_map(&mut jwk_map, edit);
        let edit = jwk_map_edit_put_jwk(b"alice", jwk_unsupported(b"key_id_3", b"key_content_3"));
        apply_edit_to_jwk_map(&mut jwk_map, edit);
        let edit = jwk_map_edit_put_jwk(b"alice", jwk_unsupported(b"key_id_0", b"key_content_0b"));
        apply_edit_to_jwk_map(&mut jwk_map, edit);
        assert!(jwk_map == JWKMap {
            entries: vector[
                ProviderJWKSet {
                    issuer: b"alice",
                    jwks: vector[
                        jwk_unsupported(b"key_id_0", b"key_content_0b"),
                        jwk_unsupported(b"key_id_3", b"key_content_3"),
                    ],
                },
                ProviderJWKSet {
                    issuer: b"bob",
                    jwks: vector[
                        jwk_unsupported(b"key_id_2", b"key_content_2b"),
                    ],
                },
                ProviderJWKSet {
                    issuer: b"carl",
                    jwks: vector[
                        jwk_rsa(RSA_JWK {
                            kid: utf8(b"0ad1fec78504f447bae65bcf5afaedb65eec9e81"),
                            kty: utf8(b"RSA"),
                            alg: utf8(b"RS256"),
                            e: utf8(b"AQAB"),
                            n: utf8(b"sm72oBH-R2Rqt4hkjp66tz5qCtq42TMnVgZg2Pdm_zs7_-EoFyNs9sD1MKsZAFaBPXBHDiWywyaHhLgwETLN9hlJIZPzGCEtV3mXJFSYG-8L6t3kyKi9X1lUTZzbmNpE0tf-eMW-3gs3VQSBJQOcQnuiANxbSXwS3PFmi173C_5fDSuC1RoYGT6X3JqLc3DWUmBGucuQjPaUF0w6LMqEIy0W_WYbW7HImwANT6dT52T72md0JWZuAKsRRnRr_bvaUX8_e3K8Pb1K_t3dD6WSLvtmEfUnGQgLynVl3aV5sRYC0Hy_IkRgoxl2fd8AaZT1X_rdPexYpx152Pl_CHJ79Q"),
                        })
                    ],
                },
            ],
        }, 1);

        let edit = jwk_map_edit_del_all();
        apply_edit_to_jwk_map(&mut jwk_map, edit);
        assert!(jwk_map == JWKMap { entries: vector[] }, 1);
    }
}
