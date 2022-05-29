// This module provides a basic NFT Marketplace with first-price auction and fixed-price sale
module AptosFramework::Marketplace {
    use AptosFramework::Coin::{Self, Coin};
    use AptosFramework::Table::{Self, Table};
    use AptosFramework::TestCoin::TestCoin;
    use AptosFramework::Timestamp;
    use AptosFramework::Token::{Self, Token, TokenId};
    use Std::Signer;
    use Std::Event::{Self, EventHandle};

    const ERROR_INVALID_BUYER: u64 = 0;
    const ERROR_INSUFFICIENT_BID: u64 = 1;
    const ERROR_AUCTION_INACTIVE: u64 = 2;
    const ERROR_AUCTION_NOT_COMPLETE: u64 = 3;
    const ERROR_NOT_CLAIMABLE: u64 = 4;
    const ERROR_CLAIM_COINS_FIRST: u64 = 5;

    struct AuctionItem has store, drop {
        min_bid: u64,
        duration: u64,
        start_time: u64,
        curr_bid: u64,
        curr_bidder: address,
    }

    struct ListedItem has store, drop {
        price: u64
    }

    struct CoinEscrow has key {
        locked_coins: Table<TokenId, Coin<TestCoin>>,
    }

    struct TokenEscrow has key {
        locked_tokens: Table<TokenId, Token>,
    }

    // Set of data sent to the event stream during a auctioning a token
    struct AuctionEvent has drop, store {
        id: TokenId,
        min_bid: u64,
        duration: u64
    }

    // Set of data sent to the event stream during a bidding for a token
    struct BidEvent has drop, store {
        id: TokenId,
        bid: u64,
    }

    // Set of data sent to the event stream during a bidding for a token
    struct ClaimCoinsEvent has drop, store {
        id: TokenId,
    }

    // Set of data sent to the event stream during a bidding for a token
    struct ClaimTokenEvent has drop, store {
        id: TokenId,
    }

    // Set of data sent to the event stream during a listing of a token (for fixed price)
    struct ListEvent has drop, store {
        id: TokenId,
        amount: u64,
    }

    // Set of data sent to the event stream during a buying of a token (for fixed price)
    struct BuyEvent has drop, store {
        id: TokenId,
    }

    struct AuctionData has key {
        auction_items: Table<TokenId, AuctionItem>,
        auction_events: EventHandle<AuctionEvent>,
        bid_events: EventHandle<BidEvent>,
        claim_coins_events: EventHandle<ClaimCoinsEvent>,
        claim_token_events: EventHandle<ClaimTokenEvent>,
    }

    struct ListedItemsData has key {
        listed_items: Table<TokenId, ListedItem>,
        listing_events: EventHandle<ListEvent>,
        buying_events: EventHandle<BuyEvent>,
    }

    public(script)fun initialize_auction(account: &signer, creator: address, collection_name: vector<u8>, name: vector<u8>, min_bid: u64, duration: u64) acquires TokenEscrow, AuctionData {
        let token_id = Token::create_token_id_raw(creator, collection_name, name);
        let account_addr = Signer::address_of(account);
        if (!exists<TokenEscrow>(account_addr)) {
            move_to(account, TokenEscrow {
                locked_tokens: Table::new<TokenId, Token>()
            });
        };
        if (!exists<AuctionData>(account_addr)) {
            move_to(account, AuctionData {
                auction_items: Table::new<TokenId, AuctionItem>(),
                auction_events: Event::new_event_handle<AuctionEvent>(account),
                bid_events: Event::new_event_handle<BidEvent>(account),
                claim_coins_events: Event::new_event_handle<ClaimCoinsEvent>(account),
                claim_token_events: Event::new_event_handle<ClaimTokenEvent>(account),
            });
        };
        let start_time = Timestamp::now_microseconds();
        let token = Token::withdraw_token(account, &token_id, 1);
        let locked_tokens =
            &mut borrow_global_mut<TokenEscrow>(account_addr).locked_tokens;

        let auction_data = borrow_global_mut<AuctionData>(account_addr);
        let auction_items = &mut auction_data.auction_items;
        
        // if auction_items still contain token_id, this means that when account_addr last auctioned this token, 
        // they did not claim the coins from the highest bidder
        // account_addr has received the same token somehow but has not claimed the coins from the initial auction
        assert!(!Table::contains(auction_items, &token_id), ERROR_CLAIM_COINS_FIRST);

        Event::emit_event<AuctionEvent>(
            &mut auction_data.auction_events,
            AuctionEvent { id: token_id, min_bid: min_bid, duration: duration },
        );

        Table::add(locked_tokens, &token_id, token);
        Table::add(auction_items, &token_id, AuctionItem {
            min_bid,
            duration,
            start_time,
            curr_bid: min_bid - 1,
            curr_bidder: account_addr
        })
    }

