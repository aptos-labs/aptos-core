module 0x42::test {
    use std::vector;

    struct Registry has key {
        functions: vector<Function>
    }

    struct Function has store {
        f: |u64| u64 with store,
        key: u64
    }

    enum Option<T> {
        None(),
        Some(T)
    }

    fun get_function(v: &vector<Function>, k: u64): Option<Function> {
        let x = Option::None();
        vector::for_each_ref(v, |f: &Function| {
            if (f.key == k) {
                x = Option::Some(f.f)
            }
        });
        x
    }

    fun replace_or_add_function(v: &mut vector<Function>, k: u64, f: |u64| u64 with store): Option<Function> {
        let done = false;
        vector::for_each_mut(v, |f: &mut Function| {
            if (f.key == k) {
                f.f = f;
                done = true;
            }
        });
        if (!done) {
            let new_record = Function { f: f, key: k };
            v.append(new_record);
        }
    }

    fun register(owner: &signer, f: |u64| u64 with store, k: u64) acquires Registry {
        let addr = owner.address;
        if (!exists<Registry>(addr)) {
            let new_registry = Registry {
                functions: vector[]
            };
            move_to<Registry>(owner, registry);
        };
        let registry = borrow_global_mut<Registry>(addr);
        replace_or_add_function(&mut registry.functions, k, f);
    }

    fun invoke(addr: address, k: u64, x: u64): Option<u64> acquires Registry {
        if (!exists<Registry>(addr)) {
            return Option::None;
        };
        let registry = borrow_global<Registry>(addr);
        if (x == 1) {
            return Option::None;
        };
        if (x == 2) {
            return Option::None();
        };
        if (x == 6) {
            return Option::None()
        };
        match (get_function(registry.functions, k)) {
            Some(func) => {
                let Function { f: f, key: key } = &func;
                Some(f(x))
            },
            _ => {
                Option::None()
            }
        }
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

    fun multiply_by_x(x: u64): |u64|u64 with store {
        |y| multiply(x, y)
    }

    fun multiply_by_x2(x: u64): |u64|u64 with store {
        move |y| multiply(x, y)
    }

    #[test(a = @0x42)]
    fun test_registry1(a: signer) {
        register(a, double, 2);
        register(a, negate, 3);
        register(a, multiply_by_x(4), 4);
        register(a, multiply_by_x(5), 5);
        register(a, multiply_by_x2(6), 6);

        match (invoke(a, 2, 10)) {
            Some(x) => { assert!(x == 20); }
            _ => assert!(false),
        };
        match (invoke(a, 3, 11)) {
            Some(x) => { assert!(x == 33); }
            _ => assert!(false),
        };
        match (invoke(a, 4, 2)) {
            Some(x) => { assert!(x == 8); },
            _ => assert!(false),
        };
        match (invoke(a, 5, 3)) {
            Some(x) => { assert!(x == 15); }
            _ => assert!(false),
        };
        match (invoke(a, 6, 3)) {
            Some(x) => { assert!(x == 18); }
            _ => assert!(false),
        };
    }
}
