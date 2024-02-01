module 0x1::bit_vector {
    struct BitVector has copy, drop, store {
        length: u64,
        bit_field: vector<bool>,
    }
    
    public fun length(arg0: &BitVector) : u64 {
        0x1::vector::length<bool>(&arg0.bit_field)
    }
    
    public fun is_index_set(arg0: &BitVector, arg1: u64) : bool {
        assert!(arg1 < 0x1::vector::length<bool>(&arg0.bit_field), 131072);
        *0x1::vector::borrow<bool>(&arg0.bit_field, arg1)
    }
    
    public fun longest_set_sequence_starting_at(arg0: &BitVector, arg1: u64) : u64 {
        assert!(arg1 < arg0.length, 131072);
        let v0 = arg1;
        while (v0 < arg0.length) {
            if (!is_index_set(arg0, v0)) {
                break
            };
            v0 = v0 + 1;
        };
        v0 - arg1
    }
    
    public fun new(arg0: u64) : BitVector {
        assert!(arg0 > 0, 131073);
        assert!(arg0 < 1024, 131073);
        let v0 = 0;
        let v1 = 0x1::vector::empty<bool>();
        while (v0 < arg0) {
            0x1::vector::push_back<bool>(&mut v1, false);
            v0 = v0 + 1;
        };
        BitVector{
            length    : arg0, 
            bit_field : v1,
        }
    }
    
    public fun set(arg0: &mut BitVector, arg1: u64) {
        assert!(arg1 < 0x1::vector::length<bool>(&arg0.bit_field), 131072);
        *0x1::vector::borrow_mut<bool>(&mut arg0.bit_field, arg1) = true;
    }
    
    public fun shift_left(arg0: &mut BitVector, arg1: u64) {
        if (arg1 >= arg0.length) {
            let v0 = &mut arg0.bit_field;
            let v1 = 0;
            while (v1 < 0x1::vector::length<bool>(v0)) {
                *0x1::vector::borrow_mut<bool>(v0, v1) = false;
                v1 = v1 + 1;
            };
        } else {
            let v2 = arg1;
            while (v2 < arg0.length) {
                if (is_index_set(arg0, v2)) {
                    let v3 = v2 - arg1;
                    set(arg0, v3);
                } else {
                    let v4 = v2 - arg1;
                    unset(arg0, v4);
                };
                v2 = v2 + 1;
            };
            v2 = arg0.length - arg1;
            while (v2 < arg0.length) {
                unset(arg0, v2);
                v2 = v2 + 1;
            };
        };
    }
    
    public fun unset(arg0: &mut BitVector, arg1: u64) {
        assert!(arg1 < 0x1::vector::length<bool>(&arg0.bit_field), 131072);
        *0x1::vector::borrow_mut<bool>(&mut arg0.bit_field, arg1) = false;
    }
    
    // decompiled from Move bytecode v6
}
