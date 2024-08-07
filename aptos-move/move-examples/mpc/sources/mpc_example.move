module module_owner::mpc_example {
    use std::signer::address_of;
    use std::vector;
    use aptos_framework::mpc;

    struct PendingResults has drop, key {
        tasks: vector<u64>,
    }

    entry fun trigger_raise(account: &signer, element: vector<u8>) acquires PendingResults {
        let my_addr = address_of(account);
        if (!exists<PendingResults>(my_addr)) {
            move_to(account, PendingResults { tasks: vector[] })
        };

        let task = mpc::raise_by_secret(element, 0);
        let tasks = &mut borrow_global_mut<PendingResults>(my_addr).tasks;
        vector::push_back(tasks, task);
    }
}
