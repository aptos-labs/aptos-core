module 0x42::freeze_mutable_borrow {
    fun test() {
        use std::vector;
        let x = vector[1, 2, 3];
        let r = &mut x;
        *vector::borrow_mut(r, *vector::borrow(r, 1)) = 4;
    }


    fun test2(): u64 {
        let x = 3;
        let r = &mut x;
        let y = &mut x;
        let _z = freeze(y);
        *r
    }

    fun test3(): u64 {
        use std::vector;
        let x = vector[1, 2, 3];
        let r = &mut x;
        *vector::borrow(freeze(r), *vector::borrow(r, 1))
    }

}
