module 0x42::valid_ref_resource {
    use std::signer;

    struct S1  has copy, drop, key { x: u64, y: i64, z: i128 } // struct with i64 and i128 fields

    fun test_borrow1(a: &i64): &i64 {
        a
    }

    fun test_borrow2(a: &i128): &i128 {
        a
    }

    fun test_deref1(a: &i64): i64 {
        *a
    }

    fun test_deref2(a: &i128): i128 {
        *a
    }

    fun test_exist1(addr: address): i64 {
        if (exists<S1>(addr)) {
            let s = borrow_global<S1>(addr);
            s.y
        } else {
            1
        }
     }

    fun test_exist2(addr: address): i128 {
        if (exists<S1>(addr)) {
            let s = borrow_global<S1>(addr);
            s.z
        } else {
            1
        }
     }

     fun test_move_to(account: &signer, addr: address) {
        let s1 = S1 {x: 1, y: -1, z: -2};
        if (!exists<S1>(addr)){
            move_to<S1>(account, s1)
        }
     }

     fun test_move_from(account: &signer, addr: address): i64 {
        let s1 = S1 {x: 1, y: -1, z: -2};
        if (exists<S1>(addr)){
            move_from<S1>(signer::address_of(account)).y
        } else {
            s1.y
        }
     }
}
