module module_owner::mpc_example {
    use std::option;
    use aptos_framework::mpc;

    struct PendingResults has drop, key {
        task: u64,
    }

    entry fun trigger_raise(account: &signer, element: vector<u8>) {
        let task = mpc::raise_by_secret(element, 0);
        let pending_elements = PendingResults { task };
        move_to(account, pending_elements);
    }

    entry fun fetch_and_verify(account: &signer, expected: vector<u8>) acquires PendingResults {
        let my_addr = std::signer::address_of(account);
        let task = borrow_global<PendingResults>(my_addr).task;
        let result = mpc::get_result(task);
        let actual = option::extract(&mut result);
        assert!(expected == actual, 7);
        let _ = move_from<PendingResults>(my_addr);
    }
}
