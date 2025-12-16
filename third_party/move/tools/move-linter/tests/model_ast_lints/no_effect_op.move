module 0xc0ffee::m {
    use std::signer;

    public fun test1_warn(){
        let x = 42;
        x;
        42;
        b"hello";
        vector<u64>[];
        vector[0];
        vector[b"hello"];
        {
            let x = 42;
            x;
            42;
            b"hello";
            vector<u64>[];
            vector[0];
            vector[b"hello"];
        }
    }

    public fun test2_warn() {
        let x = 42;
        *(&mut (x + 1)) = 1;
        *(&mut 42) = 1;
        *(&mut 42) = x;
        *(&mut 42) = x + 1;
        *(&mut 42) = x + x;
        *(&mut 42) = x - 1;
        *(&mut 42) = x - x;
        *(&mut 42) = x * 2;
        *(&mut 42) = x * x;
        *(&mut 42) = x / 2;
        *(&mut 42) = x / x;
        *(&mut 42) = x & x;
        *(&mut 42) = x | x;
        *(&mut 42) = x ^ x;
        *(&mut 42) = x << x;
        *(&mut 42) = x >> x;
        *(&mut 42) = if (x == x){ 1 }else{ 0 };
        *(&mut 42) = if (x != x){ 1 }else{ 0 };
        *(&mut 42) = if (x >  x){ 1 }else{ 0 };
        *(&mut 42) = if (x >= x){ 1 }else{ 0 };
        *(&mut 42) = if (x <  x){ 1 }else{ 0 };
        *(&mut 42) = if (x <= x){ 1 }else{ 0 };
        *(&mut 42) = { if (x <= x){ 1 }else{ 0 } };
    }

    public fun test3_warn() {
        let x = true;
        *(&mut true) = !x;
        *(&mut true) = !!x;
        *(&mut true) = x && x;
        *(&mut true) = x && !x;
        *(&mut true) = x || x;
        *(&mut true) = x || !x;
        let y = 42;
        *(&mut true) = y == y;
        *(&mut true) = y != y;
        *(&mut true) = y >  y;
        *(&mut true) = y >= y;
        *(&mut true) = y <  y;
        *(&mut true) = y <= y;
    }

    public fun test4_warn() {
        *(&mut vector<u64>[]) = vector[0_u64];
    }

    public fun test5_warn(account: signer) acquires S {
        pure1();
        pure2();
        pure3(42);
        pure4(&42);
        pure5(signer::address_of(&account));
        let x = true;
        impure1(&mut x);
        impure2(signer::address_of(&account));

        let y: u64 = 64;
        falsely_impure(&mut y);
    }

    public fun test6_warn(): bool {
        let x = 42;
        loop{
            x;
        };
        if (true){
            return {
                x;
                true
            };
            x;
        }else{
            x;
        };
        false
    }

    /*****************************************************/

    /*public fun test1_no_warn(){
        let x = 42;
        vector[
            {
                x += 43;
                x
            },
            x + 1
        ];
    }*/

    /*****************************************************/

    fun pure1(): bool{
        true
    }

    fun pure2(): bool{
        let x = 42;
        x == x
    }

    fun pure3(x: u64): u64{
        x = x * 2 + 1;
        x
    }

    fun pure4(x: &u64): u64{
        let x = *x * 2 + 1;
        x
    }

    fun pure5(addr: address): u64 acquires S{
        borrow_global<S>(addr).x
    }

    fun impure1(y: &mut bool): bool{
        let x = 42;
        *y = !*y;
        x == x
    }

    struct S has key, drop {
        x: u64,
    }

    fun impure2(addr: address): bool acquires S{
        borrow_global_mut<S>(addr).x = 42;
        true
    }

    fun falsely_impure(x: &mut u64): u64{
        *x * 2
    }
}

#[lint::skip(needless_ref_in_field_access, needless_deref_ref, needless_mutable_reference)]
module 0xc0ffee::no_op_mut_ref_effects {
    struct S has copy, key, drop { x: u64 }

    // Mutation through a field of a mutable global reference must not be treated as a no-op.
    public fun assign_through_field(addr: address) acquires S {
        let r = borrow_global_mut<S>(addr);
        *(&mut r.x) = 5;
    }

    // Mutation through an extra deref of a mutable reference must not be treated as a no-op.
    public fun assign_through_deref(x: &mut u64) {
        *(&mut *x) = 7;
    }

    // Combine deref + field projection to cover both Operation::Deref and Operation::Select.
    public fun assign_through_nested(addr: address) acquires S {
        let r = borrow_global_mut<S>(addr);
        *(&mut (*r).x) = 9;
    }
}
