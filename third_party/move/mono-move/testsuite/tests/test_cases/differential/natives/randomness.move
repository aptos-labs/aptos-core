// The randomness natives: the transaction counter starts at 0, advances once per fetch and
// comes back as 8 little-endian bytes. The test context is marked unbiasable.

// RUN: publish
module 0x1::randomness {
    native fun fetch_and_increment_txn_counter(): vector<u8>;
    native fun is_unbiasable(): bool;

    public fun first(): vector<u8> {
        fetch_and_increment_txn_counter()
    }

    public fun third(): vector<u8> {
        fetch_and_increment_txn_counter();
        fetch_and_increment_txn_counter();
        fetch_and_increment_txn_counter()
    }

    // Returns the `count`th fetch; at 257 the counter carries into its second byte.
    public fun nth(count: u64): vector<u8> {
        let index = 1;
        while (index < count) {
            fetch_and_increment_txn_counter();
            index += 1;
        };
        fetch_and_increment_txn_counter()
    }

    public fun unbiasable(): bool {
        is_unbiasable()
    }
}

// RUN: execute 0x1::randomness::first
// CHECK: results: 0x0000000000000000

// RUN: execute 0x1::randomness::third
// CHECK: results: 0x0200000000000000

// RUN: execute 0x1::randomness::nth --args 257
// CHECK: results: 0x0001000000000000

// RUN: execute 0x1::randomness::unbiasable
// CHECK: results: true
