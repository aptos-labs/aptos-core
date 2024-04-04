/// Defines feature flags for Aptos. Those are used in Aptos specific implementations of features in
/// the Move stdlib, the Aptos stdlib, and the Aptos framework.
///
/// ============================================================================================
/// Feature Flag Definitions
///
/// Each feature flag should come with documentation which justifies the need of the flag.
/// Introduction of a new feature flag requires approval of framework owners. Be frugal when
/// introducing new feature flags, as too many can make it hard to understand the code.
///
/// Each feature flag should come with a specification of a lifetime:
///
/// - a *transient* feature flag is only needed until a related code rollout has happened. This
///   is typically associated with the introduction of new native Move functions, and is only used
///   from Move code. The owner of this feature is obliged to remove it once this can be done.
///
/// - a *permanent* feature flag is required to stay around forever. Typically, those flags guard
///   behavior in native code, and the behavior with or without the feature need to be preserved
///   for playback.
///
/// Note that removing a feature flag still requires the function which tests for the feature
/// (like `code_dependency_check_enabled` below) to stay around for compatibility reasons, as it
/// is a public function. However, once the feature flag is disabled, those functions can constantly
/// return true.
module std::features {
    use std::error;
    use std::signer;
    use std::vector;

    const EINVALID_FEATURE: u64 = 1;
    const EAPI_DISABLED: u64 = 2;

    // --------------------------------------------------------------------------------------------
    // Code Publishing

    /// Whether validation of package dependencies is enabled, and the related native function is
    /// available. This is needed because of introduction of a new native function.
    /// Lifetime: transient
    const CODE_DEPENDENCY_CHECK: u64 = 1;

    public fun code_dependency_check_enabled(): bool acquires Features {
        is_enabled(CODE_DEPENDENCY_CHECK)
    }

    /// Whether during upgrade compatibility checking, friend functions should be treated similar like
    /// private functions.
    /// Lifetime: permanent
    const TREAT_FRIEND_AS_PRIVATE: u64 = 2;

    public fun treat_friend_as_private(): bool acquires Features {
        is_enabled(TREAT_FRIEND_AS_PRIVATE)
    }

    /// Whether the new SHA2-512, SHA3-512 and RIPEMD-160 hash function natives are enabled.
    /// This is needed because of the introduction of new native functions.
    /// Lifetime: transient
    const SHA_512_AND_RIPEMD_160_NATIVES: u64 = 3;
    public fun get_sha_512_and_ripemd_160_feature(): u64 { SHA_512_AND_RIPEMD_160_NATIVES }
    public fun sha_512_and_ripemd_160_enabled(): bool acquires Features {
        is_enabled(SHA_512_AND_RIPEMD_160_NATIVES)
    }

    /// Whether the new `aptos_stdlib::type_info::chain_id()` native for fetching the chain ID is enabled.
    /// This is needed because of the introduction of a new native function.
    /// Lifetime: transient
    const APTOS_STD_CHAIN_ID_NATIVES: u64 = 4;
    public fun get_aptos_stdlib_chain_id_feature(): u64 { APTOS_STD_CHAIN_ID_NATIVES }
    public fun aptos_stdlib_chain_id_enabled(): bool acquires Features {
        is_enabled(APTOS_STD_CHAIN_ID_NATIVES)
    }

    /// Whether to allow the use of binary format version v6.
    /// Lifetime: transient
    const VM_BINARY_FORMAT_V6: u64 = 5;
    public fun get_vm_binary_format_v6(): u64 { VM_BINARY_FORMAT_V6 }
    public fun allow_vm_binary_format_v6(): bool acquires Features {
        is_enabled(VM_BINARY_FORMAT_V6)
    }

    /// Whether gas fees are collected and distributed to the block proposers.
    /// Lifetime: transient
    const COLLECT_AND_DISTRIBUTE_GAS_FEES: u64 = 6;
    public fun get_collect_and_distribute_gas_fees_feature(): u64 { COLLECT_AND_DISTRIBUTE_GAS_FEES }
    public fun collect_and_distribute_gas_fees(): bool acquires Features {
        is_enabled(COLLECT_AND_DISTRIBUTE_GAS_FEES)
    }

