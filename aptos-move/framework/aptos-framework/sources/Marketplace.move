module Marketplace::Marketplace {
    use Std::GUID::{Self, ID};
    use Std::Signer;
    use AptosFramework::Table::{Self, Table};
    use AptosFramework::Token::{Self, Token};
    use AptosFramework::TestCoin;

    const ERROR_INVALID_BUYER: u64 = 0;

	struct MarketItem has key, store {
		seller: address,
        token: Token,
		price: u64,
	}

	struct Market has key {
		market_items: Table<ID, MarketItem>,
        listing_fee: u64,
	}

	public(script) fun init_market_script(market_owner: &signer, listing_fee: u64) {
		let market_items = Table::new<ID, MarketItem>();
		move_to<Market>(market_owner, Market {market_items, listing_fee})
	}

    public(script) fun list_token_for_sale_script(
        seller: &signer,
        creator: address,
        token_creation_num: u64,
		price: u64,
		market_owner_addr: address,
    ) acquires Market {
        let token_id = GUID::create_id(creator, token_creation_num);
        list_token_for_sale(seller, &token_id, price, market_owner_addr);
    }

    public(script) fun list_token_for_sale(
        seller: &signer,
        token_id: &ID,
		price: u64,
		market_owner_addr: address,
    ) acquires Market {
        let seller_addr = Signer::address_of(seller);
        let token = Token::withdraw_token(seller, token_id, 1);
        let token_id = *Token::token_id(&token);
		let market_items = &mut borrow_global_mut<Market>(market_owner_addr).market_items;
		Table::add(market_items, &token_id, MarketItem {seller: seller_addr, token: token, price: price})
    }

    public(script) fun buy_token_script(
        buyer: &signer,
        seller: address,
        creator: address,
        token_creation_num: u64,
		market_owner_addr: address,
    ) acquires Market {
        let token_id = GUID::create_id(creator, token_creation_num);
        buy_token(buyer, seller, &token_id, market_owner_addr);
	}

    public(script) fun buy_token(
        buyer: &signer,
        seller: address,
        token_id: &ID,
		market_owner_addr: address,
    ) acquires Market {
        let listing_fee = borrow_global<Market>(market_owner_addr).listing_fee;
        let buyer_addr = Signer::address_of(buyer);
        assert!(buyer_addr != seller, ERROR_INVALID_BUYER);
		let market_items = &mut borrow_global_mut<Market>(market_owner_addr).market_items;
        let market_item = Table::borrow(market_items, token_id);
        let price = market_item.price;

        TestCoin::transfer(buyer, seller, price);
        TestCoin::transfer(buyer, market_owner_addr, listing_fee);

        let market_item = Table::remove(market_items, token_id);
        let MarketItem{ seller: _, token: token, price: _,} = market_item;
        Token::deposit_token(buyer, token)
	}

    fun create_token(creator: &signer, amount: u64): ID {
        use Std::ASCII;
        use Std::Option;

        let collection_name = ASCII::string(b"Hello, World");
        Token::create_collection(
            creator,
            ASCII::string(b"Collection: Hello, World"),
            *&collection_name,
            ASCII::string(b"https://aptos.dev"),
            Option::none(),
        );
        Token::create_token(
            creator,
            collection_name,
            ASCII::string(b"Token: Hello, Token"),
            ASCII::string(b"Hello, Token"),
            amount,
            ASCII::string(b"https://aptos.dev"),
        )
    }

    #[test(market_owner = @0x1, seller = @0x2, buyer = @CoreResources)]
    public(script) fun list_buy_test(market_owner: signer, seller: signer, buyer: signer) acquires Market {
        let market_owner_addr = Signer::address_of(&market_owner);
        let seller_addr = Signer::address_of(&seller);
        let buyer_addr = Signer::address_of(&buyer);

        init_market_script(&market_owner, 1);

        let token_id = create_token(&seller, 1);

        TestCoin::initialize(&buyer, 1000000);
        TestCoin::register(&market_owner);
		TestCoin::register(&seller);
        let amount = 1000;
        TestCoin::mint_internal(&buyer, buyer_addr, amount);

        list_token_for_sale(&seller, &token_id, 10, market_owner_addr);
        buy_token(&buyer, seller_addr, &token_id, market_owner_addr);
    }
}