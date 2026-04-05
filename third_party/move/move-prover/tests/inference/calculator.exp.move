// no_ci: TODO(#19277): Z3 trace non-determinism causes baseline mismatch on CI
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
        ensures [inferred] (old(State[signer::address_of(s)]) is Continuation) ==> (S6.. |~ publish<State>(signer::address_of(s), State::Value(S1..S6 |~ {{ let __f = old(State[signer::address_of(s)]).Continuation.0; result_of<__f>(input.0) }})));
        ensures [inferred] (old(State[signer::address_of(s)]) is Value) && (input is Number) ==> (S1.. |~ publish<State>(signer::address_of(s), State::Value(input.0)));
        ensures [inferred] (old(State[signer::address_of(s)]) is Value) && (input is Add) ==> (S1.. |~ publish<State>(signer::address_of(s), State::Continuation({
            let __cap = old(State[signer::address_of(s)]).Value.0;
            |arg0| storable_add(__cap, arg0)
        })));
        ensures [inferred] (old(State[signer::address_of(s)]) is Value) && (input is Sub) ==> (S1.. |~ publish<State>(signer::address_of(s), State::Continuation({
            let __cap = old(State[signer::address_of(s)]).Value.0;
            |arg0| storable_sub(__cap, arg0)
        })));
        ensures [inferred] (old(State[signer::address_of(s)]) is Empty) && (input is Number) ==> (S1.. |~ publish<State>(signer::address_of(s), State::Value(input.0)));
        ensures [inferred] (old(State[signer::address_of(s)]) is Empty) && (input is Add | Sub) ==> (S6.. |~ publish<State>(signer::address_of(s), State::Value(S1..S6 |~ {{ let __f = old(State[signer::address_of(s)]).Continuation.0; result_of<__f>(input.0) }})));
        ensures [inferred] ..S1 |~ remove<State>(signer::address_of(s));
        aborts_if [inferred] S6 |~ (State[signer::address_of(s)] is Continuation) && exists<State>(signer::address_of(s));
        aborts_if [inferred] S1 |~ (State[signer::address_of(s)] is Continuation) && {{ let __f = State[signer::address_of(s)].Continuation.0; aborts_of<__f>(input.0) }};
        aborts_if [inferred] (State[signer::address_of(s)] is Continuation) && (input is Add | Sub);
        aborts_if [inferred] S1 |~ (State[signer::address_of(s)] is Value) && ((input is Number) && exists<State>(signer::address_of(s)));
        aborts_if [inferred] S1 |~ (State[signer::address_of(s)] is Value) && ((input is Add) && exists<State>(signer::address_of(s)));
        aborts_if [inferred] S1 |~ (State[signer::address_of(s)] is Value) && ((input is Sub) && exists<State>(signer::address_of(s)));
        aborts_if [inferred] S1 |~ (State[signer::address_of(s)] is Empty) && ((input is Number) && exists<State>(signer::address_of(s)));
        aborts_if [inferred] S6 |~ (input is Add | Sub) && ((State[signer::address_of(s)] is Empty) && exists<State>(signer::address_of(s)));
        aborts_if [inferred] S1 |~ (input is Add | Sub) && ((State[signer::address_of(s)] is Empty) && {{ let __f = State[signer::address_of(s)].Continuation.0; aborts_of<__f>(input.0) }});
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

