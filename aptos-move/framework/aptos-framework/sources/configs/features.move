/// Maintains feature flags.
module aptos_framework::features {
    use aptos_framework::system_addresses;
    use std::vector;

    // ============================================================================================
    // Feature Flag Definitions

    // Each feature flag should come with documentation which justifies the need of the flag.
    // Introduction of a new feature flag requires approval of framework owners. Be frugal when
    // introducing new feature flags, as too many can make it hard to understand the code.
    //
    // Each feature flag should come with a specification of a lifetime:
    //
    // - a *transient* feature flag is only needed until a related code rollout has happened. This
    //   is typically associated with the introduction of new native Move functions, and is only used
    //   from Move code. The owner of this feature is obliged to remove it once this can be done.
    //
    // - an *ephemeral* feature flag is required to stay around forever. Typically, those flags guard
    //   behavior in native code, and the behavior with or without the feature need to be preserved
    //   for playback.
    //
    // Features should be grouped along components. Their name should start with the module or
    // component name they are associated with.


    // --------------------------------------------------------------------------------------------
    // Code Publishing

    /// Whether validation of package dependencies is enabled, and the related native function is
    /// available. This is needed because of introduction of a new native function.
    /// Lifetime: transient
    const CODE_DEPENDENCY_CHECK: u64 = 1;
    public(friend) fun code_dependency_check_enabled(): bool acquires Features {
        is_enabled(CODE_DEPENDENCY_CHECK)
    }
    friend aptos_framework::code;

    /// Whether during upgrade compatibility checking, friend functions should be treated similar like
    /// private functions.
    /// Lifetime: ephemeral
    const TREAT_FRIEND_AS_PRIVATE: u64 = 2;
    public(friend) fun treat_friend_as_private(): bool acquires Features {
        is_enabled(TREAT_FRIEND_AS_PRIVATE)
    }

    // ============================================================================================
    // Feature Flag  Management

    /// The enabled features, represented by a bitset stored on chain.
    struct Features has key {
        features: vector<u8>,
    }

    /// Function to enable and disable features. Can only be called by @aptos_framework.
    public fun change_feature_flags(aptos_framework: &signer, enable: vector<u64>, disable: vector<u64>)
    acquires Features {
        system_addresses::assert_aptos_framework(aptos_framework);
        if (!exists<Features>(@aptos_framework)) {
            move_to<Features>(aptos_framework, Features{features: vector[]})
        };
        let features = &mut borrow_global_mut<Features>(@aptos_framework).features;
        let i = 0;
        let n = vector::length(&enable);
        while (i < n) {
            set(features, *vector::borrow(&enable, i), true);
            i = i + 1
        };
        let i = 0;
        let n = vector::length(&disable);
        while (i < n) {
            set(features, *vector::borrow(&disable, i), false);
            i = i + 1
        };
    }

    /// Check whether the feature is enabled.
    fun is_enabled(feature: u64): bool acquires Features {
        exists<Features>(@aptos_framework) &&
        contains(&borrow_global<Features>(@aptos_framework).features, feature)
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

    #[test(fx = @aptos_framework)]
    fun test_change_feature_txn(fx: signer) acquires Features {
        change_feature_flags(&fx, vector[1, 9, 23], vector[]);
        assert!(is_enabled(1), 1);
        assert!(is_enabled(9), 2);
        assert!(is_enabled(23), 3);
        change_feature_flags(&fx, vector[17], vector[9]);
        assert!(is_enabled(1), 1);
        assert!(!is_enabled(9), 2);
        assert!(is_enabled(17), 3);
        assert!(is_enabled(23), 4);
    }
}
