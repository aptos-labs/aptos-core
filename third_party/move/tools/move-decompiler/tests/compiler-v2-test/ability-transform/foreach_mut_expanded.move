module 0x42::m {

    use 0x1::vector;

    fun test_for_each_mut() {
        let v = vector[1, 2, 3];
        let i = 0;
        let len = vector::length(&v);
        let vr = &mut v;
        while (i < len) {
            let x = vector::borrow_mut(vr, i);
            *x = 2;
            i = i + 1
        };
        assert!(v == vector[2, 3, 4], 0);
    }
}
