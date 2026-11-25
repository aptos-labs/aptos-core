// `!=` on function values, as a value and as a branch condition. Checks that one closure
// equals itself, that two closures over the same function with different captured values
// differ, both arms of the branch, and the lowering of these comparisons.

// RUN: publish --print(micro-ops)
module 0x42::closure_neq {
    fun add_base(base: u64, input: u64): u64 { base + input }

    public fun self_neq(base: u64): bool {
        let first: |u64|u64 has copy + drop = |input| add_base(base, input);
        first != first
    }

    public fun distinct_neq(base: u64): bool {
        let other = base + 1;
        let first: |u64|u64 has copy + drop = |input| add_base(base, input);
        let second: |u64|u64 has copy + drop = |input| add_base(other, input);
        first != second
    }

    public fun neq_branch(base: u64, other: u64): u64 {
        let first: |u64|u64 has copy + drop = |input| add_base(base, input);
        let second: |u64|u64 has copy + drop = |input| add_base(other, input);
        if (first != second) first(10) else second(20)
    }
}

// RUN: execute 0x42::closure_neq::self_neq --args 3
// CHECK: results: false

// RUN: execute 0x42::closure_neq::distinct_neq --args 3
// CHECK: results: true

// RUN: execute 0x42::closure_neq::neq_branch --args 3, 4
// CHECK: results: 13

// RUN: execute 0x42::closure_neq::neq_branch --args 3, 3
// CHECK: results: 23
