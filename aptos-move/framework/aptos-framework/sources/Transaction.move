/// Native Move functions for transaction metadata and scheduling
module AptosFramework::Transaction {

    use Std::Option::Option;
    use Std::ASCII::String;

    /// Return the version number of the distributed database,
    /// corresponding to the number of transactions the system will have
    /// executed upon the conclusion of the current one. Reference:
    /// [Versioned database](https://aptos.dev/basics/basics-txns-states#versioned-database)
    public native fun version_number(): u64;

    /// Initialize a recurring transaction for a public script function
    /// that takes no arguments, to execute at the end of every `n`th
    /// block
    public(script) native fun init_schedule(
        account: &signer,
        script_address: address,
        script_module_name: String,
        script_function_name: String,
        n: u64,
    );

    /// Cancel the recurring schedule for a transaction previously
    /// authorized by `account`
    public(script) native fun cancel_schedule(
        account: &signer,
        script_address: address,
        script_module_name: String,
        script_function_name: String,
    );
}