module 0x42::table_option {
    use extensions::table;
    use std::option::{Self, Option};

    struct MultisigAccount has key {
        transactions: table::Table<u64, MultisigTransaction>,
        last_executed_sequence_number: u64,
    }

    struct MultisigTransaction has copy, drop, store {
        payload: Option<vector<u8>>,
    }

    public fun get_next_transaction_payload(
        multisig_account: address, provided_payload: vector<u8>): vector<u8> acquires MultisigAccount {
        let multisig_account_resource = borrow_global<MultisigAccount>(multisig_account);
        let sequence_number = multisig_account_resource.last_executed_sequence_number + 1;
        let transaction = table::borrow(&multisig_account_resource.transactions, sequence_number);

        if (option::is_some(&transaction.payload)) {
            *option::borrow(&transaction.payload)
        } else {
            provided_payload
        }
    }
    spec get_next_transaction_payload(
    multisig_account: address, provided_payload: vector<u8>
    ): vector<u8> {
        let multisig_account_resource = global<MultisigAccount>(multisig_account);
        let sequence_number = multisig_account_resource.last_executed_sequence_number + 1;
        let transaction = table::spec_get(multisig_account_resource.transactions, sequence_number);
        ensures option::is_some(transaction.payload) ==> result == option::borrow(transaction.payload); // This should pass.
    }
}
