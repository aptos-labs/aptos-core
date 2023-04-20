module 0xbeef::test {
    struct Foo has key {}

    public entry fun run_move_to(s: signer) {
        move_to<Foo>(&s, Foo {});
    }

    public entry fun run_exists() {
        exists<Foo>(@0x1);
    }
}
