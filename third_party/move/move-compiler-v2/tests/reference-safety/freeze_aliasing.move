
module 0x42::n {
    fun dead_store() {
        let x = 3;
        let r = &mut x;
        let y = &mut x;
        let _z = freeze(y);
        *r = 4;
    }

    fun eq(x: &mut u64): bool {
        x == x
    }

    fun freeze_indirect() {
        use std::vector;
        let x = vector[1, 2, 3];
        let r = &mut x;
        let i = *vector::borrow(r, 1);
        *vector::borrow_mut(r, i) = 4;
        //*vector::borrow_mut(r, *vector::borrow(r, 1)) = 4;
    }

    fun freeze_direct() {
        use std::vector;
        let x = vector[1, 2, 3];
        let r = &mut x;
        *vector::borrow_mut(r, *vector::borrow(r, 1)) = 4;
    }
}
