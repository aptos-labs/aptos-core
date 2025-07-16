module 0xABCD::existence {
    use std::signer;
    use std::vector;
    use std::string::{String, utf8};
    use aptos_std::table;
    use aptos_std::table::{Table};
    use aptos_framework::event;

    const ENOT_AUTHORIZED: u64 = 1;

    struct A has key, store, copy, drop {
        a1: u64,
        a2: String,
    }

    struct B has key, store, copy, drop {
        va: vector<A>,
        name: String,
        super_a: A,
    }

    struct C has key {
        b1: B,
        bt: Table<B, String>,
    }

    #[event]
    struct DummyEvent has drop, store {
        value: u64,
    }

    fun make_a(seed: u64): A {
        A {
            a1: seed + 42,
            a2: utf8(b"Lorem ipsum"),
        }
    }

    fun make_b(seed: u64): B {
        let va = vector::empty<A>();
        let i = 0;
        while (i < 100) {
            vector::push_back(&mut va, make_a(10 * i + seed));
            i = i + 1;
        };
        B {
            va: va,
            name: utf8(b"dolor sit amet"),
            super_a: make_a(1000 + seed),
        }
    }

    fun very_long_function(how_long: u64): u64 {
        let sum = 0;
        let i = 0;
        while (i < how_long) {
            let j = 0;
            while (j < 10) {
                let k = 0;
                while (k < 10) {
                    sum = sum + i * j * k + k * i * i + j + 1;
                    k = k + 1;
                };
                j = j + 1;
            };
            i = i + 1;
        };
        sum
    }

    fun init_module(publisher: &signer) {
        assert!(
            signer::address_of(publisher) == @0xABCD,
            ENOT_AUTHORIZED,
        );

        let bt = table::new<B, String>();
        let i = 0;
        while (i < 100) {
            table::add(&mut bt, make_b(i + 1), utf8(b"consectetur adipiscing elit"));
            i = i + 1;
        };
        move_to<C>(
            publisher,
            C {
                b1: make_b(30),
                bt: bt,
            }
        );
    }

    public entry fun check() {
        if (exists<C>(@0xABCD)) {
            event::emit(DummyEvent { value: very_long_function(50) });
        }
    }

    public entry fun modify() acquires C {
        let a = borrow_global_mut<C>(@0xABCD);
        let v = very_long_function(100);
        a.b1.super_a.a1 = a.b1.super_a.a1 + v;
    }
}
