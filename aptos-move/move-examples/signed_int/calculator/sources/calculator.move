module account::calculator {
    use std::signer::address_of;

    /// Input to the calculator
    enum Input {
        Number(i64),
        Add(i8),
        Sub(i16),
        Mul(i32),
        Div(i128),
        Mod(i256),
    }

    /// State of the calculator
    enum State has key, copy, drop {
        Value(i64),
    }

    /// Process input in the current state.
    fun process(s: &signer, input: Input) acquires State {
        let addr = address_of(s);
        match ((move_from<State>(addr), input)) {
            (Value(_), Number(x)) => move_to(s, State::Value(x)),
            (Value(x), Add(y)) => move_to(s, State::Value(x + (y as i64))),
            (Value(x), Sub(y)) => move_to(s, State::Value(x - (y as i64))),
            (Value(x), Mul(y)) => move_to(s, State::Value(x * (y as i64))),
            (Value(x), Div(y)) => move_to(s, State::Value(x / (y as i64))),
            (Value(x), Mod(y)) => move_to(s, State::Value(x % (y as i64))),
        }
    }

    fun init_module(s: &signer) {
        move_to(s, State::Value(-1))
    }

    /// Entry point functions
    entry fun number(s: &signer, x: i64) acquires State {
        process(s, Input::Number(x));
    }

    entry fun add(s: &signer, x: i8) acquires State {
        process(s, Input::Add(x))
    }

    entry fun sub(s: &signer, x: i16) acquires State {
        process(s, Input::Sub(x))
    }

    entry fun mul(s: &signer, x: i32) acquires State {
        process(s, Input::Mul(x))
    }

    entry fun div(s: &signer, x: i128) acquires State {
        process(s, Input::Div(x))
    }

    entry fun mod(s: &signer, x: i256) acquires State {
        process(s, Input::Mod(x))
    }

    #[view]
    fun view_i8(a: address): i8 acquires State {
        match (&State[a]) {
            Value(x) => *x as i8,
        }
    }

    #[view]
    fun view_i16(a: address): i16 acquires State {
        match (&State[a]) {
            Value(x) => *x as i16,
        }
    }

    #[view]
    fun view_i32(a: address): i32 acquires State {
        match (&State[a]) {
            Value(x) => *x as i32,
        }
    }

    #[view]
    fun view_i64(a: address): i64 acquires State {
        match (&State[a]) {
            Value(x) => *x,
        }
    }

    #[view]
    fun view_i128(a: address): i128 acquires State {
        match (&State[a]) {
            Value(x) => *x as i128,
        }
    }

    #[view]
    fun view_i256(a: address): i256 acquires State {
        match (&State[a]) {
            Value(x) => *x as i256,
        }
    }
}
