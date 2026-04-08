module 0x66::calculator {
    use 0x1::signer::address_of;

    const EINVALID_INPUT: u64 = 1;

    /// Input provided
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
        pragma opaque = true;
        modifies State[signer::address_of(s)];
        ensures [inferred] (old(State[signer::address_of(s)]) is Continuation) ==> {
            let a = State::Value({
                let b = old(State[signer::address_of(s)]).Continuation.0;
                S1..S6 |~ result_of<b>(input.0)
            });
            S6.. |~ publish<State>(signer::address_of(s), a)
        };
        ensures [inferred] (old(State[signer::address_of(s)]) is Value) && (input is Number) ==> (S1.. |~ publish<State>(signer::address_of(s), State::Value(input.0)));
        ensures [inferred] (old(State[signer::address_of(s)]) is Value) && (input is Add) ==> {
            let c = State::Continuation({
                let d = old(State[signer::address_of(s)]).Value.0;
                |x| storable_add(d, x)
            });
            S1.. |~ publish<State>(signer::address_of(s), c)
        };
        ensures [inferred] (old(State[signer::address_of(s)]) is Value) && (input is Sub) ==> {
            let e = State::Continuation({
                let f = old(State[signer::address_of(s)]).Value.0;
                |x| storable_sub(f, x)
            });
            S1.. |~ publish<State>(signer::address_of(s), e)
        };
        ensures [inferred] (old(State[signer::address_of(s)]) is Empty) && (input is Number) ==> (S1.. |~ publish<State>(signer::address_of(s), State::Value(input.0)));
        ensures [inferred] (old(State[signer::address_of(s)]) is Empty) && (input is Add | Sub) ==> {
            let a = State::Value({
                let b = old(State[signer::address_of(s)]).Continuation.0;
                S1..S6 |~ result_of<b>(input.0)
            });
            S6.. |~ publish<State>(signer::address_of(s), a)
        };
        ensures [inferred] ..S1 |~ remove<State>(signer::address_of(s));
        aborts_if [inferred] (State[signer::address_of(s)] is Continuation) && (S6 |~ exists<State>(signer::address_of(s)));
        aborts_if [inferred] (State[signer::address_of(s)] is Continuation) && {
            let c = State[signer::address_of(s)].Continuation.0;
            S1 |~ aborts_of<c>(input.0)
        };
        aborts_if [inferred] (State[signer::address_of(s)] is Continuation) && (input is Add | Sub);
        aborts_if [inferred] (State[signer::address_of(s)] is Value) && (S1 |~ (input is Number) && exists<State>(signer::address_of(s)));
        aborts_if [inferred] (State[signer::address_of(s)] is Value) && (S1 |~ (input is Add) && exists<State>(signer::address_of(s)));
        aborts_if [inferred] (State[signer::address_of(s)] is Value) && (S1 |~ (input is Sub) && exists<State>(signer::address_of(s)));
        aborts_if [inferred] (State[signer::address_of(s)] is Empty) && (S1 |~ (input is Number) && exists<State>(signer::address_of(s)));
        aborts_if [inferred] (input is Add | Sub) && ((State[signer::address_of(s)] is Empty) && (S6 |~ exists<State>(signer::address_of(s))));
        aborts_if [inferred] (input is Add | Sub) && ((State[signer::address_of(s)] is Empty) && {
            let d = State[signer::address_of(s)].Continuation.0;
            S1 |~ aborts_of<d>(input.0)
        });
        aborts_if [inferred] (State[signer::address_of(s)] is Empty) && (input is Add | Sub);
        aborts_if [inferred] (input is Add | Sub) && (State[signer::address_of(s)] is Empty);
        aborts_if [inferred] !exists<State>(signer::address_of(s));
        aborts_if [inferred] aborts_of<signer::address_of>(s);
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
        aborts_if [inferred] x - y < 0;
    }


    /// Entry point functions
    entry fun number(s: &signer, x: u64) acquires State {
        process(s, Input::Number(x))
    }
    spec number(s: &signer, x: u64) {
        use 0x1::signer;
        pragma opaque = true;
        modifies State[signer::address_of(s)];
        ensures [inferred] ensures_of<process>(s, Input::Number(x));
        aborts_if [inferred] aborts_of<process>(s, Input::Number(x));
    }


    entry fun add(s: &signer) acquires State {
        process(s, Input::Add)
    }
    spec add(s: &signer) {
        use 0x1::signer;
        pragma opaque = true;
        modifies State[signer::address_of(s)];
        ensures [inferred] ensures_of<process>(s, Input::Add{});
        aborts_if [inferred] aborts_of<process>(s, Input::Add{});
    }


    entry fun sub(s: &signer) acquires State {
        process(s, Input::Sub)
    }
    spec sub(s: &signer) {
        use 0x1::signer;
        pragma opaque = true;
        modifies State[signer::address_of(s)];
        ensures [inferred] ensures_of<process>(s, Input::Sub{});
        aborts_if [inferred] aborts_of<process>(s, Input::Sub{});
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
        ensures [inferred] result == State[signer::address_of(s)].Value.0;
        aborts_if [inferred] State[signer::address_of(s)] is Empty | Continuation;
        aborts_if [inferred] !exists<State>(signer::address_of(s));
        aborts_if [inferred] aborts_of<signer::address_of>(s);
    }

}
/*
Verification: Succeeded.
*/
