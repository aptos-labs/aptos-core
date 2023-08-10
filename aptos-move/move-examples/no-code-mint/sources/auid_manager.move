/// This module currently exists as a workaround to the issue of not being able to know the address of a token object
/// when it's generated as an auid address and the address isn't returned from the function it's minted from.
/// The `aptos_token.move` module will eventually return the address, but for now, we must track it manually with this.
module no_code_mint::auid_manager {
    use aptos_framework::transaction_context;
    use std::vector;

    const DERIVE_AUID_ADDRESS_SCHEME: u8 = 0xFB;

    // An unstorable list of auid generated addresses.
    // It's only used to keep track of the addresses in a single transaction
    // Since the values are only meaningful for one transaction, it's automatically destroyed at the end of the transaction
    struct AuidManager has drop {
        addresses: vector<address>,
    }

    public fun create(): AuidManager {
        AuidManager {
            addresses: vector<address> [ ],
        }
    }

    // This function computes auid addresses using the same computation function
    // the native `transaction_context::generate_auid_address` uses.
    // It is also in the object::test_correct_auid unit test.
    public fun increment(auid_manager: &mut AuidManager): address {
        let bytes = transaction_context::get_transaction_hash();
        let n = vector::length(&auid_manager.addresses) + 1;
        let bcs_n = std::bcs::to_bytes(&n);

        vector::append(&mut bytes, bcs_n);
        vector::push_back(&mut bytes, DERIVE_AUID_ADDRESS_SCHEME);

        let new_auid = aptos_framework::from_bcs::to_address(std::hash::sha3_256(bytes));
        vector::push_back(
            &mut auid_manager.addresses,
            new_auid
        );

        new_auid
    }

    public fun get(auid_manager: &AuidManager, i: u64): &address {
        vector::borrow(&auid_manager.addresses, i)
    }

    public fun borrow_auids(auid_manager: &AuidManager): &vector<address> {
        &auid_manager.addresses
    }

    #[test_only]
    public fun enable_auids_for_test(aptos_framework: &signer) {
        use std::features;

        let feature = features::get_auids();
        features::change_feature_flags(aptos_framework, vector[feature], vector[]);
    }

    #[test(aptos_framework = @0x1)]
    fun test_get_multiple_auids(
        aptos_framework: &signer,
    ) {
        enable_auids_for_test(aptos_framework);

        let auid_manager = create();
        let i = 0;
        // Test for the case where n > u8_max
        while (i < 256) {
            increment(&mut auid_manager);
            i = i + 1;
        };

        vector::for_each_ref(borrow_auids(&auid_manager), |auid| {
            let generated = transaction_context::generate_auid_address();
            assert!(*auid == generated, 0);
        });
    }
}