    /// Whether the new `aptos_stdlib::multi_ed25519::public_key_validate_internal_v2()` native is enabled.
    /// This is needed because of the introduction of a new native function.
    /// Lifetime: transient
    const MULTI_ED25519_PK_VALIDATE_V2_NATIVES: u64 = 7;
    public fun multi_ed25519_pk_validate_v2_feature(): u64 { MULTI_ED25519_PK_VALIDATE_V2_NATIVES }
    public fun multi_ed25519_pk_validate_v2_enabled(): bool acquires Features {
        is_enabled(MULTI_ED25519_PK_VALIDATE_V2_NATIVES)
    }

    /// Whether the new BLAKE2B-256 hash function native is enabled.
    /// This is needed because of the introduction of new native function(s).
    /// Lifetime: transient
    const BLAKE2B_256_NATIVE: u64 = 8;
    public fun get_blake2b_256_feature(): u64 { BLAKE2B_256_NATIVE }
    public fun blake2b_256_enabled(): bool acquires Features {
        is_enabled(BLAKE2B_256_NATIVE)
    }

    /// Whether resource groups are enabled.
    /// This is needed because of new attributes for structs and a change in storage representation.
    const RESOURCE_GROUPS: u64 = 9;
    public fun get_resource_groups_feature(): u64 { RESOURCE_GROUPS }
    public fun resource_groups_enabled(): bool acquires Features {
        is_enabled(RESOURCE_GROUPS)
    }

    /// Whether multisig accounts (different from accounts with multi-ed25519 auth keys) are enabled.
    const MULTISIG_ACCOUNTS: u64 = 10;
    public fun get_multisig_accounts_feature(): u64 { MULTISIG_ACCOUNTS }
    public fun multisig_accounts_enabled(): bool acquires Features {
        is_enabled(MULTISIG_ACCOUNTS)
    }

    /// Whether delegation pools are enabled.
    /// Lifetime: transient
    const DELEGATION_POOLS: u64 = 11;
    public fun get_delegation_pools_feature(): u64 { DELEGATION_POOLS }
    public fun delegation_pools_enabled(): bool acquires Features {
        is_enabled(DELEGATION_POOLS)
    }

    /// Whether generic algebra basic operation support in `crypto_algebra.move` are enabled.
    ///
    /// Lifetime: transient
    const CRYPTOGRAPHY_ALGEBRA_NATIVES: u64 = 12;

    public fun get_cryptography_algebra_natives_feature(): u64 { CRYPTOGRAPHY_ALGEBRA_NATIVES }

    public fun cryptography_algebra_enabled(): bool acquires Features {
        is_enabled(CRYPTOGRAPHY_ALGEBRA_NATIVES)
    }

    /// Whether the generic algebra implementation for BLS12381 operations are enabled.
    ///
    /// Lifetime: transient
    const BLS12_381_STRUCTURES: u64 = 13;

    public fun get_bls12_381_strutures_feature(): u64 { BLS12_381_STRUCTURES }

    public fun bls12_381_structures_enabled(): bool acquires Features {
        is_enabled(BLS12_381_STRUCTURES)
    }


    /// Whether native_public_key_validate aborts when a public key of the wrong length is given
    /// Lifetime: ephemeral
    const ED25519_PUBKEY_VALIDATE_RETURN_FALSE_WRONG_LENGTH: u64 = 14;

    /// Whether struct constructors are enabled
    ///
    /// Lifetime: transient
    const STRUCT_CONSTRUCTORS: u64 = 15;

    /// Whether reward rate decreases periodically.
    /// Lifetime: transient
    const PERIODICAL_REWARD_RATE_DECREASE: u64 = 16;

    public fun get_periodical_reward_rate_decrease_feature(): u64 { PERIODICAL_REWARD_RATE_DECREASE }

    public fun periodical_reward_rate_decrease_enabled(): bool acquires Features {
        is_enabled(PERIODICAL_REWARD_RATE_DECREASE)
    }

    /// Whether enable paritial governance voting on aptos_governance.
    /// Lifetime: transient
    const PARTIAL_GOVERNANCE_VOTING: u64 = 17;

    public fun get_partial_governance_voting(): u64 { PARTIAL_GOVERNANCE_VOTING }

    public fun partial_governance_voting_enabled(): bool acquires Features {
        is_enabled(PARTIAL_GOVERNANCE_VOTING)
    }

    /// Charge invariant violation error.
    /// Lifetime: transient
    const CHARGE_INVARIANT_VIOLATION: u64 = 20;

    /// Whether enable paritial governance voting on delegation_pool.
    /// Lifetime: transient
    const DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING: u64 = 21;

