/// This implements the functionality of a calculator.
/// One enters a series of keys, like '1', '+', '2', '-', '3`.
/// Each key is represented by a single transaction.
/// The state is represented using storable functions, for
/// the situation of an incomplete operation, e.g. after
/// typing '1 + ..'

//# publish
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


    fun init_module(s: &signer) {
        move_to(s, State::Empty)
    }


    #[persistent]
    fun storable_add(x: u64, y: u64): u64 {
        x + y
    }

    #[persistent]
    fun storable_sub(x: u64, y: u64): u64 {
        x - y
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

    fun view(s: &signer): u64 acquires State {
        match (&State[address_of(s)]) {
            Value(x) => *x,
            _ => abort EINVALID_INPUT
        }
    }
}

//# run 0x66::calculator::init_module --signers 0x66

//# run 0x66::calculator::number --signers 0x66 --args 10

//# run 0x66::calculator::add --signers 0x66

//# run 0x66::calculator::number --signers 0x66 --args 20

//# run 0x66::calculator::sub --signers 0x66

//# run 0x66::calculator::number --signers 0x66 --args 5

//# run 0x66::calculator::view --signers 0x66
