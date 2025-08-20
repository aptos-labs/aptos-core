module 0xABCD::scheduled_txns_perf {
    use std::signer;
    use std::option::Option;
    use aptos_std::debug;
    use std::string;
    use aptos_framework::scheduled_txns;

    #[persistent]
    fun no_op(_s: Option<signer>) {
        // This function does nothing, it is just a placeholder for testing purposes.
    }

    #[persistent]
    fun compute_intense(_s: Option<signer>) {
        let sum = 0u64;
        // Intense computation loop
        let i = 0u64;
        while (i < 10000) {
            sum = sum + i * i;  // Some arbitrary computation
            i = i + 1;
        };
    }

    public entry fun test_insert_transactions(user: &signer, current_time_ms: u64, use_compute_intense: bool) {
        //debug::print(&string::utf8(b"test_insert_transactions"));

        let txn1 = if (use_compute_intense) {
            let foo_compute_intense = |s: Option<signer>| compute_intense(s);
            scheduled_txns::new_scheduled_transaction(
                signer::address_of(user),
                current_time_ms,
                10000,
                150,
                false,
                foo_compute_intense
            )
        } else {
            let foo_no_op = |s: Option<signer>| no_op(s);
            scheduled_txns::new_scheduled_transaction(
                signer::address_of(user),
                current_time_ms,
                10000,
                150,
                false,
                foo_no_op
            )
        };

        scheduled_txns::insert(user, txn1);
    }
}