    public fun get_delegation_pool_partial_governance_voting(): u64 { DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING }

    public fun delegation_pool_partial_governance_voting_enabled(): bool acquires Features {
        is_enabled(DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING)
    }

    /// Whether alternate gas payer is supported
    /// Lifetime: transient
    const FEE_PAYER_ENABLED: u64 = 22;

    public fun fee_payer_enabled(): bool acquires Features {
        is_enabled(FEE_PAYER_ENABLED)
    }

    /// Whether enable MOVE functions to call create_auid method to create AUIDs.
    /// Lifetime: transient
    const APTOS_UNIQUE_IDENTIFIERS: u64 = 23;

    public fun get_auids(): u64 { APTOS_UNIQUE_IDENTIFIERS }

    public fun auids_enabled(): bool acquires Features {
        is_enabled(APTOS_UNIQUE_IDENTIFIERS)
    }

    /// Whether the Bulletproofs zero-knowledge range proof module is enabled, and the related native function is
    /// available. This is needed because of the introduction of a new native function.
    /// Lifetime: transient
    const BULLETPROOFS_NATIVES: u64 = 24;
    public fun get_bulletproofs_feature(): u64 { BULLETPROOFS_NATIVES }

    public fun bulletproofs_enabled(): bool acquires Features {
        is_enabled(BULLETPROOFS_NATIVES)
    }

    /// Fix the native formatter for signer.
    /// Lifetime: transient
    const SIGNER_NATIVE_FORMAT_FIX: u64 = 25;

    public fun get_signer_native_format_fix_feature(): u64 { SIGNER_NATIVE_FORMAT_FIX }

    public fun signer_native_format_fix_enabled(): bool acquires Features {
        is_enabled(SIGNER_NATIVE_FORMAT_FIX)
    }

    /// Whether emit function in `event.move` are enabled for module events.
    ///
    /// Lifetime: transient
    const MODULE_EVENT: u64 = 26;

    public fun get_module_event_feature(): u64 { MODULE_EVENT }

    public fun module_event_enabled(): bool acquires Features {
        is_enabled(MODULE_EVENT)
    }

    /// Whether the fix for a counting bug in the script path of the signature checker pass is enabled.
    /// Lifetime: transient
    const SIGNATURE_CHECKER_V2_SCRIPT_FIX: u64 = 29;

    /// Whether the Aggregator V2 API feature is enabled.
    /// Once enabled, the functions from aggregator_v2.move will be available for use.
    /// Lifetime: transient
    const AGGREGATOR_V2_API: u64 = 30;

    public fun get_aggregator_v2_api_feature(): u64 { AGGREGATOR_V2_API }

    public fun aggregator_v2_api_enabled(): bool acquires Features {
        is_enabled(AGGREGATOR_V2_API)
    }

    #[deprecated]
    public fun get_aggregator_snapshots_feature(): u64 {
        abort error::invalid_argument(EINVALID_FEATURE)
    }

    #[deprecated]
    public fun aggregator_snapshots_enabled(): bool {
        abort error::invalid_argument(EINVALID_FEATURE)
    }

    const SAFER_RESOURCE_GROUPS: u64 = 31;

    const SAFER_METADATA: u64 = 32;

    const SINGLE_SENDER_AUTHENTICATOR: u64 = 33;

    /// Whether the automatic creation of accounts is enabled for sponsored transactions.
    /// Lifetime: transient
    const SPONSORED_AUTOMATIC_ACCOUNT_CREATION: u64 = 34;

    public fun get_sponsored_automatic_account_creation(): u64 { SPONSORED_AUTOMATIC_ACCOUNT_CREATION }

    public fun sponsored_automatic_account_creation_enabled(): bool acquires Features {
        is_enabled(SPONSORED_AUTOMATIC_ACCOUNT_CREATION)
    }

    const FEE_PAYER_ACCOUNT_OPTIONAL: u64 = 35;

    /// Whether the Aggregator V2 delayed fields feature is enabled.
    /// Once enabled, Aggregator V2 functions become parallel.
    /// Lifetime: transient
    const AGGREGATOR_V2_DELAYED_FIELDS: u64 = 36;

    /// Whether enable TokenV2 collection creation and Fungible Asset creation
    /// to create higher throughput concurrent variants.
    /// Lifetime: transient
    const CONCURRENT_TOKEN_V2: u64 = 37;

