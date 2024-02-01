module 0x1::comparator {
    struct Result has drop {
        inner: u8,
    }
    
    public fun compare<T0>(arg0: &T0, arg1: &T0) : Result {
        compare_u8_vector(0x1::bcs::to_bytes<T0>(arg0), 0x1::bcs::to_bytes<T0>(arg1))
    }
    
    public fun compare_u8_vector(arg0: vector<u8>, arg1: vector<u8>) : Result {
        let v0 = 0x1::vector::length<u8>(&arg0);
        let v1 = 0x1::vector::length<u8>(&arg1);
        let v2 = 0;
        while (v2 < v0 && v2 < v1) {
            let v3 = *0x1::vector::borrow<u8>(&arg0, v2);
            let v4 = *0x1::vector::borrow<u8>(&arg1, v2);
            if (v3 < v4) {
                return Result{inner: 1}
            };
            if (v3 > v4) {
                return Result{inner: 2}
            };
            v2 = v2 + 1;
        };
        if (v0 < v1) {
            Result{inner: 1}
        } else {
            if (v0 > v1) {
                Result{inner: 2}
            } else {
                Result{inner: 0}
            }
        }
    }
    
    public fun is_equal(arg0: &Result) : bool {
        arg0.inner == 0
    }
    
    public fun is_greater_than(arg0: &Result) : bool {
        arg0.inner == 2
    }
    
    public fun is_smaller_than(arg0: &Result) : bool {
        arg0.inner == 1
    }
    
    // decompiled from Move bytecode v6
}
