module 0x42::test {
    use std::signer;
    use std::vector;
    use std::signer;

    struct FunctionValue(|u64| u64 with store+copy) has store, copy, drop;

    struct Registry has key {
        functions: vector<FunctionEntry>
    }

    struct FunctionEntry has store, copy {
        f: FunctionValue,
        key: u64
    }

    enum Option<T> has store, copy {
        None(),
        Some(T)
    }

    fun get_function(v: &vector<FunctionEntry>, k: u64): Option<FunctionValue> {
        let x = Option::None;
        vector::for_each_ref(v, |f: &FunctionEntry| {
            if (f.key == k) {
                x = Option::Some(f.f)
            }
        });
        x
    }

    fun replace_or_add_function(v: &mut vector<Function>, k: u64, new_f: |u64| u64 with store+copy): Option<|u64| u64 with store+copy> {
        let result = Option::None;
        vector::for_each_mut(v, |f: &mut Function| {
            if (f.key == k) {
                result = Option::Some(f.f);
                f.f = new_f;
            }
        });
        if (result == Option::None) {
            let new_record = Function { f: new_f, key: k };
            vector::push_back(v, new_record);
        };
        result
    }

    public fun alt_call_selected_function(v: &vector<Function>, k: u64, x: u64): Option<u64> {
        for (i in 0..(vector::length(v))) {
            if (v[i].key == k) {
                return Option::Some((v[i].f)(x))
            }
        };
        None
    }

    fun register(owner: &signer, f: |u64| u64 with store+copy, k: u64) acquires Registry {
        let addr = signer::address_of(owner);
        if (!exists<Registry>(addr)) {
            let new_registry = Registry {
                functions: vector[]
            };
            move_to<Registry>(owner, new_registry);
        };
        let registry = borrow_global_mut<Registry>(addr);
        replace_or_add_function(&mut registry.functions, k, f);
    }

    fun invoke(addr: address, k: u64, x: u64): Option<u64> acquires Registry {
        if (!exists<Registry>(addr)) {
            return Option::None
        };
        let registry = borrow_global<Registry>(addr);
        match (get_function(&registry.functions, k)) {
            Some(func) => {
                let Function { f: f, key: key } = func;
                Option::Some((f.0)(x))
            },
            _ => {
                Option::None
            }
        }
    }

    fun invoke2(addr: address, k: u64, x: u64): Option<u64> acquires Registry {
        if (!exists<Registry>(addr)) {
            return Option::None
        };
        let registry = borrow_global<Registry>(addr);
        for (i in 0..(vector::length(&registry.functions))) {
            if (registry.functions[i].key == k) {
                return Option::Some((registry.functions[i].f)(x))
            }
        };
        None
    }

    fun double(x: u64):u64 {
        x * 2
    }

    fun triple(x: u64):u64 {
        x * 3
    }

    public fun multiply(x: u64, y: u64): u64 {
        x * y
    }

    fun multiply_by_x(x: u64): FunctionValue {
        FunctionValue(move |y| multiply(x, y))
    }

    fun multiply_by_x2(x: u64): FunctionValue {
        FunctionValue(move |y| multiply(x, y))
    }

    #[test(a = @0x42)]
    fun test_registry1(a: signer) {
        register(a, double, 2);
        register(a, negate, 3);
        register(a, multiply_by_x(4), 4);
        register(a, multiply_by_x(5), 5);
        register(a, multiply_by_x2(6), 6);

        match (invoke(a, 2, 10)) {
            Option::Some(x) => { assert!(x == 20); }
            _ => assert!(false)
        };
        match (invoke(a, 3, 11)) {
            Option::Some(x) => { assert!(x == 33); }
            _ => assert!(false)
        };
        match (invoke(a, 4, 2)) {
            Option::Some(x) => { assert!(x == 8); }
            _ => assert!(false)
        };
        match (invoke(a, 5, 3)) {
            Option::Some(x) => { assert!(x == 15); }
            _ => assert!(false)
        };
        match (invoke(a, 6, 3)) {
            Option::Some(x) => { assert!(x == 18); }
            _ => assert!(false)
        };
    }

    #[test(a = @0x42)]
    fun test_registry2(a: signer) {
        register(a, double, 2);
        register(a, negate, 3);
        register(a, multiply_by_x(4), 4);
        register(a, multiply_by_x(5), 5);
        register(a, multiply_by_x2(6), 6);

        match (invoke2(a, 2, 10)) {
            Some(x) => { assert!(x == 20); }
            _ => assert!(false)
        };
        match (invoke2(a, 3, 11)) {
            Some(x) => { assert!(x == 33); }
            _ => assert!(false)
        };
        match (invoke2(a, 4, 2)) {
            Some(x) => { assert!(x == 8); }
            _ => assert!(false)
        };
        match (invoke2(a, 5, 3)) {
            Some(x) => { assert!(x == 15); }
            _ => assert!(false)
        };
        match (invoke2(a, 6, 3)) {
            Some(x) => { assert!(x == 18); }
            _ => assert!(false)
        };
    }


    #[test(a = @0x42)]
    fun test_registry3(a: signer) {
        register(a, double, 2);
        let registry = borrow_global<Registry>(a);
        assert!(registry.functions[0].key == 2);
        assert!((registry.functions[0].func)(3) == 6);
    }
}
