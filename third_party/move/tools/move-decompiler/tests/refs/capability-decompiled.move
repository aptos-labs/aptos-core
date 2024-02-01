module 0x1::capability {
    struct Cap<phantom T0> has copy, drop {
        root: address,
    }
    
    struct CapDelegateState<phantom T0> has key {
        root: address,
    }
    
    struct CapState<phantom T0> has key {
        delegates: vector<address>,
    }
    
    struct LinearCap<phantom T0> has drop {
        root: address,
    }
    
    public fun acquire<T0>(arg0: &signer, arg1: &T0) : Cap<T0> acquires CapDelegateState, CapState {
        let v0 = validate_acquire<T0>(arg0);
        Cap<T0>{root: v0}
    }
    
    public fun acquire_linear<T0>(arg0: &signer, arg1: &T0) : LinearCap<T0> acquires CapDelegateState, CapState {
        let v0 = validate_acquire<T0>(arg0);
        LinearCap<T0>{root: v0}
    }
    
    fun add_element<T0: drop>(arg0: &mut vector<T0>, arg1: T0) {
        if (!0x1::vector::contains<T0>(arg0, &arg1)) {
            0x1::vector::push_back<T0>(arg0, arg1);
        };
    }
    
    public fun create<T0>(arg0: &signer, arg1: &T0) {
        assert!(!exists<CapState<T0>>(0x1::signer::address_of(arg0)), 0x1::error::already_exists(1));
        let v0 = CapState<T0>{delegates: 0x1::vector::empty<address>()};
        move_to<CapState<T0>>(arg0, v0);
    }
    
    public fun delegate<T0>(arg0: Cap<T0>, arg1: &T0, arg2: &signer) acquires CapState {
        let v0 = 0x1::signer::address_of(arg2);
        if (exists<CapDelegateState<T0>>(v0)) {
            return
        };
        let v1 = CapDelegateState<T0>{root: arg0.root};
        move_to<CapDelegateState<T0>>(arg2, v1);
        add_element<address>(&mut borrow_global_mut<CapState<T0>>(arg0.root).delegates, v0);
    }
    
    public fun linear_root_addr<T0>(arg0: LinearCap<T0>, arg1: &T0) : address {
        arg0.root
    }
    
    fun remove_element<T0: drop>(arg0: &mut vector<T0>, arg1: &T0) {
        let (v0, v1) = 0x1::vector::index_of<T0>(arg0, arg1);
        if (v0) {
            0x1::vector::remove<T0>(arg0, v1);
        };
    }
    
    public fun revoke<T0>(arg0: Cap<T0>, arg1: &T0, arg2: address) acquires CapDelegateState, CapState {
        if (!exists<CapDelegateState<T0>>(arg2)) {
            return
        };
        let CapDelegateState {  } = move_from<CapDelegateState<T0>>(arg2);
        remove_element<address>(&mut borrow_global_mut<CapState<T0>>(arg0.root).delegates, &arg2);
    }
    
    public fun root_addr<T0>(arg0: Cap<T0>, arg1: &T0) : address {
        arg0.root
    }
    
    fun validate_acquire<T0>(arg0: &signer) : address acquires CapDelegateState, CapState {
        let v0 = 0x1::signer::address_of(arg0);
        if (exists<CapDelegateState<T0>>(v0)) {
            let v2 = borrow_global<CapDelegateState<T0>>(v0).root;
            assert!(exists<CapState<T0>>(v2), 0x1::error::invalid_state(3));
            let v3 = 0x1::vector::contains<address>(&borrow_global<CapState<T0>>(v2).delegates, &v0);
            assert!(v3, 0x1::error::invalid_state(3));
            v2
        } else {
            assert!(exists<CapState<T0>>(v0), 0x1::error::not_found(2));
            v0
        }
    }
    
    // decompiled from Move bytecode v6
}
