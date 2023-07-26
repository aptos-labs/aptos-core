module no_code_mint::auid_manager {
    use aptos_framework::transaction_context;
    use std::vector;
    use std::error;

    /// The amount of addresses we can generate in one transaction has been exceeded.
    const EGREATER_THAN_U8_MAX: u64 = 0;

    const DERIVE_AUID_ADDRESS_SCHEME: u8 = 0xFB;

    // We will use (vector::length(&addresses) + 1) as current increment value instead of tracking it
    struct AuidManager {
        addresses: vector<address>, // Already computed addresses
    }

    public fun destroy(auid_manager: AuidManager) {
        let AuidManager {
            addresses: _,
        } = auid_manager;
    }

    public fun create(): AuidManager {
        AuidManager {
            addresses: vector<address> [ ],
        }
    }

    // TODO: fix this so it works when n > 255, use bit shifting or something
    public fun increment(auid_manager: &mut AuidManager): address {
        let bytes = transaction_context::get_transaction_hash();
        let n = ((vector::length(&auid_manager.addresses) + 1) as u8);
        assert!(n < 255, error::invalid_argument(EGREATER_THAN_U8_MAX));
vector::append(&mut bytes, vector[n,0,0,0,0,0,0,0,DERIVE_AUID_ADDRESS_SCHEME]);

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
        while (i < 50) {
            increment(&mut auid_manager);
            i = i + 1;
        };

        vector::for_each_ref(borrow_auids(&auid_manager), |auid| {
            let generated = transaction_context::generate_auid_address();
            assert!(*auid == generated, 0);
        });

        destroy(auid_manager);
    }
}
