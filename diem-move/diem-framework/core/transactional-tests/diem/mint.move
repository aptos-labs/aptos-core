//# init --parent-vasps Test Alice

// Test the mint flow

//# publish
module Test::Holder {
    struct Holder<T> has key { x: T }
    public fun hold<T: store>(account: &signer, x: T)  {
        move_to(account, Holder<T> { x })
    }
}

// Minting from a privileged account should work
//
//# run --admin-script --signers DiemRoot TreasuryCompliance --show-events
script {
    use DiemFramework::XUS::XUS;
    use DiemFramework::Diem;
    use Test::Holder;

    fun main(_dr: signer, account: signer) {
        // mint 100 coins and check that the market cap increases appropriately
        let old_market_cap = Diem::market_cap<XUS>();
        let coin = Diem::mint<XUS>(&account, 100);
        assert!(Diem::value<XUS>(&coin) == 100, 8000);
        assert!(Diem::market_cap<XUS>() == old_market_cap + 100, 8001);

        // get rid of the coin
        Holder::hold(&account, coin)
    }
}

// Minting from a privileged account should work
//
//# run --admin-script --signers DiemRoot Alice
script {
    use DiemFramework::XUS::XUS;
    use DiemFramework::Diem;

    fun main(_dr: signer, account: signer) {
        let coin = Diem::mint<XUS>(&account, 100);
        Diem::destroy_zero(coin)
    }
}
// Will abort because sender doesn't have the mint capability.
