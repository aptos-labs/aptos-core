module 0x42::invalid_resource {
    use std::signer;
    use std::i64;
    use std::i128;
    fun test_exist1(addr: address): i64 {
        if (exists<i64::I64>(addr)) {
            1
        } else {
            2
        }
     }

     fun test_exist2(addr: address): i64 {
        if (exists<i64>(addr)) {
            1
        } else {
            2
        }
     }

     fun test_exist3(addr: address): i128 {
        if (exists<i128::I128>(addr)) {
            1
        } else {
            2
        }
     }

     fun test_exist4(addr: address): i128 {
        if (exists<i128>(addr)) {
            1
        } else {
            2
        }
     }

     fun test_borrow1(addr: address): i64 {
        let s = borrow_global<i64::I64>(addr);
        *s
     }

     fun test_borrow2(addr: address): i64 {
        let s = borrow_global<i64>(addr);
        *s
     }

     fun test_borrow3(addr: address): i128 {
        let s = borrow_global<i128::I128>(addr);
        *s
     }

     fun test_borrow4(addr: address): i128 {
        let s = borrow_global<i128>(addr);
        *s
     }

     fun test_move_to1(account: &signer, x: i64::I64) {
       move_to<i64::I64>(account, x)
     }

     fun test_move_to2(account: &signer, x: i64) {
       move_to<i64>(account, x)
     }

     fun test_move_to3(account: &signer, x: i128::I128) {
       move_to<i128::I128>(account, x)
     }

     fun test_move_to4(account: &signer, x: i128) {
       move_to<i128>(account, x)
     }

     fun test_move_from1(account: &signer): i64 {
        move_from<i64::I64>(signer::address_of(account))
     }

     fun test_move_from2(account: &signer): i64 {
        move_from<i64>(signer::address_of(account))
     }

     fun test_move_from3(account: &signer): i128 {
        move_from<i128::I128>(signer::address_of(account))
     }

     fun test_move_from4(account: &signer): i128 {
        move_from<i128>(signer::address_of(account))
     }
}
