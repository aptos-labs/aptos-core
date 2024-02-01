module 0x1::option {
    struct Option<T0> has copy, drop, store {
        vec: vector<T0>,
    }
    
    public fun contains<T0>(arg0: &Option<T0>, arg1: &T0) : bool {
        0x1::vector::contains<T0>(&arg0.vec, arg1)
    }
    
    public fun borrow<T0>(arg0: &Option<T0>) : &T0 {
        assert!(is_some<T0>(arg0), 262145);
        0x1::vector::borrow<T0>(&arg0.vec, 0)
    }
    
    public fun borrow_mut<T0>(arg0: &mut Option<T0>) : &mut T0 {
        assert!(is_some<T0>(arg0), 262145);
        0x1::vector::borrow_mut<T0>(&mut arg0.vec, 0)
    }
    
    public fun swap<T0>(arg0: &mut Option<T0>, arg1: T0) : T0 {
        assert!(is_some<T0>(arg0), 262145);
        let v0 = &mut arg0.vec;
        0x1::vector::push_back<T0>(v0, arg1);
        0x1::vector::pop_back<T0>(v0)
    }
    
    public fun borrow_with_default<T0>(arg0: &Option<T0>, arg1: &T0) : &T0 {
        let v0 = &arg0.vec;
        if (0x1::vector::is_empty<T0>(v0)) {
            arg1
        } else {
            0x1::vector::borrow<T0>(v0, 0)
        }
    }
    
    public fun destroy_none<T0>(arg0: Option<T0>) {
        assert!(is_none<T0>(&arg0), 262144);
        let Option { vec: v0 } = arg0;
        0x1::vector::destroy_empty<T0>(v0);
    }
    
    public fun destroy_some<T0>(arg0: Option<T0>) : T0 {
        assert!(is_some<T0>(&arg0), 262145);
        let Option { vec: v0 } = arg0;
        0x1::vector::destroy_empty<T0>(v0);
        0x1::vector::pop_back<T0>(&mut v0)
    }
    
    public fun destroy_with_default<T0: drop>(arg0: Option<T0>, arg1: T0) : T0 {
        let Option { vec: v0 } = arg0;
        if (0x1::vector::is_empty<T0>(&mut v0)) {
            arg1
        } else {
            0x1::vector::pop_back<T0>(&mut v0)
        }
    }
    
    public fun extract<T0>(arg0: &mut Option<T0>) : T0 {
        assert!(is_some<T0>(arg0), 262145);
        0x1::vector::pop_back<T0>(&mut arg0.vec)
    }
    
    public fun fill<T0>(arg0: &mut Option<T0>, arg1: T0) {
        let v0 = &mut arg0.vec;
        assert!(0x1::vector::is_empty<T0>(v0), 262144);
        0x1::vector::push_back<T0>(v0, arg1);
    }
    
    public fun from_vec<T0>(arg0: vector<T0>) : Option<T0> {
        assert!(0x1::vector::length<T0>(&arg0) <= 1, 262146);
        Option<T0>{vec: arg0}
    }
    
    public fun get_with_default<T0: copy + drop>(arg0: &Option<T0>, arg1: T0) : T0 {
        let v0 = &arg0.vec;
        if (0x1::vector::is_empty<T0>(v0)) {
            arg1
        } else {
            *0x1::vector::borrow<T0>(v0, 0)
        }
    }
    
    public fun is_none<T0>(arg0: &Option<T0>) : bool {
        0x1::vector::is_empty<T0>(&arg0.vec)
    }
    
    public fun is_some<T0>(arg0: &Option<T0>) : bool {
        !0x1::vector::is_empty<T0>(&arg0.vec)
    }
    
    public fun none<T0>() : Option<T0> {
        Option<T0>{vec: 0x1::vector::empty<T0>()}
    }
    
    public fun some<T0>(arg0: T0) : Option<T0> {
        Option<T0>{vec: 0x1::vector::singleton<T0>(arg0)}
    }
    
    public fun swap_or_fill<T0>(arg0: &mut Option<T0>, arg1: T0) : Option<T0> {
        let v0 = &mut arg0.vec;
        let v1 = if (0x1::vector::is_empty<T0>(v0)) {
            none<T0>()
        } else {
            some<T0>(0x1::vector::pop_back<T0>(v0))
        };
        0x1::vector::push_back<T0>(v0, arg1);
        v1
    }
    
    public fun to_vec<T0>(arg0: Option<T0>) : vector<T0> {
        let Option { vec: v0 } = arg0;
        v0
    }
    
    // decompiled from Move bytecode v6
}
