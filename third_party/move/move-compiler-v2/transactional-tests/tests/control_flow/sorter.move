//# publish --print-bytecode
module 0x42::heap {
    use std::vector;

    fun create1(): vector<u64> {
        vector<u64>[3, 2, 1, 5, 8, 4]
    }

    fun create2(): vector<u64> {
        vector<u64>[1, 2, 3, 4, 5, 8]
    }

    fun vcopy(x: &vector<u64>): vector<u64> {
        let y : vector<u64> = vector::empty<u64>();
        let i : u64 = 0;
        let l : u64 = vector::length<u64>(x);
        while (i < l) {
            vector::push_back<u64>(&mut y, *vector::borrow<u64>(x, i));
            i = i + 1;
        };
        y
    }

    fun sort(x: &mut vector<u64>) {
        let i: u64 = 0;
        while (i < vector::length<u64>(x)) {
            let j: u64 = i + 1;
            while (j < vector::length<u64>(x)) {
                if (*vector::borrow<u64>(x, i) > *vector::borrow<u64>(x, j)) {
                    vector::swap<u64>(x, i, j)
                };
                j = j + 1;
            };
            i = i + 1;
        }
    }

    fun array_equals(x: &vector<u64>, y: &vector<u64>): bool {
        let l1: u64 = vector::length<u64>(x);
        let l2: u64 = vector::length<u64>(y);
        if (l1 != l2) {
            return false
        };
        let i: u64 = 0;
        while (i < l1) {
            if (*vector::borrow<u64>(x, i) != *vector::borrow<u64>(y, i)) {
                return false
            };
            i = i + 1;
        };
        true
    }

    public fun main() {
        let x: vector<u64> = create1();
        let y: vector<u64> = create2();
        let z: vector<u64> = vcopy(&x);
        assert!(array_equals(&x, &z), 23);
        assert!(array_equals(&y, &y), 29);
        sort(&mut x);
        assert!(array_equals(&y, &x), 31);
        assert!(array_equals(&x, &y), 29);
        assert!(!array_equals(&x, &z), 31);
    }
}

//# run --print-bytecode
script {
use 0x42::heap::main;
fun mymain() {
    main();
}
}
