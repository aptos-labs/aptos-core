module 0x1::transaction_context_test {
    use std::vector;
    use aptos_framework::transaction_context::create_guid;
    
    /// When checking the value of aggregator fails.
    const ENOT_UNIQUE: u64 = 20;

    public entry fun create_many_guids(_account: &signer, count: u64) {
        let guids: vector<address> = vector<address>[];
        let i: u64 = 0;
        while (i < count) {
            i = i+1;
            vector::push_back(&mut guids, create_guid());
        };
        i = 0;
        while (i < count - 1) {
            let j: u64 = i + 1;
            while (j < count) {
                assert!(*vector::borrow(&guids, i) != *vector::borrow(&guids, j), ENOT_UNIQUE);
                j = j + 1;
            };
            i = i + 1;
        };
    }
}