    fun is_auction_active(start_time: u64, duration: u64): bool {
        let curr_time = Timestamp::now_microseconds();
        curr_time <= start_time + duration && curr_time >= start_time
    }

    fun is_auction_complete(start_time: u64, duration: u64): bool {
        let curr_time = Timestamp::now_microseconds();
        curr_time > start_time + duration
    }

    public(script) fun bid(account: &signer, seller: address, creator: address, collection_name: vector<u8>, name: vector<u8>, bid: u64) acquires CoinEscrow, AuctionData {
        let token_id = Token::create_token_id_raw(creator, collection_name, name);
        let account_addr = Signer::address_of(account);
        assert!(account_addr != seller, ERROR_INVALID_BUYER);

        let auction_data = borrow_global_mut<AuctionData>(seller);
        let auction_items = &mut auction_data.auction_items;
        let auction_item = Table::borrow_mut(auction_items, &token_id);
        assert!(is_auction_active(auction_item.start_time, auction_item.duration), ERROR_AUCTION_INACTIVE);

        assert!(bid > auction_item.curr_bid, ERROR_INSUFFICIENT_BID);

        if (!exists<CoinEscrow>(account_addr)) {
            move_to(account, CoinEscrow {
                locked_coins: Table::new<TokenId, Coin<TestCoin>>()
            });
        };

        if (auction_item.curr_bidder != seller) {
            let curr_bidder_locked_coins = &mut borrow_global_mut<CoinEscrow>(auction_item.curr_bidder).locked_coins;
            let coins = Table::remove(curr_bidder_locked_coins, &token_id);
            Coin::deposit<TestCoin>(auction_item.curr_bidder, coins);
        };

        Event::emit_event<BidEvent>(
            &mut auction_data.bid_events,
            BidEvent { id: token_id, bid: bid },
        );

        let locked_coins = &mut borrow_global_mut<CoinEscrow>(account_addr).locked_coins;
        let coins = Coin::withdraw<TestCoin>(account, bid);
        Table::add(locked_coins, &token_id, coins);
        auction_item.curr_bidder = account_addr;
        auction_item.curr_bid = bid;
    }

    public(script) fun claim_token(account: &signer, seller: address, creator: address, collection_name: vector<u8>, name: vector<u8>) acquires CoinEscrow, TokenEscrow, AuctionData {
        let token_id = Token::create_token_id_raw(creator, collection_name, name);
        let account_addr = Signer::address_of(account);

        let auction_data = borrow_global_mut<AuctionData>(seller);
        let auction_items = &mut auction_data.auction_items;
        let auction_item = Table::borrow(auction_items, &token_id);
        assert!(is_auction_complete(auction_item.start_time, auction_item.duration), ERROR_AUCTION_NOT_COMPLETE);

        assert!(account_addr == auction_item.curr_bidder, ERROR_NOT_CLAIMABLE);

        Event::emit_event<ClaimTokenEvent>(
            &mut auction_data.claim_token_events,
            ClaimTokenEvent { id: token_id },
        );

        let locked_tokens = &mut borrow_global_mut<TokenEscrow>(seller).locked_tokens;
        let token = Table::remove(locked_tokens, &token_id);
        Token::deposit_token(account, token);

        // if the locked coins from account_addr doesn't contain the key token_id, the seller has already claimed those coins
        // and the auction item can be removed from the auctiondata of the seller
        let locked_coins = &borrow_global<CoinEscrow>(account_addr).locked_coins;
        if (!Table::contains(locked_coins, &token_id)){
            let AuctionItem{min_bid: _, duration: _, start_time: _, curr_bid: _, curr_bidder: _,} = Table::remove(auction_items, &token_id);
        };
    }

