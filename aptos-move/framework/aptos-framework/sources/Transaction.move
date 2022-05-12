/// Native Move functions for transaction metadata and scheduling
module AptosFramework::Transaction {

    use AptosFramework::TypeInfo
    use Std::ASCII::String;

    /// Return the version number of the distributed database,
    /// corresponding to the number of transactions the system will have
    /// executed upon the conclusion of the current one. Reference:
    /// [Versioned database](https://aptos.dev/basics/basics-txns-states#versioned-database)
    public native fun version_number(): u64;

    /// Schedule transaction for the epilogue of a block, delaying by
    /// `delay` blocks. A `delay` of 0 schedules a transaction during
    /// the epilogue of the current block, and can only be called from a
    /// non-epilogue transaction. Calling during the epilogue requires
    /// a `delay` of at least 1, corresponding to a transaction
    /// scheduled during the epilogue of the next block.
    public(script) native fun schedule(
        account: &signer,
        script_address: address,
        script_module_name: String,
        script_function_name: String,
        type_arguments: &vector<TypeInfo>,
        arguments: &vector<String>,
        delay: u64,
    );
}