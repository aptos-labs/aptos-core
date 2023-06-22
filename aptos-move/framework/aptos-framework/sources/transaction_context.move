module aptos_framework::transaction_context {

    use std::features;

    /// create_unique_address uses this for domain separation within its native implementation
    #[test_only]
    const DERIVE_UUID_ADDRESS_SCHEME: u8 = 0xFB;

    /// UUID feature is not supported.
    const EUUID_NOT_SUPPORTED: u64 = 3;

    /// A wrapper denoting universally unique identifer (UUID)
    /// for storing an address
    struct UUID has drop, store {
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
        assert!(features::uuids_enabled(), EUUID_NOT_SUPPORTED);
        create_unique_address()
    }

    /// Return the script hash of the current entry function.
    public native fun get_script_hash(): vector<u8>;

    /// This method runs `create_unique_address` native function and returns
    /// the generated unique address wrapped in the UUID class.
    public fun create_uuid(): UUID {
        assert!(features::uuids_enabled(), EUUID_NOT_SUPPORTED);
        return UUID {
            unique_address: create_unique_address()
        }
    }

    #[test]
    fun test_correct_uuid() {
        let uuid1 = create_unique_addr();
        let bytes = std::bcs::to_bytes(&aptos_framework::guid::create(get_txn_hash(), 1));
        vector::push_back(&mut bytes, DERIVE_UUID_ADDRESS_SCHEME);
        let uuid2 = aptos_framework::from_bcs::to_address(std::hash::sha3_256(bytes));
        assert!(uuid1 == uuid2, 0);
    }
}