    public(script) fun claim_coins(account: &signer, creator: address, collection_name: vector<u8>, name: vector<u8>) acquires CoinEscrow, TokenEscrow, AuctionData {
        let token_id = Token::create_token_id_raw(creator, collection_name, name);
        let account_addr = Signer::address_of(account);

        let auction_data = borrow_global_mut<AuctionData>(account_addr);
        let auction_items = &mut auction_data.auction_items;
        let auction_item = Table::borrow(auction_items, &token_id);
        assert!(is_auction_complete(auction_item.start_time, auction_item.duration), ERROR_AUCTION_NOT_COMPLETE);
        assert!(account_addr!=auction_item.curr_bidder, ERROR_NOT_CLAIMABLE);

        Event::emit_event<ClaimCoinsEvent>(
            &mut auction_data.claim_coins_events,
            ClaimCoinsEvent { id: token_id },
        );

        let locked_coins = &mut borrow_global_mut<CoinEscrow>(auction_item.curr_bidder).locked_coins;
        let coins = Table::remove(locked_coins, &token_id);
        Coin::deposit<TestCoin>(account_addr, coins);

        // if the locked token from account_addr doesn't contain the key token_id, the highest bidder has already claimed those coins
        // and the auction item can be removed from the auctiondata of the seller
        let locked_tokens = &borrow_global<TokenEscrow>(account_addr).locked_tokens;
        if (!Table::contains(locked_tokens, &token_id)){
            let AuctionItem{min_bid: _, duration: _, start_time: _, curr_bid: _, curr_bidder: _,} = Table::remove(auction_items, &token_id);
        };
    }

    // part of the fixed price sale flow
    public(script) fun list_token(account: &signer, creator: address, collection_name: vector<u8>, name: vector<u8>, price: u64) acquires TokenEscrow, ListedItemsData{
        let token_id = Token::create_token_id_raw(creator, collection_name, name);
        let account_addr = Signer::address_of(account);
        if (!exists<TokenEscrow>(account_addr)) {
            move_to(account, TokenEscrow {
                locked_tokens: Table::new<TokenId, Token>()
            });
        };
        if (!exists<ListedItemsData>(account_addr)) {
            move_to(account, ListedItemsData {
                listed_items: Table::new<TokenId, ListedItem>(),
                listing_events: Event::new_event_handle<ListEvent>(account),
                buying_events: Event::new_event_handle<BuyEvent>(account),
            });
        };
        let token = Token::withdraw_token(account, &token_id, 1);
        let locked_tokens =
            &mut borrow_global_mut<TokenEscrow>(account_addr).locked_tokens;

        let listedItemsData = borrow_global_mut<ListedItemsData>(account_addr);
        let listed_items = &mut listedItemsData.listed_items;

        Event::emit_event<ListEvent>(
            &mut listedItemsData.listing_events,
            ListEvent { id: token_id, amount: price },
        );

        Table::add(locked_tokens, &token_id, token);
        Table::add(listed_items, &token_id, ListedItem {
            price
        })
    }

    // part of the fixed price sale flow
    public(script) fun buy_token(account: &signer, seller: address, creator: address, collection_name: vector<u8>, name: vector<u8>) acquires TokenEscrow, ListedItemsData {
        let token_id = Token::create_token_id_raw(creator, collection_name, name);
        let account_addr = Signer::address_of(account);
        assert!(account_addr != seller, ERROR_INVALID_BUYER);

        let listedItemsData = borrow_global_mut<ListedItemsData>(seller);
        Event::emit_event<BuyEvent>(
            &mut listedItemsData.buying_events,
            BuyEvent { id: token_id },
        );

        let listed_items = &mut listedItemsData.listed_items;
        let listed_item = Table::borrow_mut(listed_items, &token_id);

        Coin::transfer<TestCoin>(account, seller, listed_item.price);

        let locked_tokens = &mut borrow_global_mut<TokenEscrow>(seller).locked_tokens;
        let token = Table::remove(locked_tokens, &token_id);
        Token::deposit_token(account, token);

        let ListedItem{price: _,} = Table::remove(listed_items, &token_id);
    }
}