    public fun get_concurrent_token_v2_feature(): u64 { CONCURRENT_TOKEN_V2 }

    public fun concurrent_token_v2_enabled(): bool acquires Features {
        // concurrent token v2 cannot be used if aggregator v2 api is not enabled.
        is_enabled(CONCURRENT_TOKEN_V2) && aggregator_v2_api_enabled()
    }

    #[deprecated]
    public fun get_concurrent_assets_feature(): u64 {
        abort error::invalid_argument(EINVALID_FEATURE)
    }

    #[deprecated]
    public fun concurrent_assets_enabled(): bool {
        abort error::invalid_argument(EINVALID_FEATURE)
    }

    const LIMIT_MAX_IDENTIFIER_LENGTH: u64 = 38;

    /// Whether allow changing beneficiaries for operators.
    /// Lifetime: transient
    const OPERATOR_BENEFICIARY_CHANGE: u64 = 39;

    public fun get_operator_beneficiary_change_feature(): u64 { OPERATOR_BENEFICIARY_CHANGE }

    public fun operator_beneficiary_change_enabled(): bool acquires Features {
        is_enabled(OPERATOR_BENEFICIARY_CHANGE)
    }

    const VM_BINARY_FORMAT_V7: u64 = 40;

    const RESOURCE_GROUPS_SPLIT_IN_VM_CHANGE_SET: u64 = 41;

    /// Whether the operator commission rate change in delegation pool is enabled.
    /// Lifetime: transient
    const COMMISSION_CHANGE_DELEGATION_POOL: u64 = 42;

    public fun get_commission_change_delegation_pool_feature(): u64 { COMMISSION_CHANGE_DELEGATION_POOL }

    public fun commission_change_delegation_pool_enabled(): bool acquires Features {
        is_enabled(COMMISSION_CHANGE_DELEGATION_POOL)
    }

    /// Whether the generic algebra implementation for BN254 operations are enabled.
    ///
    /// Lifetime: transient
    const BN254_STRUCTURES: u64 = 43;

    public fun get_bn254_strutures_feature(): u64 { BN254_STRUCTURES }

    public fun bn254_structures_enabled(): bool acquires Features {
        is_enabled(BN254_STRUCTURES)
    }

    /// Deprecated by `aptos_framework::randomness_config::RandomnessConfig`.
    const RECONFIGURE_WITH_DKG: u64 = 45;
    public fun get_reconfigure_with_dkg_feature(): u64 { RECONFIGURE_WITH_DKG }
    public fun reconfigure_with_dkg_enabled(): bool acquires Features {
        is_enabled(RECONFIGURE_WITH_DKG)
    }

    /// Whether the OIDB feature is enabled, possibly with the ZK-less verification mode.
    ///
    /// Lifetime: transient
    const KEYLESS_ACCOUNTS: u64 = 46;

    public fun get_keyless_accounts_feature(): u64 { KEYLESS_ACCOUNTS }

    public fun keyless_accounts_enabled(): bool acquires Features {
        is_enabled(KEYLESS_ACCOUNTS)
    }

    /// Whether the ZK-less mode of the keyless accounts feature is enabled.
    ///
    /// Lifetime: transient
    const KEYLESS_BUT_ZKLESS_ACCOUNTS: u64 = 47;

    public fun get_keyless_but_zkless_accounts_feature(): u64 { KEYLESS_BUT_ZKLESS_ACCOUNTS }

    public fun keyless_but_zkless_accounts_feature_enabled(): bool acquires Features {
        is_enabled(KEYLESS_BUT_ZKLESS_ACCOUNTS)
    }

    /// Deprecated by `aptos_framework::jwk_consensus_config::JWKConsensusConfig`.
    const JWK_CONSENSUS: u64 = 49;

    public fun get_jwk_consensus_feature(): u64 { JWK_CONSENSUS }

    public fun jwk_consensus_enabled(): bool acquires Features {
        is_enabled(JWK_CONSENSUS)
    }

    /// Whether enable Fungible Asset creation
    /// to create higher throughput concurrent variants.
    /// Lifetime: transient
    const CONCURRENT_FUNGIBLE_ASSETS: u64 = 50;

    public fun get_concurrent_fungible_assets_feature(): u64 { CONCURRENT_FUNGIBLE_ASSETS }

