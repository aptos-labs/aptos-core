module GoldCoin::GoldCoin {
    use Std::Signer;
    use BasicCoin::BasicCoin;

    struct GoldCoin has drop {}

    public fun setup_and_mint(account: &signer, amount: u64) {
        BasicCoin::publish_balance<GoldCoin>(account);
        BasicCoin::mint<GoldCoin>(Signer::address_of(account), amount, GoldCoin{});
    }

    public fun transfer(from: &signer, to: address, amount: u64) {
        BasicCoin::transfer<GoldCoin>(from, to, amount, GoldCoin {});
    }
}
