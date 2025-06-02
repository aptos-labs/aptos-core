//# publish
module 0x42::foo {
    struct Foo<T: store> has key, drop {
        f: |T|vector<T>,
    }

    public fun make_foo<T: store>(account: &signer) {
        let f = |e| std::vector::singleton<T>(e);
        move_to(account, Foo { f })
    }

}

//# publish
module 0x42::test {
    fun run<T: store>(account: &signer) {
        let f = |a| 0x42::foo::make_foo<T>(a);
        f(account);
    }
}
