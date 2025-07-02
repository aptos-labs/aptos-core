//# publish
module 0x42::print_values {
    enum A has key, store, drop {
        V0 { x: u8, y: u8 }
        V1 { x: u8, }
    }

    struct B has key, store, drop {
        a: A,
        data: vector<u8>,
    }

    public fun return_bool(): bool {
        return true
    }

    public fun return_u8(): u8 {
        return 0
    }

    public fun return_u16(): u16 {
        return 1
    }

    public fun return_u32(): u32 {
        return 2
    }

    public fun return_u64(): u64 {
        return 3
    }

    public fun return_u128(): u128 {
        return 4
    }

    public fun return_u256(): u256 {
        return 5
    }

    public fun return_addr(): address {
        return @0x6
    }

    public fun return_vector(): vector<u64> {
        return vector[1, 2]
    }

    public fun return_signer(account: signer): signer {
        return account
    }

    public fun return_struct(): B {
        B { a: A::V1 { x: 23 }, data: vector[1, 2, 3] }
    }

    public fun sum(a: u64, b: u64): u64 {
        a + b
    }

    // Note: fails with invariant violation because MoveVM serializes the return value, and returned value is not
    // serializable because it is an anonymous lambda-lifted function.
    public fun return_anonymous(): || {
        || {}
    }

    public fun return_add(): |u64,u64|u64 {
        |x, y| sum(x, y)
    }

    public fun return_add_first(y: u64): |u64|u64 {
        |x| sum(y, x)
    }

    public fun return_add_last(y: u64): |u64|u64 {
        |x| sum(x, y)
    }

    public fun return_add_two_and_one(): ||u64 {
        || sum(2, 1)
    }

    public fun return_closure_capture_struct(): ||B {
        return_struct
    }
}

//# run 0x42::print_values::return_bool

//# run 0x42::print_values::return_u8

//# run 0x42::print_values::return_u16

//# run 0x42::print_values::return_u32

//# run 0x42::print_values::return_u64

//# run 0x42::print_values::return_u128

//# run 0x42::print_values::return_addr

//# run 0x42::print_values::return_vector

//# run 0x42::print_values::return_signer --signers 0x42

//# run 0x42::print_values::return_struct

//# run 0x42::print_values::return_anonymous

//# run 0x42::print_values::return_add

//# run 0x42::print_values::return_add_first --args 5

//# run 0x42::print_values::return_add_last --args 10

//# run 0x42::print_values::return_add_two_and_one

//# run 0x42::print_values::return_closure_capture_struct
