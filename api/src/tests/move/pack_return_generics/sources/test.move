module addr::return_generics {

    struct State<T> has key, copy {
        value: T,
    }

    entry fun set<T: store>(s: &signer, value: T) {
        move_to(s, State { value });
    }

    #[view]
    public fun get1<T: store+copy>(s: address): State<T> acquires State {
        *borrow_global<State<T>>(s)
    }

    #[view]
    public fun get2<T: store+copy>(s: address): T acquires State {
        borrow_global<State<T>>(s).value
    }

    #[view]
    public fun get3<T: store+copy>(s: address): vector<State<T>> acquires State {
        vector[get1<T>(s)]
    }
}
