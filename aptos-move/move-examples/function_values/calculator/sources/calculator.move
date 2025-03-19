module account::calculator {
    use std::signer::address_of;

    const EINVALID_INPUT: u64 = 1;

    /// Input to the calculator
    enum Input {
        Number(u64),
        Add,
        Sub,
    }

    /// State of the calculator
    enum State has key, copy, drop {
        Empty,
        Value(u64),
        WaitForNumber(|u64|u64)
    }

    /// Process input in the current state.
    fun process(s: &signer, input: Input) acquires State {
        let addr = address_of(s);
        match ((move_from<State>(addr), input)) {
            (Empty, Number(x)) => move_to(s, State::Value(x)),
            (Value(_), Number(x)) => move_to(s, State::Value(x)),
            (Value(x), Add) => move_to(s, State::WaitForNumber(|y| storable_add(x, y))),
            (Value(x), Sub) => move_to(s, State::WaitForNumber(|y| storable_sub(x, y))),
            (WaitForNumber(f), Number(x)) => move_to(s, State::Value(f(x))),
            (_, _) => abort EINVALID_INPUT
        }
    }

    #[persistent]
    fun storable_add(x: u64, y: u64): u64 {
        x + y
    }

    #[persistent]
    fun storable_sub(x: u64, y: u64): u64 {
        x - y
    }

    fun init_module(s: &signer) {
        move_to(s, State::Empty)
    }


    /// Entry point functions
    entry fun number(s: &signer, x: u64) acquires State {
        process(s, Input::Number(x))
    }

    entry fun add(s: &signer) acquires State {
        process(s, Input::Add)
    }

    entry fun sub(s: &signer) acquires State {
        process(s, Input::Sub)
    }

    #[view]
    fun view(a: address): u64 acquires State {
        match (&State[a]) {
            Value(x) => *x,
            _ => abort EINVALID_INPUT
        }
    }
}
