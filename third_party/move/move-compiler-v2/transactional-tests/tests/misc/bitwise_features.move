//# publish
module 0x42::TestFeatures {
    use std::vector;

    /// The enabled features, represented by a bitset stored on chain.
    struct Features has key {
        features: vector<u8>,
    }

    /// Helper to check whether a feature flag is enabled.
    fun contains(features: &vector<u8>, feature: u64): bool {
        let byte_index = feature / 8;
        let bit_mask = 1 << ((feature % 8) as u8);
        byte_index < vector::length(features) && ((*vector::borrow(features, byte_index))) & bit_mask != 0
    }

    fun set(features: &mut vector<u8>, feature: u64, include: bool) {
        let old_n = vector::length(features); // ghost var
        let _old_features = *features; // ghost var
        let n = old_n; // ghost var

        let byte_index = feature / 8;
        let bit_mask = 1 << ((feature % 8) as u8);
        while ({
            vector::length(features) <= byte_index
        }) {
            vector::push_back(features, 0);
            n = n + 1;
        };
        let entry = vector::borrow_mut(features, byte_index);
        if (include)
            *entry = *entry | bit_mask
        else
            *entry = *entry & (0xff ^ bit_mask)
    }

    public fun enable_feature_flags(enable: vector<u64>) acquires Features {
        let features = &mut borrow_global_mut<Features>(@0xbeef).features;
        let i = 0;
        let n = vector::length(&enable);
        while ({
            i < n
        }) {
            set(features, *vector::borrow(&enable, i), true);
            i = i + 1;
        };
    }

    public fun test(s: signer) acquires Features {
        move_to<Features>(&s, Features { features: vector[1, 2, 3] });
        enable_feature_flags(vector[2]);
        assert!(contains(&borrow_global<Features>(@0xbeef).features, 2), 0);
        assert!(!contains(&borrow_global<Features>(@0xbeef).features, 1), 1);
        assert!(!contains(&borrow_global<Features>(@0xbeef).features, 3), 2);
    }
}

//# run 0x42::TestFeatures::test --signers 0xbeef
