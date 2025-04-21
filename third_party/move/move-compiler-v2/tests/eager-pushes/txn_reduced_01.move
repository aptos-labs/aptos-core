module 0xc0ffee::m {
    use std::vector;

    public fun test(from: &signer, to_vec: vector<address>, amount_vec: vector<u64>) {
        let len = vector::length(&to_vec);
        let i = 0;
        while (i < len) {
            let to = vector::borrow(&to_vec, i);
            let amount = vector::borrow(&amount_vec, i);
            call_other(from, *to, *amount);
            i = i + 1;
        }
    }

    fun call_other(_from: &signer, _to: address, _amount: u64) {}
}
