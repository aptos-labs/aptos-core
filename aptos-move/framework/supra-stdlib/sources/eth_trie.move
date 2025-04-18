module supra_std::eth_trie {

    use std::features;
    use std::vector;

    /// SUPRA_ETH_TRIE feature APIs are disabled.
    const EETH_TRIE_FEATURE_DISABLED: u64 = 1;

    /// Public wrapper function that calls the native and returns a bool.
    /// Returns true if the inclusion proof is valid i.e. the value exists in the tree
    /// Also returns the value corresponding to the key
    public fun verify_eth_trie_inclusion_proof(
        root: vector<u8>,
        key: vector<u8>,
        proof: vector<vector<u8>>
    ): (bool, vector<u8>) {
        let (proof_is_valid, value) = verify_proof_eth_trie(root, key, proof);
        (proof_is_valid && !vector::is_empty(&value), value)
    }

    /// Public wrapper function that calls the native and returns a bool.
    /// Returns true if the exclusion proof is valid i.e. the value does not exist in the tree
    public fun verify_eth_trie_exclusion_proof(
        root: vector<u8>,
        key: vector<u8>,
        proof: vector<vector<u8>>
    ): bool {
        let (proof_is_valid, value) = verify_proof_eth_trie(root, key, proof);
        proof_is_valid && vector::is_empty(&value)
    }

    /// Public wrapper function that calls the native and returns status and the possible extracted value.
    /// Note: no inclusion or exclusion checks are done
    public fun verify_proof_eth_trie(
        root: vector<u8>,
        key: vector<u8>,
        proof: vector<vector<u8>>
    ): (bool, vector<u8>) {
        assert!(features::supra_eth_trie_enabled(), EETH_TRIE_FEATURE_DISABLED);
        native_verify_proof_eth_trie(root, key, proof)
    }

    //
    // Native functions
    //
    native fun native_verify_proof_eth_trie(
        root: vector<u8>,
        key: vector<u8>,
        proof: vector<vector<u8>>
    ): (bool, vector<u8>);

    #[test_only]
    native fun generate_random_trie(num_keys: u64): (vector<u8>, vector<vector<vector<u8>>>);

    /////////////////////////
    // Test functions
    /////////////////////////

    #[test_only]
    fun prepare_env(supra_framework: &signer) {
        let flag = vector[features::get_supra_eth_trie_feature()];
        features::change_feature_flags_for_testing(
            supra_framework, flag, vector::empty<u64>()
        );
    }

    #[test(supra_framework = @supra_framework)]
    public fun test_proof_basic(supra_framework: signer) {
        prepare_env(&supra_framework);

        // These constants must match the values computed by your trie.
        // For example, suppose the trie built with:
        //   "doe" -> "reindeer"
        //   "dog" -> "puppy"
        //   "dogglesworth" -> "cat"
        // produces the following root:
        // 0x8aad789dff2f538bca5d8ea56e8abe10f4c7ba3a5dea95fea4cd6e7c3a1168d3
        let root: vector<u8> = vector[
            0x8a, 0xad, 0x78, 0x9d, 0xff, 0x2f, 0x53, 0x8b, 0xca, 0x5d, 0x8e, 0xa5, 0x6e,
            0x8a, 0xbe, 0x10, 0xf4, 0xc7, 0xba, 0x3a, 0x5d, 0xea, 0x95, 0xfe, 0xa4, 0xcd,
            0x6e, 0x7c, 0x3a, 0x11, 0x68, 0xd3
        ];

        // Key "doe"
        let key: vector<u8> = b"doe";

        // Proof nodes (the exact bytes must match the output from Rust trie).
        // Here we hardcode two proof nodes.
        let proof: vector<vector<u8>> = vector::empty();
        // For example, the first proof node corresponding to hex:
        // "e5831646f6a0db6ae1fda66890f6693f36560d36b4dca68b4d838f17016b151efe1d4c95c453"
        let node1: vector<u8> = vector[
            0xe5, 0x83, 0x16, 0x46, 0xf6, 0xa0, 0xdb, 0x6a, 0xe1, 0xfd, 0xa6, 0x68, 0x90,
            0xf6, 0x69, 0x3f, 0x36, 0x56, 0x0d, 0x36, 0xb4, 0xdc, 0xa6, 0x8b, 0x4d, 0x83,
            0x8f, 0x17, 0x01, 0x6b, 0x15, 0x1e, 0xfe, 0x1d, 0x4c, 0x95, 0xc4, 0x53
        ];
        // And the second proof node corresponding to hex:
        // "f83b8080808080ca20887265696e6465657280a037efd11993cb04a54048c25320e9f29c50a432d28afdf01598b2978ce1ca3068808080808080808080"
        let node2: vector<u8> = vector[
            0xf8, 0x3b, 0x80, 0x80, 0x80, 0x80, 0x80, 0xca, 0x20, 0x88, 0x72, 0x65, 0x69,
            0x6e, 0x64, 0x65, 0x65, 0x72, 0x80, 0xa0, 0x37, 0xef, 0xd1, 0x19, 0x93, 0xcb,
            0x04, 0xa5, 0x40, 0x48, 0xc2, 0x53, 0x20, 0xe9, 0xf2, 0x9c, 0x50, 0xa4, 0x32,
            0xd2, 0x8a, 0xfd, 0xf0, 0x15, 0x98, 0xb2, 0x97, 0x8c, 0xe1, 0xca, 0x30, 0x68,
            0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80
        ];
        vector::push_back(&mut proof, node1);
        vector::push_back(&mut proof, node2);

        let (flag, value) = verify_eth_trie_inclusion_proof(root, key, proof);
        // Expect true since "doe" exists.
        assert!(flag, 1);

        // Also check the value
        let expected_val = b"reindeer";
        assert!(value == expected_val, 2);
    }

    #[test]
    #[expected_failure(abort_code = EETH_TRIE_FEATURE_DISABLED, location = Self)]
    public fun test_proof_inclusion_eth_trie_feature_disabled() {

        let root: vector<u8> = vector[];
        let key: vector<u8> = b"doe";
        let proof: vector<vector<u8>> = vector::empty();

        verify_eth_trie_inclusion_proof(root, key, proof);
    }

    #[test]
    #[expected_failure(abort_code = EETH_TRIE_FEATURE_DISABLED, location = Self)]
    public fun test_proof_exclusion_eth_trie_feature_disabled() {

        let root: vector<u8> = vector[];
        let key: vector<u8> = b"doe";
        let proof: vector<vector<u8>> = vector::empty();

        verify_eth_trie_exclusion_proof(root, key, proof);
    }

    #[test(supra_framework = @supra_framework)]
    public fun test_proof_nonexistent(supra_framework: signer) {
        prepare_env(&supra_framework);

        // Use the same root as before.
        let root: vector<u8> = vector[
            0x8a, 0xad, 0x78, 0x9d, 0xff, 0x2f, 0x53, 0x8b, 0xca, 0x5d, 0x8e, 0xa5, 0x6e,
            0x8a, 0xbe, 0x10, 0xf4, 0xc7, 0xba, 0x3a, 0x5d, 0xea, 0x95, 0xfe, 0xa4, 0xcd,
            0x6e, 0x7c, 0x3a, 0x11, 0x68, 0xd3
        ];
        // Key "dogg" (which does not exist)
        let key: vector<u8> = b"dogg";
        // The expected proof for "dogg" (three nodes in this case).
        let proof: vector<vector<u8>> = vector::empty();
        // "e5831646f6a0db6ae1fda66890f6693f36560d36b4dca68b4d838f17016b151efe1d4c95c453"
        let p1: vector<u8> = vector[
            0xe5, 0x83, 0x16, 0x46, 0xf6, 0xa0, 0xdb, 0x6a, 0xe1, 0xfd, 0xa6, 0x68, 0x90,
            0xf6, 0x69, 0x3f, 0x36, 0x56, 0x0d, 0x36, 0xb4, 0xdc, 0xa6, 0x8b, 0x4d, 0x83,
            0x8f, 0x17, 0x01, 0x6b, 0x15, 0x1e, 0xfe, 0x1d, 0x4c, 0x95, 0xc4, 0x53
        ];
        // "f83b8080808080ca20887265696e6465657280a037efd11993cb04a54048c25320e9f29c50a432d28afdf01598b2978ce1ca3068808080808080808080"
        let p2: vector<u8> = vector[
            0xf8, 0x3b, 0x80, 0x80, 0x80, 0x80, 0x80, 0xca, 0x20, 0x88, 0x72, 0x65, 0x69,
            0x6e, 0x64, 0x65, 0x65, 0x72, 0x80, 0xa0, 0x37, 0xef, 0xd1, 0x19, 0x93, 0xcb,
            0x04, 0xa5, 0x40, 0x48, 0xc2, 0x53, 0x20, 0xe9, 0xf2, 0x9c, 0x50, 0xa4, 0x32,
            0xd2, 0x8a, 0xfd, 0xf0, 0x15, 0x98, 0xb2, 0x97, 0x8c, 0xe1, 0xca, 0x30, 0x68,
            0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80
        ];
        // "e4808080808080ce89376c6573776f72746883636174808080808080808080857075707079"
        let p3: vector<u8> = vector[
            0xe4, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0xce, 0x89, 0x37, 0x6c, 0x65, 0x73,
            0x77, 0x6f, 0x72, 0x74, 0x68, 0x83, 0x63, 0x61, 0x74, 0x80, 0x80, 0x80, 0x80,
            0x80, 0x80, 0x80, 0x80, 0x80, 0x85, 0x70, 0x75, 0x70, 0x70, 0x79
        ];
        vector::push_back(&mut proof, p1);
        vector::push_back(&mut proof, p2);
        vector::push_back(&mut proof, p3);

        let flag = verify_eth_trie_exclusion_proof(root, key, proof);
        // Since "dogg" does not exist, the exclusion proof should be valid.
        assert!(flag, 1);
    }

    #[test(supra_framework = @supra_framework)]
    public fun test_proof_empty(supra_framework: signer) {
        prepare_env(&supra_framework);
        let root: vector<u8> = vector[
            // (Suppose this is the root of a trie that contains some keys.)
            0x8a, 0xad, 0x78, 0x9d, 0xff, 0x2f, 0x53, 0x8b, 0xca, 0x5d, 0x8e, 0xa5, 0x6e,
            0x8a, 0xbe, 0x10, 0xf4, 0xc7, 0xba, 0x3a, 0x5d, 0xea, 0x95, 0xfe, 0xa4, 0xcd,
            0x6e, 0x7c, 0x3a, 0x11, 0x68, 0xd3
        ];
        let key: vector<u8> = b"doe";
        let proof: vector<vector<u8>> = vector::empty();

        // Since the proof is empty, both inclusion and exclusion proof verification should return false
        let (flag_inclusion, _value) = verify_eth_trie_inclusion_proof(root, key, proof);
        assert!(!flag_inclusion, 1);
        let flag_exclusion = verify_eth_trie_exclusion_proof(root, key, proof);
        assert!(!flag_exclusion, 1);
    }

    #[test(supra_framework = @supra_framework)]
    public fun test_proof_bad(supra_framework: signer) {
        prepare_env(&supra_framework);
        let root: vector<u8> = vector[
            // (Suppose this is the root of a trie that contains some keys.)
            0x8a, 0xad, 0x78, 0x9d, 0xff, 0x2f, 0x53, 0x8b, 0xca, 0x5d, 0x8e, 0xa5, 0x6e,
            0x8a, 0xbe, 0x10, 0xf4, 0xc7, 0xba, 0x3a, 0x5d, 0xea, 0x95, 0xfe, 0xa4, 0xcd,
            0x6e, 0x7c, 0x3a, 0x11, 0x68, 0xd3
        ];
        let key: vector<u8> = b"doe";
        let proof: vector<vector<u8>> = vector[b"aaa", b"ccc"];

        // Since the proof is invalid, both inclusion and exclusion proof verification should return false
        let (flag_inclusion, _value) = verify_eth_trie_inclusion_proof(root, key, proof);
        assert!(!flag_inclusion, 1);
        let flag_exclusion = verify_eth_trie_exclusion_proof(root, key, proof);
        assert!(!flag_exclusion, 1);
    }

    #[test(supra_framework = @supra_framework)]
    public fun test_proof_random_trie(supra_framework: signer) {
        prepare_env(&supra_framework);
        let (root, outer_vec) = generate_random_trie(100);

        let i = 0;
        while (i < vector::length(&outer_vec)) {
            let triple_vec = vector::borrow(&outer_vec, i);
            // triple_vec[0] is the key
            let key = vector::borrow(triple_vec, 0);
            // The rest of triple_vec are proof nodes
            let j = 1;
            let proof = vector::empty<vector<u8>>();
            while (j < vector::length(triple_vec)) {
                vector::push_back(&mut proof, *vector::borrow(triple_vec, j));
                j = j + 1;
            };

            // Now call verify_proof
            let (flag, value) = verify_eth_trie_inclusion_proof(root, *key, proof);
            // Because we inserted key=val in the Rust code, the proof should be correct
            // and we expect `flag == true`.
            assert!(flag, 1);

            // Also check the value
            assert!(value == *key, 2);

            i = i + 1;
        };
    }
}
