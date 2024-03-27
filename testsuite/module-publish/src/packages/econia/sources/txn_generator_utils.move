module econia::txn_generator_utils {
    use econia::market;
    use econia::incentives;
    use econia::assets::{Self, QC, UC, AAC, ABC, ACC, ADC, AEC, AFC, AGC, AHC, AIC, AJC, AKC, ALC, AMC, ANC, AOC, APC, AQC, ARC, ASC, ATC, AUC, AVC, AWC, AXC, AYC, AZC, BAC, BBC, BCC, BDC, BEC, BFC, BGC, BHC, BIC, BJC, BKC, BLC, BMC, BNC, BOC, BPC, BQC, BRC, BSC, BTC, BUC, BVC, BWC, BXC, BYC, BZC, CAC, CBC, CCC, CDC, CEC, CFC, CGC, CHC, CIC, CJC, CKC, CLC, CMC, CNC, COC, CPC, CQC, CRC, CSC, CTC, CUC, CVC, CWC, CXC, CYC, CZC, DAC, DBC, DCC, DDC, DEC, DFC, DGC, DHC, DIC, DJC, DKC, DLC, DMC, DNC, DOC, DPC, DQC, DRC, DSC, DTC, DUC, DVC, DWC, DXC, DYC, DZC};
    use econia::user;
    use aptos_framework::signer;
    use std::vector;
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
        assert!(num_markets < 105, E_NUM_MARKETS_HIGH);

        if (num_markets > 0) {
            register_market<AAC, QC>(publisher);
        };
        if (num_markets > 1) {
            register_market<ABC, QC>(publisher);
        };
        if (num_markets > 2) {
            register_market<ACC, QC>(publisher);
        };
        if (num_markets > 3) {
            register_market<ADC, QC>(publisher);
        };
        if (num_markets > 4) {
            register_market<AEC, QC>(publisher);
        };
        if (num_markets > 5) {
            register_market<AFC, QC>(publisher);
        };
        if (num_markets > 6) {
            register_market<AGC, QC>(publisher);
        };
        if (num_markets > 7) {
            register_market<AHC, QC>(publisher);
        };
        if (num_markets > 8) {
            register_market<AIC, QC>(publisher);
        };
        if (num_markets > 9) {
            register_market<AJC, QC>(publisher);
        };
        if (num_markets > 10) {
            register_market<AKC, QC>(publisher);
        };
        if (num_markets > 11) {
            register_market<ALC, QC>(publisher);
        };
        if (num_markets > 12) {
            register_market<AMC, QC>(publisher);
        };
        if (num_markets > 13) {
            register_market<ANC, QC>(publisher);
        };
        if (num_markets > 14) {
            register_market<AOC, QC>(publisher);
        };
        if (num_markets > 15) {
            register_market<APC, QC>(publisher);
        };
        if (num_markets > 16) {
            register_market<AQC, QC>(publisher);
        };
        if (num_markets > 17) {
            register_market<ARC, QC>(publisher);
        };
        if (num_markets > 18) {
            register_market<ASC, QC>(publisher);
        };
        if (num_markets > 19) {
            register_market<ATC, QC>(publisher);
        };
        if (num_markets > 20) {
            register_market<AUC, QC>(publisher);
        };
        if (num_markets > 21) {
            register_market<AVC, QC>(publisher);
        };
        if (num_markets > 22) {
            register_market<AWC, QC>(publisher);
        };
        if (num_markets > 23) {
            register_market<AXC, QC>(publisher);
        };
        if (num_markets > 24) {
            register_market<AYC, QC>(publisher);
        };
        if (num_markets > 25) {
            register_market<AZC, QC>(publisher);
        };
        if (num_markets > 26) {
            register_market<BAC, QC>(publisher);
        };
        if (num_markets > 27) {
            register_market<BBC, QC>(publisher);
        };
        if (num_markets > 28) {
            register_market<BCC, QC>(publisher);
        };
        if (num_markets > 29) {
            register_market<BDC, QC>(publisher);
        };
        if (num_markets > 30) {
            register_market<BEC, QC>(publisher);
        };
        if (num_markets > 31) {
            register_market<BFC, QC>(publisher);
        };
        if (num_markets > 32) {
            register_market<BGC, QC>(publisher);
        };
        if (num_markets > 33) {
            register_market<BHC, QC>(publisher);
        };
        if (num_markets > 34) {
            register_market<BIC, QC>(publisher);
        };
        if (num_markets > 35) {
            register_market<BJC, QC>(publisher);
        };
        if (num_markets > 36) {
            register_market<BKC, QC>(publisher);
        };
        if (num_markets > 37) {
            register_market<BLC, QC>(publisher);
        };
        if (num_markets > 38) {
            register_market<BMC, QC>(publisher);
        };
        if (num_markets > 39) {
            register_market<BNC, QC>(publisher);
        };
        if (num_markets > 40) {
            register_market<BOC, QC>(publisher);
        };
        if (num_markets > 41) {
            register_market<BPC, QC>(publisher);
        };
        if (num_markets > 42) {
            register_market<BQC, QC>(publisher);
        };
        if (num_markets > 43) {
            register_market<BRC, QC>(publisher);
        };
        if (num_markets > 44) {
            register_market<BSC, QC>(publisher);
        };
        if (num_markets > 45) {
            register_market<BTC, QC>(publisher);
        };
        if (num_markets > 46) {
            register_market<BUC, QC>(publisher);
        };
        if (num_markets > 47) {
            register_market<BVC, QC>(publisher);
        };
        if (num_markets > 48) {
            register_market<BWC, QC>(publisher);
        };
        if (num_markets > 49) {
            register_market<BXC, QC>(publisher);
        };
        if (num_markets > 50) {
            register_market<BYC, QC>(publisher);
        };
        if (num_markets > 51) {
            register_market<BZC, QC>(publisher);
        };
        if (num_markets > 52) {
            register_market<CAC, QC>(publisher);
        };
        if (num_markets > 53) {
            register_market<CBC, QC>(publisher);
        };
        if (num_markets > 54) {
            register_market<CCC, QC>(publisher);
        };
        if (num_markets > 55) {
            register_market<CDC, QC>(publisher);
        };
        if (num_markets > 56) {
            register_market<CEC, QC>(publisher);
        };
        if (num_markets > 57) {
            register_market<CFC, QC>(publisher);
        };
        if (num_markets > 58) {
            register_market<CGC, QC>(publisher);
        };
        if (num_markets > 59) {
            register_market<CHC, QC>(publisher);
        };
        if (num_markets > 60) {
            register_market<CIC, QC>(publisher);
        };
        if (num_markets > 61) {
            register_market<CJC, QC>(publisher);
        };
        if (num_markets > 62) {
            register_market<CKC, QC>(publisher);
        };
        if (num_markets > 63) {
            register_market<CLC, QC>(publisher);
        };
        if (num_markets > 64) {
            register_market<CMC, QC>(publisher);
        };
        if (num_markets > 65) {
            register_market<CNC, QC>(publisher);
        };
        if (num_markets > 66) {
            register_market<COC, QC>(publisher);
        };
        if (num_markets > 67) {
            register_market<CPC, QC>(publisher);
        };
        if (num_markets > 68) {
            register_market<CQC, QC>(publisher);
        };
        if (num_markets > 69) {
            register_market<CRC, QC>(publisher);
        };
        if (num_markets > 70) {
            register_market<CSC, QC>(publisher);
        };
        if (num_markets > 71) {
            register_market<CTC, QC>(publisher);
        };
        if (num_markets > 72) {
            register_market<CUC, QC>(publisher);
        };
        if (num_markets > 73) {
            register_market<CVC, QC>(publisher);
        };
        if (num_markets > 74) {
            register_market<CWC, QC>(publisher);
        };
        if (num_markets > 75) {
            register_market<CXC, QC>(publisher);
        };
        if (num_markets > 76) {
            register_market<CYC, QC>(publisher);
        };
        if (num_markets > 77) {
            register_market<CZC, QC>(publisher);
        };
        if (num_markets > 78) {
            register_market<DAC, QC>(publisher);
        };
        if (num_markets > 79) {
            register_market<DBC, QC>(publisher);
        };
        if (num_markets > 80) {
            register_market<DCC, QC>(publisher);
        };
        if (num_markets > 81) {
            register_market<DDC, QC>(publisher);
        };
        if (num_markets > 82) {
            register_market<DEC, QC>(publisher);
        };
        if (num_markets > 83) {
            register_market<DFC, QC>(publisher);
        };
        if (num_markets > 84) {
            register_market<DGC, QC>(publisher);
        };
        if (num_markets > 85) {
            register_market<DHC, QC>(publisher);
        };
        if (num_markets > 86) {
            register_market<DIC, QC>(publisher);
        };
        if (num_markets > 87) {
            register_market<DJC, QC>(publisher);
        };
        if (num_markets > 88) {
            register_market<DKC, QC>(publisher);
        };
        if (num_markets > 89) {
            register_market<DLC, QC>(publisher);
        };
        if (num_markets > 90) {
            register_market<DMC, QC>(publisher);
        };
        if (num_markets > 91) {
            register_market<DNC, QC>(publisher);
        };
        if (num_markets > 92) {
            register_market<DOC, QC>(publisher);
        };
        if (num_markets > 93) {
            register_market<DPC, QC>(publisher);
        };
        if (num_markets > 94) {
            register_market<DQC, QC>(publisher);
        };
        if (num_markets > 95) {
            register_market<DRC, QC>(publisher);
        };
        if (num_markets > 96) {
            register_market<DSC, QC>(publisher);
        };
        if (num_markets > 97) {
            register_market<DTC, QC>(publisher);
        };
        if (num_markets > 98) {
            register_market<DUC, QC>(publisher);
        };
        if (num_markets > 99) {
            register_market<DVC, QC>(publisher);
        };
        if (num_markets > 100) {
            register_market<DWC, QC>(publisher);
        };
        if (num_markets > 101) {
            register_market<DXC, QC>(publisher);
        };
        if (num_markets > 102) {
            register_market<DYC, QC>(publisher);
        };
        if (num_markets > 103) {
            register_market<DZC, QC>(publisher);
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
        user::deposit_coins<QuoteCoinType>(signer::address_of(user), market_id, NO_CUSTODIAN, assets::mint<QuoteCoinType>(publisher, 10000000000));
        user::deposit_coins<BaseCoinType>(signer::address_of(user), market_id, NO_CUSTODIAN, assets::mint<BaseCoinType>(publisher, 10000000000));
    }

    public entry fun place_bid_limit_order<BaseCoinType, QuoteCoinType>(user: &signer, size: u64, price: u64, market_id: u64) {
        market::place_limit_order_user<BaseCoinType, QuoteCoinType>(user, market_id, @econia, BID, size, price, 3, ABORT);
    }

    public entry fun place_ask_limit_order<BaseCoinType, QuoteCoinType>(user: &signer, size: u64, price: u64, market_id: u64) {
        market::place_limit_order_user<BaseCoinType, QuoteCoinType>(user, market_id, @econia, ASK, size, price, 3, ABORT);
    }

    public entry fun place_bid_market_order<BaseCoinType, QuoteCoinType>(user: &signer, size: u64, market_id: u64) {
        market::place_market_order_user<BaseCoinType, QuoteCoinType>(user, market_id, @econia, BID, size, CANCEL_MAKER);
    }

    public entry fun place_ask_market_order<BaseCoinType, QuoteCoinType>(user: &signer, size: u64, market_id: u64) {
        market::place_market_order_user<BaseCoinType, QuoteCoinType>(user, market_id, @econia, ASK, size, CANCEL_MAKER);
    }

    struct Order has drop, store {
        market_id: u64,
        direction: bool,
        order_id: u128,
    }

    struct Orders has key, drop {
        orders: vector<Order>
    }

    public entry fun place_limit_order<BaseCoinType, QuoteCoinType>(user: &signer, market_id: u64, direction: bool, size: u64, price: u64, restriction: u8, self_match_behavior: u8) acquires Orders {
        let (order_id, _, _, _) = market::place_limit_order_user<BaseCoinType, QuoteCoinType>(user, market_id, @econia, direction, size, price, restriction, self_match_behavior);

        if (exists<Orders>(signer::address_of(user))) {
            vector::push_back(&mut borrow_global_mut<Orders>(signer::address_of(user)).orders, 
                                Order{
                                    market_id: market_id, 
                                    direction: direction, 
                                    order_id: order_id
                                }
                            );
        } else {
            let orders = vector::empty();
            vector::push_back(&mut orders, Order{
                                            market_id: market_id, 
                                            direction: direction, 
                                            order_id: order_id
                                        }
                            );
            move_to<Orders>(user, Orders {orders: orders});
        }
    }

    public entry fun place_market_order<BaseCoinType, QuoteCoinType>(user: &signer, market_id: u64, direction: bool, size: u64, self_match_behavior: u8) {
        market::place_market_order_user<BaseCoinType, QuoteCoinType>(user, market_id, @econia, direction, size, self_match_behavior);
    }

    public entry fun place_cancel_order(user: &signer) acquires Orders {
        if (exists<Orders>(signer::address_of(user))) {
            let orders = borrow_global_mut<Orders>(signer::address_of(user));
            if (!vector::is_empty(&orders.orders)) {
                let order = vector::pop_back(&mut orders.orders);
                market::cancel_order_user(user, order.market_id, order.direction, order.order_id);                
            }
        }
    }
}