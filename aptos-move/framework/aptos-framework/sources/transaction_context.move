module aptos_framework::transaction_context {

    use std::features;

    /// AUID feature is not supported.
    const EAUID_NOT_SUPPORTED: u64 = 3;

    /// A wrapper denoting aptos unique identifer (AUID)
    /// for storing an address
    struct AUID has drop, store {
        unique_address: address
    }

    /// Return the transaction hash of the current transaction
    public native fun get_txn_hash(): vector<u8>;

    /// Return a universally unique identifier (of type address) generated
    /// by hashing the transaction hash of this transaction and a sequence number
    /// specific to this transaction. This function can be called any
    /// number of times inside a single transaction. Each such call increments
    /// the sequence number and generates a new unique address.
    /// Uses Scheme in types/src/transaction/authenticator.rs for domain separation
    /// from other ways of generating unique addresses.
    native fun create_unique_address(): address;

    /// Return a universally unique identifier. Internally calls
    /// the private function `create_unique_address`. This function is
    /// created for to feature gate the `create_unique_address` function.
    public fun create_unique_addr(): address {
        assert!(features::auids_enabled(), EAUID_NOT_SUPPORTED);
        create_unique_address()
    }

    /// Return the script hash of the current entry function.
    public native fun get_script_hash(): vector<u8>;

    /// This method runs `create_unique_address` native function and returns
    /// the generated unique address wrapped in the AUID class.
    public fun create_auid(): AUID {
        assert!(features::auids_enabled(), EAUID_NOT_SUPPORTED);
        return AUID {
            unique_address: create_unique_address()
        }
    }

    public fun get_unique_address(auid: AUID): address {
        auid.unique_address
    }
}
