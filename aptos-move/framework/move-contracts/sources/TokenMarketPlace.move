module MoveContracts::TokenMarketPlace {
    use AptosFramework::Token::{Self, TokenId};
    use AptosFramework::Coin::Self;
    use AptosFramework::Table::{Self, Table};
    use AptosFramework::TypeInfo::{Self, TypeInfo};
    use Std::Signer;

    const ETOKEN_MARKET_PLACE_ALREADY_EXISTS: u64 = 0;
    const ENO_SUFFICIENT_TOKEN_BALANCE_TO_SELL: u64 = 1;
    const ESELL_ORDER_EXISTED: u64 = 2;
    const ESELL_ORDER_NOT_EXISTED: u64 = 3;
    const ENO_SUFFICIENT_COIN_BALANCE_TO_BUY: u64 = 4;
    const ENO_TOKEN_TO_SELL: u64 = 5;
    const ENO_ENOUGH_TOKEN_TO_SELL: u64 = 6;
    const ELIST_PRICE_IS_HIGHER: u64 = 7;
    const ECURRENCY_TYPE_NO_MATCH: u64 = 8;
    const ETOKEN_MARKET_PLACE_DOESNOT_EXISTS: u64 = 9;
    const ENO_SELLER_IN_MARKET_PLACE: u64 = 10;

    struct TokenMarketPlace has key {
        seller_inventoies: Table<address, SellerInventory>
    }

    struct SellerInventory has store {
        orders: Table<TokenId, SellOrder>,
        // for one address, the same token has same price
    }

    struct SellOrder has store, drop {
        id: TokenId,
        amount: u64,
        list_price: u64,
        coin_info: TypeInfo
    }

    public(script) fun create_marketplace(admin: &signer) {
        let admin_addr = Signer::address_of(admin);
        // init a token store
        Token::initialize_token_store(admin);

        assert!(!exists<TokenMarketPlace>(admin_addr), ETOKEN_MARKET_PLACE_ALREADY_EXISTS);
        move_to(
            admin,
            TokenMarketPlace{
                seller_inventoies: Table::new()
            },
        );
    }

    public fun init_seller_inventory(seller: &signer, market_place: address) acquires TokenMarketPlace {
        let token_market_place = borrow_global<TokenMarketPlace>(market_place);
        let sell_addr = Signer::address_of(seller);
        if (Table::empty(&token_market_place.seller_inventoies)) {
            let market = borrow_global_mut<TokenMarketPlace>(market_place);
            Table::add(&mut market.seller_inventoies, &sell_addr, SellerInventory{
                orders: Table::new()
            });
        };
    }

    public fun init_token_store(user: &signer) {
        Token::initialize_token_store(user);
    }

    public(script) fun sell<CoinType>(
        seller: &signer,
        market_place: address,
        id: &TokenId,
        price: u64,
        amount: u64) acquires TokenMarketPlace {
        // validate if the seller has the token balance
        let balance = Token::balanceOf(Signer::address_of(seller), id);
        assert!(balance >= amount, ENO_SUFFICIENT_TOKEN_BALANCE_TO_SELL);

        // init sell inventory
        init_seller_inventory(seller, market_place);

        // validate if the marketplace already has the sell order.
        let seller_listing = Table::borrow(
            &borrow_global<TokenMarketPlace>(market_place).seller_inventoies,
            &Signer::address_of(seller)
        );
        assert!(!Table::contains(&seller_listing.orders, id), ESELL_ORDER_EXISTED);

        //construct a sell order and add it to the inventory
        let sell_order = SellOrder{
            id: *id,
            amount,
            list_price: price,
            coin_info: TypeInfo::type_of<CoinType>()
        };
        let seller_inventory = Table::borrow_mut(
            &mut borrow_global_mut<TokenMarketPlace>(market_place).seller_inventoies,
            &Signer::address_of(seller)
        );
        Table::add(&mut seller_inventory.orders, id, sell_order);

        // transfer the capability to the marketplace
        Token::approve(seller, market_place, id, true);
    }

    public(script) fun cancel_sell(
        seller: &signer,
        market_place: address,
        id: &TokenId,
    ) acquires TokenMarketPlace {
        // validate if the marketplace already has the sell order.
        assert!(exists<TokenMarketPlace>(market_place), ETOKEN_MARKET_PLACE_DOESNOT_EXISTS);
        assert!(
            Table::contains(
                &borrow_global<TokenMarketPlace>(market_place).seller_inventoies,
                &Signer::address_of(seller)),
            ENO_SELLER_IN_MARKET_PLACE);

        let seller_listing = Table::borrow(
            &borrow_global<TokenMarketPlace>(market_place).seller_inventoies,
            &Signer::address_of(seller)
        );
        assert!(Table::contains(&seller_listing.orders, id), ESELL_ORDER_NOT_EXISTED);

        // remove the marketplace transfer capability
        Token::approve(seller, market_place, id, false);
        // remove the sell order from the listing
        remove_sell_order(market_place, Signer::address_of(seller), id);
    }

    fun remove_sell_order(market_place: address, seller: address, id: &TokenId) acquires TokenMarketPlace {
        let seller_listing = Table::borrow_mut(
            &mut borrow_global_mut<TokenMarketPlace>(market_place).seller_inventoies,
            &seller
        );
        Table::remove(&mut seller_listing.orders, id);

        // if the seller has not sell_orders. remove the seller
        if (Table::empty(&seller_listing.orders)) {
            let SellerInventory{
                orders
            } = Table::remove(
                &mut borrow_global_mut<TokenMarketPlace>(market_place).seller_inventoies,
                &seller);
            Table::destroy_empty(orders);
        }
    }

    public(script) fun buy_from<CoinType>(
        buyer: &signer,
        market_place: address,
        owner: address,
        id: &TokenId,
        price: u64,
        amount: u64) acquires TokenMarketPlace {
        // validate if the buyer has the amount balance
        let buyer_addr = Signer::address_of(buyer);
        let balance = Coin::balance<CoinType>(buyer_addr);
        assert!(balance >= price * amount, ENO_SUFFICIENT_COIN_BALANCE_TO_BUY);

        // validate if there is a sell order with right amount
        let seller_listing = Table::borrow(
            &borrow_global<TokenMarketPlace>(market_place).seller_inventoies,
            &owner
        );
        assert!(Table::contains(&seller_listing.orders, id), ENO_TOKEN_TO_SELL);
        let sell_order = Table::borrow(&seller_listing.orders, id);
        assert!(sell_order.coin_info == TypeInfo::type_of<CoinType>(), ECURRENCY_TYPE_NO_MATCH);
        assert!(sell_order.amount >= amount, ENO_SUFFICIENT_TOKEN_BALANCE_TO_SELL);
        assert!(sell_order.list_price <= price, ELIST_PRICE_IS_HIGHER);

        // validate seller indeed owns enough token
        assert!(Token::balanceOf(owner, id) >= amount, ENO_SUFFICIENT_TOKEN_BALANCE_TO_SELL);

        //transfer the coin from buyer to seller
        let coins = Coin::withdraw<CoinType>(buyer, price * amount);
        Coin::deposit<CoinType>(owner, coins);

        // update sell order
        let sell_amount = sell_order.amount;
        let seller_listing = Table::borrow_mut(
            &mut borrow_global_mut<TokenMarketPlace>(market_place).seller_inventoies,
            &owner
        );

        if (sell_amount == amount) {
            remove_sell_order(market_place, owner, id);
        } else {
            let sell = Table::borrow_mut(&mut seller_listing.orders, id);
            sell.amount = sell.amount - amount;
        };

        // transfer token from seller to buyer
        Token::initialize_token_store(buyer);
        Token::initialize_token(buyer, id);
        Token::transfer(market_place, owner, buyer_addr, id, amount);
    }

    #[test_only]
    struct FakeMoney {}

    #[test_only]
    public(script) fun create_marketplace_for_testing(
        marketplace: &signer,
        seller: &signer,
        buyer: &signer,
        sell_price: u64,
        sell_amount: u64,
        buy_price: u64,
        buy_amount: u64,
        tid: &TokenId,
    ) acquires TokenMarketPlace {
        let seller_addr = Signer::address_of(seller);
        create_marketplace(marketplace);
        let market_addr = Signer::address_of(marketplace);

        sell<FakeMoney>(seller, market_addr, tid, sell_price, sell_amount);
        // create coin for testing
        Coin::initialize<FakeMoney>(buyer, b"Fake money", b"FKM", 0, true);
        Coin::register<FakeMoney>(seller);
        Coin::register<FakeMoney>(buyer);
        Coin::mint<FakeMoney>(buyer, Signer::address_of(buyer), 100);

        buy_from<FakeMoney>(buyer, market_addr, seller_addr, tid, buy_price, buy_amount);
    }

    #[test(marketplace = @0x3, seller = @0x1, buyer = @0x2)]
    public(script) fun test_happy_sell_and_buy(
        marketplace: &signer,
        seller: &signer,
        buyer: &signer
    ) acquires TokenMarketPlace {
        let token_id = Token::create_collection_and_token(seller, 1, 2, 1);
        create_marketplace_for_testing(
            marketplace,
            seller,
            buyer,
            2,
            1,
            2,
            1,
            &token_id
        );
    }

    #[test(marketplace = @0x4, seller = @0x5, buyer = @0x2)]
    public(script) fun test_cancel_sell_order(
        marketplace: &signer,
        seller: &signer,
        buyer: &signer
    ) acquires TokenMarketPlace {
        let token_id = Token::create_collection_and_token(seller, 1, 2, 1);
        create_marketplace_for_testing(
            marketplace,
            seller,
            buyer,
            0,
            1,
            0,
            0,
            &token_id
        );
        cancel_sell(seller, Signer::address_of(marketplace), &token_id);
    }
}
