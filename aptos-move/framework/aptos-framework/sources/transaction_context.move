module aptos_framework::transaction_context {
    use std::error;
    use std::features;

    /// Transaction context is not available outside of the transaction prologue, execution, or epilogue phases.
    const ETRANSACTION_CONTEXT_NOT_AVAILABLE: u64 = 1;

    /// The transaction context extension feature is not enabled.
    const ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED: u64 = 2;

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

    /// Return the sender's address for the current transaction.
    /// This function aborts if called outside of the transaction prologue, execution, or epilogue phases.
    native fun sender_internal(): address;
    public fun sender(): address {
        assert!(features::transaction_context_extension_enabled(), error::invalid_state(ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED));
        sender_internal()
    }

    /// Return the list of the secondary signers for the current transaction.
    /// If the current transaction has no secondary signers, this function returns an empty vector.
    /// This function aborts if called outside of the transaction prologue, execution, or epilogue phases.
    native fun secondary_signers_internal(): vector<address>;
    public fun secondary_signers(): vector<address> {
        assert!(features::transaction_context_extension_enabled(), error::invalid_state(ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED));
        secondary_signers_internal()
    }

    /// Return the gas payer address for the current transaction.
    /// It is either the sender's address if no separate gas fee payer is specified for the current transaction,
    /// or the address of the separate gas fee payer if one is specified.
    /// This function aborts if called outside of the transaction prologue, execution, or epilogue phases.
    native fun gas_payer_internal(): address;
    public fun gas_payer(): address {
        assert!(features::transaction_context_extension_enabled(), error::invalid_state(ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED));
        gas_payer_internal()
    }

    /// Return the max gas amount in units which is specified for the current transaction.
    /// This function aborts if called outside of the transaction prologue, execution, or epilogue phases.
    native fun max_gas_amount_internal(): u64;
    public fun max_gas_amount(): u64 {
        assert!(features::transaction_context_extension_enabled(), error::invalid_state(ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED));
        max_gas_amount_internal()
    }

    /// Return the gas unit price in Octas which is specified for the current transaction.
    /// This function aborts if called outside of the transaction prologue, execution, or epilogue phases.
    native fun gas_unit_price_internal(): u64;
    public fun gas_unit_price(): u64 {
        assert!(features::transaction_context_extension_enabled(), error::invalid_state(ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED));
        gas_unit_price_internal()
    }

    /// Return the chain ID specified for the current transaction.
    /// This function aborts if called outside of the transaction prologue, execution, or epilogue phases.
    native fun chain_id_internal(): u8;
    public fun chain_id(): u8 {
        assert!(features::transaction_context_extension_enabled(), error::invalid_state(ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED));
        chain_id_internal()
    }

    #[test(fx = @std)]
    fun test_auid_uniquess(fx: signer) {
        use std::features;
        use std::vector;

        let feature = features::get_auids();
        features::change_feature_flags_for_testing(&fx, vector[feature], vector[]);

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

    #[test]
    #[expected_failure(abort_code=196609, location = Self)]
    fun test_call_sender() {
        // expected to fail with the error code of `invalid_state(E_TRANSACTION_CONTEXT_NOT_AVAILABLE)`
        let _sender = sender();
    }

    #[test]
    #[expected_failure(abort_code=196609, location = Self)]
    fun test_call_secondary_signers() {
        // expected to fail with the error code of `invalid_state(E_TRANSACTION_CONTEXT_NOT_AVAILABLE)`
        let _secondary_signers = secondary_signers();
    }

    #[test]
    #[expected_failure(abort_code=196609, location = Self)]
    fun test_call_gas_payer() {
        // expected to fail with the error code of `invalid_state(E_TRANSACTION_CONTEXT_NOT_AVAILABLE)`
        let _gas_payer = gas_payer();
    }

    #[test]
    #[expected_failure(abort_code=196609, location = Self)]
    fun test_call_max_gas_amount() {
        // expected to fail with the error code of `invalid_state(E_TRANSACTION_CONTEXT_NOT_AVAILABLE)`
        let _max_gas_amount = max_gas_amount();
    }

    #[test]
    #[expected_failure(abort_code=196609, location = Self)]
    fun test_call_gas_unit_price() {
        // expected to fail with the error code of `invalid_state(E_TRANSACTION_CONTEXT_NOT_AVAILABLE)`
        let _gas_unit_price = gas_unit_price();
    }

    #[test]
    #[expected_failure(abort_code=196609, location = Self)]
    fun test_call_chain_id() {
        // expected to fail with the error code of `invalid_state(E_TRANSACTION_CONTEXT_NOT_AVAILABLE)`
        let _chain_id = chain_id();
    }
}