// TODO(#19277): stored function value verification
/*
Verification: exiting with verification errors
error: function does not abort under this condition
   ┌─ calculator.enriched.move:50:9
   │
50 │         aborts_if [inferred] S6 |~ (State[signer::address_of(s)] is Continuation) && exists<State>(signer::address_of(s));
   │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   │
   =     at calculator.enriched.move:22: process
   =         s = <redacted>
   =         input = <redacted>
   =     at calculator.enriched.move:23: process
   =     at ../move-stdlib/sources/signer.move:26: address_of
   =         s = <redacted>
   =     at ../move-stdlib/sources/signer.move:27: address_of
   =         result = <redacted>
   =     at ../move-stdlib/sources/signer.move:28: address_of
   =     at calculator.enriched.move:24: process
   =         <redacted> = <redacted>
   =     at calculator.enriched.move:25: process
   =     at calculator.enriched.move:26: process
   =         <redacted> = <redacted>
   =     at calculator.enriched.move:24: process
   =     at calculator.enriched.move:32: process
   =     at calculator.enriched.move:50: process (spec)

error: function does not abort under this condition
   ┌─ calculator.enriched.move:57:9
   │
57 │         aborts_if [inferred] S6 |~ (input is Add | Sub) && ((State[signer::address_of(s)] is Empty) && exists<State>(signer::address_of(s)));
   │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   │
   =     at calculator.enriched.move:22: process
   =         s = <redacted>
   =         input = <redacted>
   =     at calculator.enriched.move:23: process
   =     at ../move-stdlib/sources/signer.move:26: address_of
   =         s = <redacted>
   =     at ../move-stdlib/sources/signer.move:27: address_of
   =         result = <redacted>
   =     at ../move-stdlib/sources/signer.move:28: address_of
   =     at calculator.enriched.move:24: process
   =         <redacted> = <redacted>
   =     at calculator.enriched.move:25: process
   =     at calculator.enriched.move:26: process
   =     at calculator.enriched.move:27: process
   =     at calculator.enriched.move:28: process
   =         <redacted> = <redacted>
   =     at calculator.enriched.move:28: process
   =     at calculator.enriched.move:24: process
   =     at calculator.enriched.move:32: process
   =     at calculator.enriched.move:50: process (spec)
   =     at calculator.enriched.move:51: process (spec)
   =     at calculator.enriched.move:52: process (spec)
   =     at calculator.enriched.move:53: process (spec)
   =     at calculator.enriched.move:54: process (spec)
   =     at calculator.enriched.move:55: process (spec)
   =     at calculator.enriched.move:56: process (spec)
   =     at calculator.enriched.move:57: process (spec)

error: abort not covered by any of the `aborts_if` clauses
   ┌─ calculator.enriched.move:33:5
   │
29 │               (Continuation(f), Number(x)) => move_to(s, State::Value(f(x))),
   │                                               ------------------------------ abort happened here with execution failure
   ·
33 │ ╭     spec process(s: &signer, input: Input) {
34 │ │         use 0x1::signer;
35 │ │         pragma opaque = true;
36 │ │         modifies State[signer::address_of(s)];
   · │
62 │ │         aborts_if [inferred] aborts_of<signer::address_of>(s);
63 │ │     }
   │ ╰─────^
   │
   =     at calculator.enriched.move:22: process
   =         s = <redacted>
   =         input = <redacted>
   =     at calculator.enriched.move:23: process
   =     at ../move-stdlib/sources/signer.move:26: address_of
   =         s = <redacted>
   =     at ../move-stdlib/sources/signer.move:27: address_of
   =         result = <redacted>
   =     at ../move-stdlib/sources/signer.move:28: address_of
   =     at calculator.enriched.move:24: process
   =         <redacted> = <redacted>
   =     at calculator.enriched.move:25: process
   =     at calculator.enriched.move:26: process
   =     at calculator.enriched.move:27: process
   =     at calculator.enriched.move:28: process
   =     at calculator.enriched.move:29: process
   =         f = <redacted>
   =         <redacted> = <redacted>
   =     at calculator.enriched.move:29: process
   =         ABORTED

error: function does not abort under this condition
    ┌─ calculator.enriched.move:111:9
    │
111 │         aborts_if [inferred] aborts_of<process>(s, Input::Number(x));
    │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    │
    =     at calculator.enriched.move:103: number
    =         s = <redacted>
    =         x = <redacted>
    =     at calculator.enriched.move:104: number
    =     at calculator.enriched.move:105: number
    =     at calculator.enriched.move:111: number (spec)

error: function does not abort under this condition
    ┌─ calculator.enriched.move:123:9
    │
123 │         aborts_if [inferred] aborts_of<process>(s, Input::Add{});
    │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    │
    =     at calculator.enriched.move:115: add
    =         s = <redacted>
    =     at calculator.enriched.move:116: add
    =     at calculator.enriched.move:117: add
    =     at calculator.enriched.move:123: add (spec)

error: function does not abort under this condition
    ┌─ calculator.enriched.move:135:9
    │
135 │         aborts_if [inferred] aborts_of<process>(s, Input::Sub{});
    │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    │
    =     at calculator.enriched.move:127: sub
    =         s = <redacted>
    =     at calculator.enriched.move:128: sub
    =     at calculator.enriched.move:129: sub
    =     at calculator.enriched.move:135: sub (spec)
*/
