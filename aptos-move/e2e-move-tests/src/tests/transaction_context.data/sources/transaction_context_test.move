module 0x1::transaction_context_test {
    use std::vector;
    use aptos_framework::transaction_context::create_unique_addr;
    use std::features;

    /// When checking the value of aggregator fails.
    const ENOT_UNIQUE: u64 = 20;

    public entry fun create_many_uuids(_account: &signer, count: u64) {
        if (features::auids_enabled()) {
            let uuids: vector<address> = vector<address>[];
            let i: u64 = 0;
            while (i < count) {
                i = i+1;
                vector::push_back(&mut uuids, create_unique_addr());
            };
            i = 0;
            while (i < count - 1) {
                let j: u64 = i + 1;
                while (j < count) {
                    assert!(*vector::borrow(&uuids, i) != *vector::borrow(&uuids, j), ENOT_UNIQUE);
                    j = j + 1;
                };
                i = i + 1;
            };
        }
    }
}
