spec aptos_framework::transaction_context {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: Fetching the transaction hash should return a vector with 32 bytes.
    /// Criticality: Medium
    /// Implementation: The get_transaction_hash function calls the native function get_txn_hash, which fetches the
    /// NativeTransactionContext struct and returns the txn_hash field.
    /// Enforcement: Audited that the native function returns the txn hash, whose size is 32 bytes. This has been
    /// modeled as the abstract postcondition that the returned vector is of length 32. Formally verified via [high-level-req-1](get_txn_hash).
    ///
    /// No.: 2
    /// Requirement: Fetching the unique address should never abort.
    /// Criticality: Low
    /// Implementation: The function auid_address returns the unique address from a supplied AUID resource.
    /// Enforcement: Formally verified via [high-level-req-2](auid_address).
    ///
    /// No.: 3
    /// Requirement: Generating the unique address should return a vector with 32 bytes.
    /// Criticality: Medium
    /// Implementation: The generate_auid_address function checks calls the native function generate_unique_address
    /// which fetches the NativeTransactionContext struct, increments the auid_counter by one, and then creates a new
    /// authentication key from a preimage, which is then returned.
    /// Enforcement: Audited that the native function returns an address, and the length of an address is 32 bytes.
    /// This has been modeled as the abstract postcondition that the returned vector is of length 32.
    /// Formally verified via [high-level-req-3](generate_auid_address).
    ///
    /// No.: 4
    /// Requirement: Fetching the script hash of the current entry function should never fail and should return a vector
    /// with 32 bytes if the transaction payload is a script, otherwise an empty vector.
    /// Criticality: Low
    /// Implementation: The native function get_script_hash returns the NativeTransactionContext.script_hash field.
    /// Enforcement: Audited that the native function holds the required property. This has been modeled as the abstract
    /// spec. Formally verified via [high-level-req-4](get_script_hash).
    /// </high-level-req>
    spec get_script_hash(): vector<u8> {
        pragma opaque;
        // property 4: Fetching the script hash of the current entry function should never fail
        // and should return a vector with 32 bytes if the transaction payload is a script, otherwise an empty vector.
        /// [high-level-req-4]
        aborts_if [abstract] false;
        ensures [abstract] result == spec_get_script_hash();
        ensures [abstract] len(result) == 32;
    }
    spec fun spec_get_script_hash(): vector<u8>;
    spec get_txn_hash(): vector<u8> {
        pragma opaque;
        aborts_if [abstract] false;
        ensures result == spec_get_txn_hash();
    }
    spec fun spec_get_txn_hash(): vector<u8>;
    spec get_transaction_hash(): vector<u8> {
        pragma opaque;
        aborts_if [abstract] false;
        ensures result == spec_get_txn_hash();
        // property 1: Fetching the transaction hash should return a vector with 32 bytes, if the auid feature flag is enabled.
        /// [high-level-req-1]
        ensures [abstract] len(result) == 32;
    }
    spec generate_unique_address(): address {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_generate_unique_address();
    }
    spec fun spec_generate_unique_address(): address;
    spec generate_auid_address(): address {
        pragma opaque;
        aborts_if [abstract] false;
        // property 3: Generating the unique address should return a vector with 32 bytes, if the auid feature flag is enabled.
        /// [high-level-req-3]
        ensures [abstract] result == spec_generate_unique_address();
    }
    spec auid_address(auid: &AUID): address {
        // property 2: Fetching the unique address should never abort.
        /// [high-level-req-2]
        aborts_if false;
    }

    spec sender_internal(): address {
        //TODO: temporary mockup
        pragma opaque;
    }
    spec secondary_signers_internal(): vector<address> {
        //TODO: temporary mockup
        pragma opaque;
    }
    spec gas_payer_internal(): address {
        //TODO: temporary mockup
        pragma opaque;
    }
    spec max_gas_amount_internal(): u64 {
        //TODO: temporary mockup
        pragma opaque;
    }
    spec gas_unit_price_internal(): u64 {
        //TODO: temporary mockup
        pragma opaque;
    }
    spec chain_id_internal(): u8 {
        //TODO: temporary mockup
        pragma opaque;
    }
    spec entry_function_payload_internal(): Option<EntryFunctionPayload> {
        //TODO: temporary mockup
        pragma opaque;
    }

    spec multisig_payload_internal(): Option<MultisigPayload> {
        //TODO: temporary mockup
        pragma opaque;
    }

    spec monotonically_increasing_counter_internal(timestamp_us: u64): u128 {
        //TODO: temporary mockup
        pragma opaque;
    }
}