    public fun concurrent_fungible_assets_enabled(): bool acquires Features {
        // concurrent fungible assets cannot be used if aggregator v2 api is not enabled.
        is_enabled(CONCURRENT_FUNGIBLE_ASSETS) && aggregator_v2_api_enabled()
    }

    /// Whether deploying to objects is enabled.
    const OBJECT_CODE_DEPLOYMENT: u64 = 52;

    public fun is_object_code_deployment_enabled(): bool acquires Features {
        is_enabled(OBJECT_CODE_DEPLOYMENT)
    }

    /// Whether checking the maximum object nesting is enabled.
    const MAX_OBJECT_NESTING_CHECK: u64 = 53;

    public fun get_max_object_nesting_check_feature(): u64 { MAX_OBJECT_NESTING_CHECK }

    public fun max_object_nesting_check_enabled(): bool acquires Features {
        is_enabled(MAX_OBJECT_NESTING_CHECK)
    }

    /// Whether keyless accounts support passkey-based ephemeral signatures.
    ///
    /// Lifetime: transient
    const KEYLESS_ACCOUNTS_WITH_PASSKEYS: u64 = 54;

    public fun get_keyless_accounts_with_passkeys_feature(): u64 { KEYLESS_ACCOUNTS_WITH_PASSKEYS }

    public fun keyless_accounts_with_passkeys_feature_enabled(): bool acquires Features {
        is_enabled(KEYLESS_ACCOUNTS_WITH_PASSKEYS)
    }

    /// Whether the Multisig V2 enhancement feature is enabled.
    ///
    /// Lifetime: transient
    const MULTISIG_V2_ENHANCEMENT: u64 = 55;

    public fun get_multisig_v2_enhancement_feature(): u64 { MULTISIG_V2_ENHANCEMENT }

    public fun multisig_v2_enhancement_feature_enabled(): bool acquires Features {
        is_enabled(MULTISIG_V2_ENHANCEMENT)
    }

    /// Whether delegators allowlisting for delegation pools is supported.
    /// Lifetime: transient
    const DELEGATION_POOL_ALLOWLISTING: u64 = 56;

    public fun get_delegation_pool_allowlisting_feature(): u64 { DELEGATION_POOL_ALLOWLISTING }

    public fun delegation_pool_allowlisting_enabled(): bool acquires Features {
        is_enabled(DELEGATION_POOL_ALLOWLISTING)
    }

    // ============================================================================================
    // Feature Flag Implementation

    /// The provided signer has not a framework address.
    const EFRAMEWORK_SIGNER_NEEDED: u64 = 1;

    /// The enabled features, represented by a bitset stored on chain.
    struct Features has key {
        features: vector<u8>,
    }

    /// This resource holds the feature vec updates received in the current epoch.
    /// On epoch change, the updates take effect and this buffer is cleared.
    struct PendingFeatures has key {
        features: vector<u8>,
    }

    /// Deprecated to prevent validator set changes during DKG.
    ///
    /// Genesis/tests should use `change_feature_flags_internal()` for feature vec initialization.
    ///
    /// Governance proposals should use `change_feature_flags_for_next_epoch()` to enable/disable features.
    public fun change_feature_flags(_framework: &signer, _enable: vector<u64>, _disable: vector<u64>) {
        abort(error::invalid_state(EAPI_DISABLED))
    }

    /// Update feature flags directly. Only used in genesis/tests.
    fun change_feature_flags_internal(framework: &signer, enable: vector<u64>, disable: vector<u64>) acquires Features {
        assert!(signer::address_of(framework) == @std, error::permission_denied(EFRAMEWORK_SIGNER_NEEDED));
        if (!exists<Features>(@std)) {
            move_to<Features>(framework, Features { features: vector[] })
        };
        let features = &mut borrow_global_mut<Features>(@std).features;
        vector::for_each_ref(&enable, |feature| {
            set(features, *feature, true);
        });
        vector::for_each_ref(&disable, |feature| {
            set(features, *feature, false);
        });
    }

