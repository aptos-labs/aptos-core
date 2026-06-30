// RUN: publish --print(stackless,micro-ops)
module 0x1::test {
    const NUMS: vector<u64> = vector[100, 200, 300];

    fun nums_len(): u64 {
        std::vector::length(&NUMS)
    }

    fun nums_first(): u64 {
        *std::vector::borrow(&NUMS, 0)
    }

    fun nums_last(): u64 {
        *std::vector::borrow(&NUMS, 2)
    }

    fun nums_modification(): u64 {
        let nums = NUMS;
        let v = std::vector::borrow_mut(&mut nums, 0);
        *v = 400;
        *std::vector::borrow(&nums, 0)
    }
}

// RUN: execute 0x1::test::nums_len
// CHECK: results: 3

// RUN: execute 0x1::test::nums_first
// CHECK: results: 100

// RUN: execute 0x1::test::nums_last
// CHECK: results: 300

// RUN: execute 0x1::test::nums_modification
// CHECK: results: 400
