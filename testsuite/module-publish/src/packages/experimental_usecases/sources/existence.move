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
            a2: utf8(b"a2a2a2"),
        }
    }

    fun make_b(seed: u64): B {
        let va = vector::empty<A>();
        vector::push_back(&mut va, make_a(100 + seed));
        vector::push_back(&mut va, make_a(200 + seed));
        vector::push_back(&mut va, make_a(500 + seed));
        B {
            va: va,
            name: utf8(b"Super epic B"),
            super_a: make_a(1000 + seed),
        }
    }

    public entry fun create(publisher: &signer) {
        assert!(
            signer::address_of(publisher) == @0xABCD,
            ENOT_AUTHORIZED,
        );

        let bt = table::new<B, String>();
        table::add(&mut bt, make_b(10), utf8(b"B number 1"));
        table::add(&mut bt, make_b(20), utf8(b"B number 2"));
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
            event::emit(DummyEvent { value: 321 });
        }
    }

    public entry fun modify() acquires C {
        let a = borrow_global_mut<C>(@0xABCD);
        a.b1.super_a.a1 = a.b1.super_a.a1 + 1;
    }
}
