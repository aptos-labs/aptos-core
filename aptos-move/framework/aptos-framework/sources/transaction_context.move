module aptos_framework::transaction_context {
    /// A wrapper denoting aptos unique identifer (AUID)
    /// for storing an address
    struct AUID has drop, store {
        unique_address: address
    }

    /// Return the transaction hash of the current transaction.
    native fun get_txn_hash(): vector<u8>;

    /// Return the transaction hash of the current transaction.
    /// Internally calls the private function `get_txn_hash`.
    /// This function is created for to feature gate the `get_txn_hash` function.
    public fun get_transaction_hash(): vector<u8> {
        get_txn_hash()
    }

    /// Return a universally unique identifier (of type address) generated
    /// by hashing the transaction hash of this transaction and a sequence number
    /// specific to this transaction. This function can be called any
    /// number of times inside a single transaction. Each such call increments
    /// the sequence number and generates a new unique address.
    /// Uses Scheme in types/src/transaction/authenticator.rs for domain separation
    /// from other ways of generating unique addresses.
    native fun generate_unique_address(): address;

    /// Return a aptos unique identifier. Internally calls
    /// the private function `generate_unique_address`. This function is
    /// created for to feature gate the `generate_unique_address` function.
    public fun generate_auid_address(): address {
        generate_unique_address()
    }

    /// Return the script hash of the current entry function.
    public native fun get_script_hash(): vector<u8>;

    /// This method runs `generate_unique_address` native function and returns
    /// the generated unique address wrapped in the AUID class.
    public fun generate_auid(): AUID {
        return AUID {
            unique_address: generate_unique_address()
        }
    }

    public fun auid_address(auid: &AUID): address {
        auid.unique_address
    }

    #[test(fx = @std)]
    fun test_auid_uniquess(fx: signer) {
        use std::features;
        use std::vector;

        let feature = features::get_auids();
        features::change_feature_flags(&fx, vector[feature], vector[]);

        let auids: vector<address> = vector<address>[];
        let i: u64 = 0;
        let count: u64 = 50;
        while (i < count) {
            i = i + 1;
            vector::push_back(&mut auids, generate_auid_address());
        };
        i = 0;
        while (i < count - 1) {
            let j: u64 = i + 1;
            while (j < count) {
                assert!(*vector::borrow(&auids, i) != *vector::borrow(&auids, j), 0);
                j = j + 1;
            };
            i = i + 1;
        };
    }
}
