module econia::txn_generator_utils {
    use econia::market;
    use econia::incentives;
    use econia::assets::{Self, BC, QC, UC, AC, DC, EC, FC, GC, HC, IC, JC, KC, LC};
    use econia::user;
    use aptos_framework::signer;
    // use aptos_framework::account;

    const E_NUM_MARKETS_ZERO: u64 = 101;
    const E_NUM_MARKETS_HIGH: u64 = 102;

    /// Lot size for pure coin test market.
    const LOT_SIZE_COIN: u64 = 2;
    /// Tick size for pure coin test market.
    const TICK_SIZE_COIN: u64 = 3;
    /// Minimum size for pure coin test market.
    const MIN_SIZE_COIN: u64 = 4;
    /// Underwriter ID for generic test market.
    const UNDERWRITER_ID: u64 = 345;
    /// Custodian ID flag for no custodian.
    const NO_CUSTODIAN: u64 = 0;
    /// Market ID for pure coin test market.
    const MARKET_ID_COIN: u64 = 1;
    /// Flag for ask side.
    const ASK: bool = true;
    /// Flag for bid side.
    const BID: bool = false;
    /// Flag to abort during a self match.
    const ABORT: u8 = 0;
    /// Flag to cancel maker order only during a self match.
    const CANCEL_MAKER: u8 = 2;

    public entry fun register_multiple_markets(publisher: &signer, num_markets: u64) {
        assert!(num_markets > 0, E_NUM_MARKETS_ZERO);
        assert!(num_markets < 12, E_NUM_MARKETS_HIGH);

        if (num_markets > 0) {
            register_market<AC, QC>(publisher);
        };
        if (num_markets > 1) {
            register_market<BC, QC>(publisher);
        };
        if (num_markets > 2) {
            register_market<DC, QC>(publisher);
        };
        if (num_markets > 3) {
            register_market<EC, QC>(publisher);
        };
        if (num_markets > 4) {
            register_market<FC, QC>(publisher);
        };
        if (num_markets > 5) {
            register_market<GC, QC>(publisher);
        };
        if (num_markets > 6) {
            register_market<HC, QC>(publisher);
        };
        if (num_markets > 7) {
            register_market<IC, QC>(publisher);
        };
        if (num_markets > 8) {
            register_market<JC, QC>(publisher);
        };
        if (num_markets > 9) {
            register_market<KC, QC>(publisher);
        };
        if (num_markets > 10) {
            register_market<LC, QC>(publisher);
        };
    }

    public entry fun register_market<BaseCoinType, QuoteCoinType>(publisher: &signer) {
        assert!(@econia == signer::address_of(publisher), 101);
        // Get market registration fee.
        let fee = incentives::get_market_registration_fee();
        // Register pure coin market.
        market::register_market_base_coin<BaseCoinType, QuoteCoinType, UC>(
            LOT_SIZE_COIN, TICK_SIZE_COIN, MIN_SIZE_COIN,
            assets::mint(publisher, fee));
    }

    public entry fun register_market_accounts<BaseCoinType, QuoteCoinType>(user: &signer, market_id: u64) {
        user::register_market_account<BaseCoinType, QuoteCoinType>(user, market_id, NO_CUSTODIAN);
    }

    public entry fun deposit_coins<BaseCoinType, QuoteCoinType>(user: &signer, publisher: &signer, market_id: u64) {
        user::deposit_coins<QuoteCoinType>(signer::address_of(user), market_id, NO_CUSTODIAN, assets::mint<QuoteCoinType>(publisher, 1000000));
        user::deposit_coins<BaseCoinType>(signer::address_of(user), market_id, NO_CUSTODIAN, assets::mint<BaseCoinType>(publisher, 1000000));
    }

    public entry fun place_bid_limit_order<BaseCoinType, QuoteCoinType>(user: &signer, size: u64, price: u64, market_id: u64) {
        market::place_limit_order_user<BaseCoinType, QuoteCoinType>(user, market_id, @econia, BID, size, price, 2, CANCEL_MAKER);
    }

    public entry fun place_ask_limit_order<BaseCoinType, QuoteCoinType>(user: &signer, size: u64, price: u64, market_id: u64) {
        market::place_limit_order_user<BaseCoinType, QuoteCoinType>(user, market_id, @econia, ASK, size, price, 2, CANCEL_MAKER);
    }

    public entry fun place_bid_market_order<BaseCoinType, QuoteCoinType>(user: &signer, size: u64, market_id: u64) {
        market::place_market_order_user<BaseCoinType, QuoteCoinType>(user, market_id, @econia, BID, size, CANCEL_MAKER);
    }

    public entry fun place_ask_market_order<BaseCoinType, QuoteCoinType>(user: &signer, size: u64, market_id: u64) {
        market::place_market_order_user<BaseCoinType, QuoteCoinType>(user, market_id, @econia, ASK, size, CANCEL_MAKER);
    }
}
