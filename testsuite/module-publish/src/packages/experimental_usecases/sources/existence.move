module 0xABCD::existence {
    use std::signer;
    use aptos_framework::event;

    const ENOT_AUTHORIZED: u64 = 1;

    struct A has key {
        value: u64,
    }

    struct B has key {
        value: u64,
    }

    #[event]
    struct DummyEvent has drop, store {
        value: u64,
    }

    public entry fun create(publisher: &signer) {
        assert!(
            signer::address_of(publisher) == @0xABCD,
            ENOT_AUTHORIZED,
        );

        move_to<A>(
            publisher,
            A {
                value: 123,
            }
        );
    }

    public entry fun check() {
        if (exists<A>(@0xABCD)) {
            event::emit(DummyEvent { value: 321 });
        }
    }

    public entry fun modify() acquires A {
        let a = borrow_global_mut<A>(@0xABCD);
        a.value = a.value + 1;
    }
}
