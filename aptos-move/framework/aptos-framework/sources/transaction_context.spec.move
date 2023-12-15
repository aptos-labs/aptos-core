spec aptos_framework::transaction_context {
    spec get_script_hash(): vector<u8> {
        pragma opaque;
        // property 4: Fetching the script hash of the current entry function should never fail
        // and should return a vector with 32 bytes if the transaction payload is a script, otherwise an empty vector.
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
        // property 1: Fetching the transaction hash should return a vector with 32 bytes, if the auid feature flag is enabled.
        ensures [abstract] len(result) == 32;
    }
    spec generate_unique_address(): address {
        pragma opaque;
        ensures [abstract] result == spec_generate_unique_address();
    }
    spec fun spec_generate_unique_address(): address;
    spec generate_auid_address(): address {
        pragma opaque;
        // property 3: Generating the unique address should return a vector with 32 bytes, if the auid feature flag is enabled.
        ensures [abstract] result == spec_generate_unique_address();
    }
    spec auid_address(auid: &AUID): address {
        // property 2: Fetching the unique address should never abort.
        aborts_if false;
    }
}
