module 0x1::vector {
    public fun contains<T0>(arg0: &vector<T0>, arg1: &T0) : bool {
        let v0 = 0;
        while (v0 < length<T0>(arg0)) {
            if (borrow<T0>(arg0, v0) == arg1) {
                return true
            };
            v0 = v0 + 1;
        };
        false
    }
    
    public fun index_of<T0>(arg0: &vector<T0>, arg1: &T0) : (bool, u64) {
        let v0 = 0;
        while (v0 < length<T0>(arg0)) {
            if (borrow<T0>(arg0, v0) == arg1) {
                return (true, v0)
            };
            v0 = v0 + 1;
        };
        (false, 0)
    }
    
    public fun range(arg0: u64, arg1: u64) : vector<u64> {
        range_with_step(arg0, arg1, 1)
    }
    
    public fun append<T0>(arg0: &mut vector<T0>, arg1: vector<T0>) {
        reverse<T0>(&mut arg1);
        reverse_append<T0>(arg0, arg1);
    }
    
    native public fun borrow<T0>(arg0: &vector<T0>, arg1: u64) : &T0;
    native public fun borrow_mut<T0>(arg0: &mut vector<T0>, arg1: u64) : &mut T0;
    native public fun destroy_empty<T0>(arg0: vector<T0>);
    native public fun empty<T0>() : vector<T0>;
    public fun insert<T0>(arg0: &mut vector<T0>, arg1: u64, arg2: T0) {
        let v0 = length<T0>(arg0);
        assert!(arg1 <= v0, 131072);
        push_back<T0>(arg0, arg2);
        while (arg1 < v0) {
            swap<T0>(arg0, arg1, v0);
            arg1 = arg1 + 1;
        };
    }
    
    public fun is_empty<T0>(arg0: &vector<T0>) : bool {
        length<T0>(arg0) == 0
    }
    
    native public fun length<T0>(arg0: &vector<T0>) : u64;
    native public fun pop_back<T0>(arg0: &mut vector<T0>) : T0;
    native public fun push_back<T0>(arg0: &mut vector<T0>, arg1: T0);
    public fun range_with_step(arg0: u64, arg1: u64, arg2: u64) : vector<u64> {
        assert!(arg2 > 0, 131075);
        let v0 = vector[];
        while (arg0 < arg1) {
            push_back<u64>(&mut v0, arg0);
            arg0 = arg0 + arg2;
        };
        v0
    }
    
    public fun remove<T0>(arg0: &mut vector<T0>, arg1: u64) : T0 {
        let v0 = length<T0>(arg0);
        if (arg1 >= v0) {
            abort 131072
        };
        while (arg1 < v0 - 1) {
            let v1 = arg1;
            let v2 = arg1 + 1;
            arg1 = v2;
            swap<T0>(arg0, v1, v2);
        };
        pop_back<T0>(arg0)
    }
    
    public fun remove_value<T0>(arg0: &mut vector<T0>, arg1: &T0) : vector<T0> {
        let (v0, v1) = index_of<T0>(arg0, arg1);
        if (v0) {
            let v3 = empty<T0>();
            push_back<T0>(&mut v3, remove<T0>(arg0, v1));
            v3
        } else {
            empty<T0>()
        }
    }
    
    public fun reverse<T0>(arg0: &mut vector<T0>) {
        reverse_slice<T0>(arg0, 0, length<T0>(arg0));
    }
    
    public fun reverse_append<T0>(arg0: &mut vector<T0>, arg1: vector<T0>) {
        let v0 = length<T0>(&arg1);
        while (v0 > 0) {
            push_back<T0>(arg0, pop_back<T0>(&mut arg1));
            v0 = v0 - 1;
        };
        destroy_empty<T0>(arg1);
    }
    
    public fun reverse_slice<T0>(arg0: &mut vector<T0>, arg1: u64, arg2: u64) {
        assert!(arg1 <= arg2, 131073);
        if (arg1 == arg2) {
            return
        };
        arg2 = arg2 - 1;
        while (arg1 < arg2) {
            swap<T0>(arg0, arg1, arg2);
            arg1 = arg1 + 1;
            arg2 = arg2 - 1;
        };
    }
    
    public fun rotate<T0>(arg0: &mut vector<T0>, arg1: u64) : u64 {
        rotate_slice<T0>(arg0, 0, arg1, length<T0>(arg0))
    }
    
    public fun rotate_slice<T0>(arg0: &mut vector<T0>, arg1: u64, arg2: u64, arg3: u64) : u64 {
        reverse_slice<T0>(arg0, arg1, arg2);
        reverse_slice<T0>(arg0, arg2, arg3);
        reverse_slice<T0>(arg0, arg1, arg3);
        arg1 + arg3 - arg2
    }
    
    public fun singleton<T0>(arg0: T0) : vector<T0> {
        let v0 = empty<T0>();
        push_back<T0>(&mut v0, arg0);
        v0
    }
    
    public fun slice<T0: copy>(arg0: &vector<T0>, arg1: u64, arg2: u64) : vector<T0> {
        assert!(arg1 <= arg2 && arg2 <= length<T0>(arg0), 131076);
        let v0 = empty<T0>();
        while (arg1 < arg2) {
            push_back<T0>(&mut v0, *borrow<T0>(arg0, arg1));
            arg1 = arg1 + 1;
        };
        v0
    }
    
    native public fun swap<T0>(arg0: &mut vector<T0>, arg1: u64, arg2: u64);
    public fun swap_remove<T0>(arg0: &mut vector<T0>, arg1: u64) : T0 {
        assert!(!is_empty<T0>(arg0), 131072);
        swap<T0>(arg0, arg1, length<T0>(arg0) - 1);
        pop_back<T0>(arg0)
    }
    
    public fun trim<T0>(arg0: &mut vector<T0>, arg1: u64) : vector<T0> {
        let v0 = trim_reverse<T0>(arg0, arg1);
        reverse<T0>(&mut v0);
        v0
    }
    
    public fun trim_reverse<T0>(arg0: &mut vector<T0>, arg1: u64) : vector<T0> {
        let v0 = length<T0>(arg0);
        let v1 = v0;
        assert!(arg1 <= v0, 131072);
        let v2 = empty<T0>();
        while (arg1 < v1) {
            push_back<T0>(&mut v2, pop_back<T0>(arg0));
            v1 = v1 - 1;
        };
        v2
    }
    
    // decompiled from Move bytecode v6
}
