// RUN: publish --print(stackless,micro-ops)
module 0x1::test {
    const FLAGS: vector<bool> = vector[true, false];

    fun flag(n: u64): bool {
        *std::vector::borrow(&FLAGS, n)
    }
}

// RUN: execute 0x1::test::flag --args 0
// CHECK: results: true

// RUN: execute 0x1::test::flag --args 1
// CHECK: results: false
