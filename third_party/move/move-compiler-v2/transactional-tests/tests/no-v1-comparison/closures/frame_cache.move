//# publish
module 0x42::dummy {
    struct Dummy has key {}
}

//# publish
module 0x42::foo {
    struct Foo<phantom T> has key, store { value: u64 }

    public fun foo_roundtrip<T>() {
        let foo = Foo<T> { value: 0 };
        let Foo<T> { value: _ } = foo;
    }

    public fun foo_exists<T>(account: &signer): bool {
        let addr = std::signer::address_of(account);
        exists<Foo<T>>(addr)
    }
}

//# publish
module 0x42::bar {
    struct Bar<phantom T> has key { value: u64 }

    public fun dummy_bar_exists(account: &signer): bool {
        let addr = std::signer::address_of(account);
        exists<Bar<0x42::dummy::Dummy>>(addr)
    }

    public fun make_dummy_bar(account: &signer) {
        move_to(account, Bar<0x42::dummy::Dummy> { value: 10 })
    }
}

//# publish
module 0x42::poc {
    fun run(account: &signer) {
        let f = || 0x42::foo::foo_roundtrip<0x42::dummy::Dummy>();
        f();
        let g = |acc| 0x42::bar::make_dummy_bar(acc);
        g(account);
    }
}

//# run 0x42::poc::run --signers 0x42

//# run 0x42::bar::dummy_bar_exists --signers 0x42

//# run 0x42::foo::foo_exists --signers 0x42 --type-args 0x42::dummy::Dummy
