// TODO(#19422): Z3 seed sensitivity causes timeout without inline-spec-lets
// flag: --inline-spec-lets
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
        let address_of_0 = signer::address_of(s);
        ensures [inferred] (old(State[address_of_0]) is Continuation) ==> {
            let a = State::Value(S1..S6 |~ result_of<old(State[address_of_0]).Continuation.0>(input.0));
            S6.. |~ publish<State>(address_of_0, a)
        };
        ensures [inferred] (old(State[address_of_0]) is Value) && (input is Number) ==> (S1.. |~ publish<State>(address_of_0, State::Value(input.0)));
        ensures [inferred] (old(State[address_of_0]) is Value) && (input is Add) ==> {
            let a = State::Continuation({
                let b = old(State[address_of_0]).Value.0;
                |x| storable_add(b, x)
            });
            S1.. |~ publish<State>(address_of_0, a)
        };
        ensures [inferred] (old(State[address_of_0]) is Value) && (input is Sub) ==> {
            let a = State::Continuation({
                let b = old(State[address_of_0]).Value.0;
                |x| storable_sub(b, x)
            });
            S1.. |~ publish<State>(address_of_0, a)
        };
        ensures [inferred] (old(State[address_of_0]) is Empty) && (input is Number) ==> (S1.. |~ publish<State>(address_of_0, State::Value(input.0)));
        ensures [inferred] (old(State[address_of_0]) is Empty) && (input is Add | Sub) ==> {
            let a = State::Value(S1..S6 |~ result_of<old(State[address_of_0]).Continuation.0>(input.0));
            S6.. |~ publish<State>(address_of_0, a)
        };
        ensures [inferred] ..S1 |~ remove<State>(address_of_0);
        aborts_if [inferred] (State[address_of_0] is Continuation) && (S6 |~ exists<State>(address_of_0));
        aborts_if [inferred] (State[address_of_0] is Continuation) && (S1 |~ aborts_of<State[address_of_0].Continuation.0>(input.0));
        aborts_if [inferred] (State[address_of_0] is Continuation) && (input is Add | Sub);
        aborts_if [inferred] (State[address_of_0] is Value) && (S1 |~ (input is Number) && exists<State>(address_of_0));
        aborts_if [inferred] (State[address_of_0] is Value) && (S1 |~ (input is Add) && exists<State>(address_of_0));
        aborts_if [inferred] (State[address_of_0] is Value) && (S1 |~ (input is Sub) && exists<State>(address_of_0));
        aborts_if [inferred] (State[address_of_0] is Empty) && (S1 |~ (input is Number) && exists<State>(address_of_0));
        aborts_if [inferred] (input is Add | Sub) && ((State[address_of_0] is Empty) && (S6 |~ exists<State>(address_of_0)));
        aborts_if [inferred] (input is Add | Sub) && ((State[address_of_0] is Empty) && (S1 |~ aborts_of<State[address_of_0].Continuation.0>(input.0)));
        aborts_if [inferred] (State[address_of_0] is Empty) && (input is Add | Sub);
        aborts_if [inferred] (input is Add | Sub) && (State[address_of_0] is Empty);
        aborts_if [inferred] !exists<State>(address_of_0);
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
        let address_of_0 = signer::address_of(s);
        ensures [inferred] result == State[address_of_0].Value.0;
        aborts_if [inferred] State[address_of_0] is Empty | Continuation;
        aborts_if [inferred] !exists<State>(address_of_0);
        aborts_if [inferred] aborts_of<signer::address_of>(s);
    }

}
/*
Verification: Succeeded.
*/
