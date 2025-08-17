module 0xc0ffee::dummy1 {
    fun baz() {}

    fun bar(b: bool) {
        let () = if (b) { baz() } else { () };
    }
}


module 0xc0ffee::dummy2 {
    struct State has key {
        value: u64
    }

    fun tuple_assignments(s: &signer, state: State) {
        let () = move_to<State>(s, state);
    }
}