    /// Enable and disable features for the next epoch.
    public fun change_feature_flags_for_next_epoch(framework: &signer, enable: vector<u64>, disable: vector<u64>) acquires PendingFeatures, Features {
        assert!(signer::address_of(framework) == @std, error::permission_denied(EFRAMEWORK_SIGNER_NEEDED));

        // Figure out the baseline feature vec that the diff will be applied to.
        let new_feature_vec = if (exists<PendingFeatures>(@std)) {
            // If there is a buffered feature vec, use it as the baseline.
            let PendingFeatures { features } = move_from<PendingFeatures>(@std);
            features
        } else if (exists<Features>(@std)) {
            // Otherwise, use the currently effective feature flag vec as the baseline, if it exists.
            borrow_global<Features>(@std).features
        } else {
            // Otherwise, use an empty feature vec.
            vector[]
        };

        // Apply the diff and save it to the buffer.
        apply_diff(&mut new_feature_vec, enable, disable);
        move_to(framework, PendingFeatures { features: new_feature_vec });
    }

    /// Apply all the pending feature flag changes. Should only be used at the end of a reconfiguration with DKG.
    ///
    /// While the scope is public, it can only be usd in system transactions like `block_prologue` and governance proposals,
    /// who have permission to set the flag that's checked in `extract()`.
    public fun on_new_epoch(vm_or_framework: &signer) acquires Features, PendingFeatures {
        ensure_vm_or_framework_signer(vm_or_framework);
        if (exists<PendingFeatures>(@std)) {
            let PendingFeatures { features } = move_from<PendingFeatures>(@std);
            borrow_global_mut<Features>(@std).features = features;
        }
    }

    #[view]
    /// Check whether the feature is enabled.
    public fun is_enabled(feature: u64): bool acquires Features {
        exists<Features>(@std) &&
            contains(&borrow_global<Features>(@std).features, feature)
    }

    /// Helper to include or exclude a feature flag.
    fun set(features: &mut vector<u8>, feature: u64, include: bool) {
        let byte_index = feature / 8;
        let bit_mask = 1 << ((feature % 8) as u8);
        while (vector::length(features) <= byte_index) {
            vector::push_back(features, 0)
        };
        let entry = vector::borrow_mut(features, byte_index);
        if (include)
            *entry = *entry | bit_mask
        else
            *entry = *entry & (0xff ^ bit_mask)
    }

    /// Helper to check whether a feature flag is enabled.
    fun contains(features: &vector<u8>, feature: u64): bool {
        let byte_index = feature / 8;
        let bit_mask = 1 << ((feature % 8) as u8);
        byte_index < vector::length(features) && (*vector::borrow(features, byte_index) & bit_mask) != 0
    }

    fun apply_diff(features: &mut vector<u8>, enable: vector<u64>, disable: vector<u64>) {
        vector::for_each(enable, |feature| {
            set(features, feature, true);
        });
        vector::for_each(disable, |feature| {
            set(features, feature, false);
        });
    }

    fun ensure_vm_or_framework_signer(account: &signer) {
        let addr = signer::address_of(account);
        assert!(addr == @std || addr == @vm, error::permission_denied(EFRAMEWORK_SIGNER_NEEDED));
    }

    #[verify_only]
    public fun change_feature_flags_for_verification(framework: &signer, enable: vector<u64>, disable: vector<u64>) acquires Features {
        change_feature_flags_internal(framework, enable, disable)
    }

    #[test_only]
    public fun change_feature_flags_for_testing(framework: &signer, enable: vector<u64>, disable: vector<u64>) acquires Features {
        change_feature_flags_internal(framework, enable, disable)
    }

    #[test]
    fun test_feature_sets() {
        let features = vector[];
        set(&mut features, 1, true);
        set(&mut features, 5, true);
        set(&mut features, 17, true);
        set(&mut features, 23, true);
        assert!(contains(&features, 1), 0);
        assert!(contains(&features, 5), 1);
        assert!(contains(&features, 17), 2);
        assert!(contains(&features, 23), 3);
        set(&mut features, 5, false);
        set(&mut features, 17, false);
        assert!(contains(&features, 1), 0);
        assert!(!contains(&features, 5), 1);
        assert!(!contains(&features, 17), 2);
        assert!(contains(&features, 23), 3);
    }

    #[test(fx = @std)]
    fun test_change_feature_txn(fx: signer) acquires Features {
        change_feature_flags_for_testing(&fx, vector[1, 9, 23], vector[]);
        assert!(is_enabled(1), 1);
        assert!(is_enabled(9), 2);
        assert!(is_enabled(23), 3);
        change_feature_flags_for_testing(&fx, vector[17], vector[9]);
        assert!(is_enabled(1), 1);
        assert!(!is_enabled(9), 2);
        assert!(is_enabled(17), 3);
        assert!(is_enabled(23), 4);
    }
}
