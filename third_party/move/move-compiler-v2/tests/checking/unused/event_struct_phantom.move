module 0x1::event {
    public fun emit<T: drop + store>(_msg: T) {
        abort 0
    }
}

module 0x42::m {
    use 0x1::event;

    struct Coin {}

    // Event struct with phantom type parameter
    struct Deposit<phantom CoinType> has drop, store {
        account: address,
        amount: u64,
    }

    public fun test() {
        event::emit(Deposit<Coin> { account: @0x1, amount: 100 });
    }
}
