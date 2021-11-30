/// Module implementing an odd coin, where only odd number of coins can be
/// transferred each time.
module NamedAddr::MyOddCoin {
    use NamedAddr::BasicCoin;

    struct MyOddCoin has drop {}

    const ENOT_ODD: u64 = 0;

    public(script) fun transfer(from: signer, to: address, amount: u64) {
        assert!(amount % 2 == 1, ENOT_ODD);
        BasicCoin::transfer<MyOddCoin>(&from, to, amount, MyOddCoin {});
    }
}
