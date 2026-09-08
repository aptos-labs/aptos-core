module 0x42::calculator {
    use 0x1::signer::address_of;

    const EINVALID_INPUT: u64 = 1;

    enum Input {
        Number(u64),
        Add,
        Sub,
    }

    /// State of the calculator
    enum State has key, copy, drop {
        Empty,
        Value(u64),
        Continuation(|u64|u64)
    }

    /// Process input in the current state.
    fun process(s: &signer, input: Input) acquires State {
        let addr = address_of(s);
        match ((move_from<State>(addr), input)) {
            (Empty, Number(x)) => move_to(s, State::Value(x)),
            (Value(_), Number(x)) => move_to(s, State::Value(x)),
            (Value(x), Add) => move_to(s, State::Continuation(|y| storable_add(x, y))),
            (Value(x), Sub) => move_to(s, State::Continuation(|y| storable_sub(x, y))),
            (Continuation(f), Number(x)) => move_to(s, State::Value(f(x))),
            (_, _) => abort EINVALID_INPUT
        }
    }
    spec process(s: &signer, input: Input) {
        use 0x1::signer;
        pragma opaque = true, aborts_if_is_partial = true;
        modifies State[signer::address_of(s)];
        ensures [inferred = sathard] (old(State[signer::address_of(s)]) is Continuation) && (input is Number) ==> {
            let a = signer::address_of(s);
            let b = State::Value(S1.. |~ result_of<old(State[signer::address_of(s)]).Continuation.0>(input.0));
            S1.. |~ publish<State>(a, b)
        };
        ensures [inferred] (old(State[signer::address_of(s)]) is Value) && (input is Number) ==> {
            let a = signer::address_of(s);
            S1.. |~ publish<State>(a, State::Value(input.0))
        };
        ensures [inferred] (old(State[signer::address_of(s)]) is Value) && (input is Add) ==> {
            let a = signer::address_of(s);
            let b = State::Continuation({
                let c = old(State[signer::address_of(s)]).Value.0;
                |x| storable_add(c, x)
            });
            S1.. |~ publish<State>(a, b)
        };
        ensures [inferred] (old(State[signer::address_of(s)]) is Value) && (input is Sub) ==> {
            let a = signer::address_of(s);
            let b = State::Continuation({
                let c = old(State[signer::address_of(s)]).Value.0;
                |x| storable_sub(c, x)
            });
            S1.. |~ publish<State>(a, b)
        };
        ensures [inferred] (old(State[signer::address_of(s)]) is Empty) && (input is Number) ==> {
            let a = signer::address_of(s);
            S1.. |~ publish<State>(a, State::Value(input.0))
        };
        ensures [inferred] {
            let a = signer::address_of(s);
            ..S1 |~ remove<State>(a)
        };
        aborts_if [inferred] !exists<State>(signer::address_of(s));
        aborts_if [inferred] (State[signer::address_of(s)] is Continuation) && (input is Add | Sub);
        aborts_if [inferred] (input is Add | Sub) && (State[signer::address_of(s)] is Empty);
        aborts_if [inferred] (State[signer::address_of(s)] is Continuation) && (input is Number) && (S1 |~ exists<State>(signer::address_of(s)));
        aborts_if [inferred] (State[signer::address_of(s)] is Value) && (input is Number) && (S1 |~ exists<State>(signer::address_of(s)));
        aborts_if [inferred] (State[signer::address_of(s)] is Value) && (input is Add) && (S1 |~ exists<State>(signer::address_of(s)));
        aborts_if [inferred] (State[signer::address_of(s)] is Value) && (input is Sub) && (S1 |~ exists<State>(signer::address_of(s)));
        aborts_if [inferred] (State[signer::address_of(s)] is Empty) && (input is Number) && (S1 |~ exists<State>(signer::address_of(s)));
    }

    fun init_module(s: &signer) {
        move_to(s, State::Empty)
    }
    spec init_module(s: &signer) {
        use 0x1::signer;
        pragma opaque = true;
        modifies State[signer::address_of(s)];
        ensures [inferred] publish<State>(signer::address_of(s), State::Empty{});
        aborts_if [inferred] exists<State>(signer::address_of(s));
    }

    #[persistent]
    fun storable_add(x: u64, y: u64): u64 {
        x + y
    }
    spec storable_add(x: u64, y: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == x + y;
        aborts_if [inferred] x + y > MAX_U64;
    }

    #[persistent]
    fun storable_sub(x: u64, y: u64): u64 {
        x - y
    }
    spec storable_sub(x: u64, y: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == x - y;
        aborts_if [inferred] x < y;
    }

    entry fun number(s: &signer, x: u64) acquires State {
        process(s, Input::Number(x))
    }
    spec number {
        use 0x1::signer;
        pragma opaque = true, aborts_if_is_partial = true;
        modifies State[signer::address_of(s)];
        ensures [inferred] ensures_of<process>(s, Input::Number(x));
    } proof {
        split State[address_of(s)];
    }

    entry fun add(s: &signer) acquires State {
        process(s, Input::Add)
    }
    spec add(s: &signer) {
        use 0x1::signer;
        pragma opaque = true, aborts_if_is_partial = true;
        modifies State[signer::address_of(s)];
        ensures [inferred] ensures_of<process>(s, Input::Add{});
    }

    entry fun sub(s: &signer) acquires State {
        process(s, Input::Sub)
    }
    spec sub(s: &signer) {
        use 0x1::signer;
        pragma opaque = true, aborts_if_is_partial = true;
        modifies State[signer::address_of(s)];
        ensures [inferred] ensures_of<process>(s, Input::Sub{});
    }

    fun view(s: &signer): u64 acquires State {
        match (&State[address_of(s)]) {
            Value(x) => *x,
            _ => abort EINVALID_INPUT
        }
    }
    spec view(s: &signer): u64 {
        use 0x1::signer;
        pragma opaque = true;
        ensures [inferred] (State[signer::address_of(s)] is Value) ==> result == State[signer::address_of(s)].Value.0;
        aborts_if [inferred] State[signer::address_of(s)] is Empty | Continuation;
        aborts_if [inferred] !exists<State>(signer::address_of(s));
    }

}
