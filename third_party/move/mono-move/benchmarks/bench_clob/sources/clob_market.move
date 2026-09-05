/// Central limit order book over two bit-packed AVL queues, one per side.
///
/// Bids are a descending queue keyed by price and asks an ascending one, so
/// the head of either queue is always the order with the best price and, among
/// equal prices, the oldest. Collateral is real: a market account holds one
/// fungible store per asset, placing an order locks base or quote, and a fill
/// moves the assets between stores.
///
/// # Market order ids
///
/// A market order id is a `u128` whose low 64 bits are the AVL access key and
/// whose high 64 bits are a per-market counter. The access key alone locates
/// the order in constant time, and the counter rules out acting on a stale id
/// whose node has since been reused. The access key also carries the queue's
/// sort order, which is what tells `cancel_order` which side to look on.
///
/// # Entry points that produce an id
///
/// An entry function cannot return, so `place_limit_order` and
/// `change_order_size` each have a plain `..._id` counterpart that does the
/// same work and returns the market order id.
module bench::clob_market {
    use std::bcs;
    use std::signer;
    use std::vector;
    use aptos_std::table::{Self, Table};
    use aptos_framework::fungible_asset::{Self, FungibleStore, Metadata};
    use aptos_framework::object::{Self, Object};
    use aptos_framework::primary_fungible_store;
    use bench::clob_avl_queue::{Self, AvlQueue};
    use bench::clob_mock_fa;

    /// Only the package address can hold the registry.
    const E_NOT_BENCH: u64 = 1;
    /// No market with the given id.
    const E_NO_MARKET: u64 = 2;
    /// User has no account on this market.
    const E_NO_MARKET_ACCOUNT: u64 = 3;
    /// User already has an account on this market.
    const E_ACCOUNT_EXISTS: u64 = 4;
    /// Asset is neither the base nor the quote of this market.
    const E_UNKNOWN_ASSET: u64 = 5;
    /// Price is zero or wider than the 32-bit insertion key.
    const E_INVALID_PRICE: u64 = 6;
    /// Size is zero.
    const E_INVALID_SIZE: u64 = 7;
    /// Available balance does not cover the order.
    const E_INSUFFICIENT_FUNDS: u64 = 8;
    /// Market order id names an order belonging to somebody else.
    const E_NOT_ORDER_OWNER: u64 = 9;
    /// Market order id names a node that has since been reused.
    const E_STALE_ORDER_ID: u64 = 10;
    /// Lot size or tick size is zero.
    const E_INVALID_MARKET: u64 = 11;
    /// Jittered seed prices would run below zero.
    const E_SEED_PRICE_TOO_LOW: u64 = 12;
    /// `run` was given an `expected` that `index_orders` did not produce.
    const E_BAD_RESULT: u64 = 13;

    const BID: bool = true;
    const ASK: bool = false;

    /// Widest price the AVL queue's 32-bit insertion key can hold.
    const HI_PRICE: u64 = 0xffffffff;
    /// Low half of a market order id.
    const HI_ACCESS_KEY: u128 = 0xffffffffffffffff;

    const LCG_MUL: u64 = 1103515245;
    const LCG_INC: u64 = 12345;
    const LCG_MOD: u64 = 1000003;
    const CHECKSUM_MOD: u64 = 1000000007;

    /// Ticks the jitter of a seeded price spans per order. Four keeps most
    /// price levels distinct while still colliding often enough to grow the
    /// per-level lists.
    const SEED_TICKS_PER_ORDER: u64 = 4;
    /// Largest seeded order, in lots.
    const SEED_MAX_SIZE: u64 = 8;

    struct Order has store, drop {
        /// Remaining size, in lots.
        size: u64,
        user: address,
        /// High half of the market order id this order was issued under.
        order_id: u64,
    }

    struct Market has store {
        base: Object<Metadata>,
        quote: Object<Metadata>,
        lot_size: u64,
        tick_size: u64,
        bids: AvlQueue<Order>,
        asks: AvlQueue<Order>,
        /// Orders ever placed on this market.
        n_orders: u64,
    }

    struct Registry has key {
        markets: Table<u64, Market>,
        n_markets: u64,
    }

    struct Account has store {
        base_store: Object<FungibleStore>,
        quote_store: Object<FungibleStore>,
        base_available: u64,
        base_locked: u64,
        quote_available: u64,
        quote_locked: u64,
    }

    struct MarketAccounts has key {
        accounts: Table<u64, Account>,
    }

    // Registration.

    /// Register a market and return its id. Ids count from one.
    public fun register_market_id(
        admin: &signer,
        base: Object<Metadata>,
        quote: Object<Metadata>,
        lot_size: u64,
        tick_size: u64,
    ): u64 acquires Registry {
        assert!(lot_size > 0 && tick_size > 0, E_INVALID_MARKET);
        if (!exists<Registry>(@bench)) {
            assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
            move_to(admin, Registry { markets: table::new(), n_markets: 0 });
        };
        let registry = borrow_global_mut<Registry>(@bench);
        registry.n_markets = registry.n_markets + 1;
        let market_id = registry.n_markets;
        table::add(
            &mut registry.markets,
            market_id,
            Market {
                base,
                quote,
                lot_size,
                tick_size,
                // Best bid is the highest price, best ask the lowest.
                bids: clob_avl_queue::new<Order>(false),
                asks: clob_avl_queue::new<Order>(true),
                n_orders: 0,
            },
        );
        market_id
    }

    public entry fun register_market(
        admin: &signer,
        base: Object<Metadata>,
        quote: Object<Metadata>,
        lot_size: u64,
        tick_size: u64,
    ) acquires Registry {
        register_market_id(admin, base, quote, lot_size, tick_size);
    }

    public entry fun register_market_account(
        user: &signer, market_id: u64
    ) acquires Registry, MarketAccounts {
        let user_addr = signer::address_of(user);
        let (base, quote) = market_assets(market_id);
        if (!exists<MarketAccounts>(user_addr)) {
            move_to(user, MarketAccounts { accounts: table::new() });
        };
        // Checked before the stores are created, since creating a store at an
        // address that already holds one aborts inside the object module.
        assert!(
            !table::contains(
                &borrow_global<MarketAccounts>(user_addr).accounts, market_id),
            E_ACCOUNT_EXISTS,
        );
        let base_store = new_store(user, market_id, true, base);
        let quote_store = new_store(user, market_id, false, quote);
        let accounts =
            &mut borrow_global_mut<MarketAccounts>(user_addr).accounts;
        table::add(
            accounts,
            market_id,
            Account {
                base_store,
                quote_store,
                base_available: 0,
                base_locked: 0,
                quote_available: 0,
                quote_locked: 0,
            },
        );
    }

    /// Collateral store for one asset of one market, at a named object so its
    /// address is derived from the owner rather than stored anywhere.
    fun new_store(
        user: &signer, market_id: u64, is_base: bool, asset: Object<Metadata>
    ): Object<FungibleStore> {
        let seed = b"bench_clob";
        vector::append(&mut seed, bcs::to_bytes(&market_id));
        vector::push_back(&mut seed, if (is_base) 0 else 1);
        fungible_asset::create_store(
            &object::create_named_object(user, seed), asset)
    }

    /// Move `amount` of `asset` from the user's primary store into their
    /// market account.
    public entry fun deposit(
        user: &signer, market_id: u64, asset: Object<Metadata>, amount: u64
    ) acquires Registry, MarketAccounts {
        let user_addr = signer::address_of(user);
        let (base, quote) = market_assets(market_id);
        let asset_addr = object::object_address(&asset);
        let is_base = asset_addr == object::object_address(&base);
        assert!(
            is_base || asset_addr == object::object_address(&quote),
            E_UNKNOWN_ASSET,
        );
        let fa = primary_fungible_store::withdraw(user, asset, amount);
        assert!(exists<MarketAccounts>(user_addr), E_NO_MARKET_ACCOUNT);
        let account = table::borrow_mut(
            &mut borrow_global_mut<MarketAccounts>(user_addr).accounts,
            market_id,
        );
        if (is_base) {
            clob_mock_fa::deposit(asset, account.base_store, fa);
            account.base_available = account.base_available + amount;
        } else {
            clob_mock_fa::deposit(asset, account.quote_store, fa);
            account.quote_available = account.quote_available + amount;
        }
    }

    // Orders.

    /// Match against the opposite side up to `price`, then rest whatever is
    /// left. Returns the market order id of the resting remainder, or zero
    /// when the order filled completely.
    public fun place_limit_order_id(
        user: &signer, market_id: u64, side: bool, price: u64, size: u64
    ): u128 acquires Registry, MarketAccounts {
        assert!(price > 0 && price <= HI_PRICE, E_INVALID_PRICE);
        assert!(size > 0, E_INVALID_SIZE);
        let user_addr = signer::address_of(user);
        let remaining = match_orders(market_id, user_addr, side, price, size);
        if (remaining == 0) return 0;
        post_order(market_id, user_addr, side, price, remaining)
    }

    public entry fun place_limit_order(
        user: &signer, market_id: u64, side: bool, price: u64, size: u64
    ) acquires Registry, MarketAccounts {
        place_limit_order_id(user, market_id, side, price, size);
    }

    /// Take `size` lots off the opposite side at any price. Stops early when
    /// the book runs out rather than aborting.
    public entry fun place_market_order(
        user: &signer, market_id: u64, side: bool, size: u64
    ) acquires Registry, MarketAccounts {
        assert!(size > 0, E_INVALID_SIZE);
        let limit_price = if (side == BID) HI_PRICE else 0;
        match_orders(
            market_id, signer::address_of(user), side, limit_price, size);
    }

    public entry fun cancel_order(
        user: &signer, market_id: u64, market_order_id: u128
    ) acquires Registry, MarketAccounts {
        let user_addr = signer::address_of(user);
        let access_key = ((market_order_id & HI_ACCESS_KEY) as u64);
        let order_id = ((market_order_id >> 64) as u64);
        let side = order_side(access_key);
        let price = clob_avl_queue::access_key_insertion_key(access_key);
        let registry = borrow_global_mut<Registry>(@bench);
        assert!(table::contains(&registry.markets, market_id), E_NO_MARKET);
        let market = table::borrow_mut(&mut registry.markets, market_id);
        let lot_size = market.lot_size;
        let tick_size = market.tick_size;
        let queue =
            if (side == BID) &mut market.bids else &mut market.asks;
        let order = clob_avl_queue::borrow(queue, access_key);
        assert!(order.user == user_addr, E_NOT_ORDER_OWNER);
        assert!(order.order_id == order_id, E_STALE_ORDER_ID);
        let size = order.size;
        clob_avl_queue::remove(queue, access_key);
        unlock(
            user_addr,
            market_id,
            side,
            size * lot_size,
            size * price * tick_size,
        );
    }

    /// Resize a resting order. Shrinking keeps the order where it is;
    /// growing takes it off the book and puts it back, so it loses time
    /// priority and gets a new market order id, which is returned.
    public fun change_order_size_id(
        user: &signer, market_id: u64, market_order_id: u128, new_size: u64
    ): u128 acquires Registry, MarketAccounts {
        assert!(new_size > 0, E_INVALID_SIZE);
        let user_addr = signer::address_of(user);
        let access_key = ((market_order_id & HI_ACCESS_KEY) as u64);
        let order_id = ((market_order_id >> 64) as u64);
        let side = order_side(access_key);
        let price = clob_avl_queue::access_key_insertion_key(access_key);
        let registry = borrow_global_mut<Registry>(@bench);
        assert!(table::contains(&registry.markets, market_id), E_NO_MARKET);
        let market = table::borrow_mut(&mut registry.markets, market_id);
        let lot_size = market.lot_size;
        let tick_size = market.tick_size;
        let queue =
            if (side == BID) &mut market.bids else &mut market.asks;
        let order = clob_avl_queue::borrow(queue, access_key);
        assert!(order.user == user_addr, E_NOT_ORDER_OWNER);
        assert!(order.order_id == order_id, E_STALE_ORDER_ID);
        let old_size = order.size;
        if (new_size == old_size) return market_order_id;
        if (new_size < old_size) {
            let resized = clob_avl_queue::borrow_mut(queue, access_key);
            resized.size = new_size;
            let freed = old_size - new_size;
            unlock(
                user_addr,
                market_id,
                side,
                freed * lot_size,
                freed * price * tick_size,
            );
            return market_order_id
        };
        clob_avl_queue::remove(queue, access_key);
        unlock(
            user_addr,
            market_id,
            side,
            old_size * lot_size,
            old_size * price * tick_size,
        );
        post_order(market_id, user_addr, side, price, new_size)
    }

    public entry fun change_order_size(
        user: &signer, market_id: u64, market_order_id: u128, new_size: u64
    ) acquires Registry, MarketAccounts {
        change_order_size_id(user, market_id, market_order_id, new_size);
    }

    /// Fill an LCG-jittered book of `n_bids` bids below and `n_asks` asks
    /// above `base_price`, all owned by `admin` and collateralised from
    /// `admin`'s market account. Evenly spaced prices would produce an
    /// unnaturally balanced tree, so prices are jittered.
    public entry fun seed_book(
        admin: &signer,
        market_id: u64,
        n_bids: u64,
        n_asks: u64,
        base_price: u64,
        spread: u64,
        seed: u64,
    ) acquires Registry, MarketAccounts {
        let user = signer::address_of(admin);
        let bid_range = n_bids * SEED_TICKS_PER_ORDER + 1;
        let ask_range = n_asks * SEED_TICKS_PER_ORDER + 1;
        assert!(base_price > spread + bid_range, E_SEED_PRICE_TOO_LOW);
        assert!(base_price + spread + ask_range <= HI_PRICE, E_INVALID_PRICE);
        let x = seed % LCG_MOD;
        let i = 0;
        while (i < n_bids) {
            x = ((x * LCG_MUL) + LCG_INC) % LCG_MOD;
            let price = base_price - spread - (x % bid_range);
            x = ((x * LCG_MUL) + LCG_INC) % LCG_MOD;
            let size = 1 + (x % SEED_MAX_SIZE);
            post_order(market_id, user, BID, price, size);
            i = i + 1;
        };
        let i = 0;
        while (i < n_asks) {
            x = ((x * LCG_MUL) + LCG_INC) % LCG_MOD;
            let price = base_price + spread + (x % ask_range);
            x = ((x * LCG_MUL) + LCG_INC) % LCG_MOD;
            let size = 1 + (x % SEED_MAX_SIZE);
            post_order(market_id, user, ASK, price, size);
            i = i + 1;
        }
    }

    /// Walk one side in price order, folding price and size into a checksum,
    /// and stop after `limit` orders. Raising `limit` is what makes a single
    /// transaction arbitrarily expensive.
    public fun index_orders(
        market_id: u64, side: bool, limit: u64
    ): u64 acquires Registry {
        let registry = borrow_global<Registry>(@bench);
        assert!(table::contains(&registry.markets, market_id), E_NO_MARKET);
        let market = table::borrow(&registry.markets, market_id);
        let queue = if (side == BID) &market.bids else &market.asks;
        let access_key = clob_avl_queue::head_access_key(queue);
        let acc = 0;
        let n = 0;
        while (access_key != 0 && n < limit) {
            let price =
                clob_avl_queue::access_key_insertion_key(access_key);
            let order = clob_avl_queue::borrow(queue, access_key);
            acc = (acc * LCG_MOD + price + order.size) % CHECKSUM_MOD;
            access_key = clob_avl_queue::next_access_key(queue, access_key);
            n = n + 1;
        };
        acc
    }

    public entry fun run(
        _s: &signer, market_id: u64, side: bool, limit: u64, expected: u64
    ) acquires Registry {
        assert!(index_orders(market_id, side, limit) == expected, E_BAD_RESULT);
    }

    // Matching.

    /// Take up to `size` lots off the opposite side, stopping at
    /// `limit_price` or when the book runs out. Returns the unfilled
    /// remainder.
    fun match_orders(
        market_id: u64,
        taker: address,
        taker_side: bool,
        limit_price: u64,
        size: u64,
    ): u64 acquires Registry, MarketAccounts {
        let remaining = size;
        while (remaining > 0) {
            let registry = borrow_global_mut<Registry>(@bench);
            assert!(table::contains(&registry.markets, market_id), E_NO_MARKET);
            let market = table::borrow_mut(&mut registry.markets, market_id);
            let base = market.base;
            let quote = market.quote;
            let lot_size = market.lot_size;
            let tick_size = market.tick_size;
            let queue =
                if (taker_side == BID) &mut market.asks else &mut market.bids;
            if (clob_avl_queue::is_empty(queue)) break;
            let price = clob_avl_queue::get_head_key(queue);
            let crosses =
                if (taker_side == BID) price <= limit_price
                else price >= limit_price;
            if (!crosses) break;
            let head = clob_avl_queue::borrow_head(queue);
            let maker = head.user;
            let head_size = head.size;
            let fill = if (head_size <= remaining) head_size else remaining;
            if (fill == head_size) {
                clob_avl_queue::pop_head(queue);
            } else {
                let resting = clob_avl_queue::borrow_head_mut(queue);
                resting.size = head_size - fill;
            };
            let base_amount = fill * lot_size;
            let quote_amount = fill * price * tick_size;
            remaining = remaining - fill;
            fill_taker(
                taker, market_id, taker_side, base_amount, quote_amount);
            fill_maker(
                maker, market_id, taker_side, base_amount, quote_amount);
            let seller = if (taker_side == ASK) taker else maker;
            let buyer = if (taker_side == ASK) maker else taker;
            settle(
                base,
                quote,
                market_id,
                seller,
                buyer,
                base_amount,
                quote_amount,
            );
        };
        remaining
    }

    /// Rest `size` lots at `price`, locking the collateral first. Returns the
    /// market order id.
    fun post_order(
        market_id: u64, user: address, side: bool, price: u64, size: u64
    ): u128 acquires Registry, MarketAccounts {
        let (lot_size, tick_size) = market_lot_tick(market_id);
        lock(
            user,
            market_id,
            side,
            size * lot_size,
            size * price * tick_size,
        );
        let registry = borrow_global_mut<Registry>(@bench);
        let market = table::borrow_mut(&mut registry.markets, market_id);
        market.n_orders = market.n_orders + 1;
        let order_id = market.n_orders;
        let order = Order { size, user, order_id };
        let access_key = if (side == BID) {
            clob_avl_queue::insert(&mut market.bids, price, order)
        } else {
            clob_avl_queue::insert(&mut market.asks, price, order)
        };
        ((order_id as u128) << 64) | (access_key as u128)
    }

    /// The access key carries the queue's sort order, and asks are the
    /// ascending queue.
    fun order_side(access_key: u64): bool {
        if (clob_avl_queue::access_key_is_ascending(access_key)) ASK else BID
    }

    /// Debit what the taker pays out of its available balance and credit what
    /// it receives.
    fun fill_taker(
        taker: address,
        market_id: u64,
        taker_side: bool,
        base_amount: u64,
        quote_amount: u64,
    ) acquires MarketAccounts {
        assert!(exists<MarketAccounts>(taker), E_NO_MARKET_ACCOUNT);
        let accounts = &mut borrow_global_mut<MarketAccounts>(taker).accounts;
        assert!(table::contains(accounts, market_id), E_NO_MARKET_ACCOUNT);
        let account = table::borrow_mut(accounts, market_id);
        if (taker_side == BID) {
            assert!(
                account.quote_available >= quote_amount, E_INSUFFICIENT_FUNDS);
            account.quote_available = account.quote_available - quote_amount;
            account.base_available = account.base_available + base_amount;
        } else {
            assert!(
                account.base_available >= base_amount, E_INSUFFICIENT_FUNDS);
            account.base_available = account.base_available - base_amount;
            account.quote_available = account.quote_available + quote_amount;
        }
    }

    /// Consume the maker's locked collateral and credit the proceeds.
    fun fill_maker(
        maker: address,
        market_id: u64,
        taker_side: bool,
        base_amount: u64,
        quote_amount: u64,
    ) acquires MarketAccounts {
        assert!(exists<MarketAccounts>(maker), E_NO_MARKET_ACCOUNT);
        let accounts = &mut borrow_global_mut<MarketAccounts>(maker).accounts;
        assert!(table::contains(accounts, market_id), E_NO_MARKET_ACCOUNT);
        let account = table::borrow_mut(accounts, market_id);
        if (taker_side == BID) {
            account.base_locked = account.base_locked - base_amount;
            account.quote_available = account.quote_available + quote_amount;
        } else {
            account.quote_locked = account.quote_locked - quote_amount;
            account.base_available = account.base_available + base_amount;
        }
    }

    /// Move base from the seller to the buyer and quote the other way.
    fun settle(
        base: Object<Metadata>,
        quote: Object<Metadata>,
        market_id: u64,
        seller: address,
        buyer: address,
        base_amount: u64,
        quote_amount: u64,
    ) acquires MarketAccounts {
        let (seller_base, seller_quote) = account_stores(seller, market_id);
        let (buyer_base, buyer_quote) = account_stores(buyer, market_id);
        clob_mock_fa::deposit(
            base,
            buyer_base,
            clob_mock_fa::withdraw(base, seller_base, base_amount),
        );
        clob_mock_fa::deposit(
            quote,
            seller_quote,
            clob_mock_fa::withdraw(quote, buyer_quote, quote_amount),
        );
    }

    fun lock(
        user: address,
        market_id: u64,
        side: bool,
        base_amount: u64,
        quote_amount: u64,
    ) acquires MarketAccounts {
        assert!(exists<MarketAccounts>(user), E_NO_MARKET_ACCOUNT);
        let accounts = &mut borrow_global_mut<MarketAccounts>(user).accounts;
        assert!(table::contains(accounts, market_id), E_NO_MARKET_ACCOUNT);
        let account = table::borrow_mut(accounts, market_id);
        if (side == BID) {
            assert!(
                account.quote_available >= quote_amount, E_INSUFFICIENT_FUNDS);
            account.quote_available = account.quote_available - quote_amount;
            account.quote_locked = account.quote_locked + quote_amount;
        } else {
            assert!(
                account.base_available >= base_amount, E_INSUFFICIENT_FUNDS);
            account.base_available = account.base_available - base_amount;
            account.base_locked = account.base_locked + base_amount;
        }
    }

    fun unlock(
        user: address,
        market_id: u64,
        side: bool,
        base_amount: u64,
        quote_amount: u64,
    ) acquires MarketAccounts {
        assert!(exists<MarketAccounts>(user), E_NO_MARKET_ACCOUNT);
        let accounts = &mut borrow_global_mut<MarketAccounts>(user).accounts;
        assert!(table::contains(accounts, market_id), E_NO_MARKET_ACCOUNT);
        let account = table::borrow_mut(accounts, market_id);
        if (side == BID) {
            account.quote_locked = account.quote_locked - quote_amount;
            account.quote_available = account.quote_available + quote_amount;
        } else {
            account.base_locked = account.base_locked - base_amount;
            account.base_available = account.base_available + base_amount;
        }
    }

    // Accessors.

    fun account_stores(
        user: address, market_id: u64
    ): (Object<FungibleStore>, Object<FungibleStore>) acquires MarketAccounts {
        assert!(exists<MarketAccounts>(user), E_NO_MARKET_ACCOUNT);
        let account = table::borrow(
            &borrow_global<MarketAccounts>(user).accounts, market_id);
        (account.base_store, account.quote_store)
    }

    fun market_assets(
        market_id: u64
    ): (Object<Metadata>, Object<Metadata>) acquires Registry {
        assert!(exists<Registry>(@bench), E_NO_MARKET);
        let registry = borrow_global<Registry>(@bench);
        assert!(table::contains(&registry.markets, market_id), E_NO_MARKET);
        let market = table::borrow(&registry.markets, market_id);
        (market.base, market.quote)
    }

    fun market_lot_tick(market_id: u64): (u64, u64) acquires Registry {
        assert!(exists<Registry>(@bench), E_NO_MARKET);
        let registry = borrow_global<Registry>(@bench);
        assert!(table::contains(&registry.markets, market_id), E_NO_MARKET);
        let market = table::borrow(&registry.markets, market_id);
        (market.lot_size, market.tick_size)
    }

    #[view]
    /// Best resting price on `side`, or zero when that side is empty.
    public fun best_price(market_id: u64, side: bool): u64 acquires Registry {
        let registry = borrow_global<Registry>(@bench);
        let market = table::borrow(&registry.markets, market_id);
        let queue = if (side == BID) &market.bids else &market.asks;
        if (clob_avl_queue::is_empty(queue)) 0
        else clob_avl_queue::get_head_key(queue)
    }

    #[view]
    /// Height of the price tree on `side`, which is how many dependent table
    /// reads a lookup at the deepest price level costs.
    public fun book_height(market_id: u64, side: bool): u64 acquires Registry {
        let registry = borrow_global<Registry>(@bench);
        let market = table::borrow(&registry.markets, market_id);
        let queue = if (side == BID) &market.bids else &market.asks;
        clob_avl_queue::get_height(queue)
    }

    #[view]
    /// Number of resting orders on `side`, capped at `limit`.
    public fun n_orders(
        market_id: u64, side: bool, limit: u64
    ): u64 acquires Registry {
        let registry = borrow_global<Registry>(@bench);
        let market = table::borrow(&registry.markets, market_id);
        let queue = if (side == BID) &market.bids else &market.asks;
        let access_key = clob_avl_queue::head_access_key(queue);
        let n = 0;
        while (access_key != 0 && n < limit) {
            access_key = clob_avl_queue::next_access_key(queue, access_key);
            n = n + 1;
        };
        n
    }

    #[view]
    /// Total resting size on `side`, capped at `limit` orders.
    public fun resting_size(
        market_id: u64, side: bool, limit: u64
    ): u64 acquires Registry {
        let registry = borrow_global<Registry>(@bench);
        let market = table::borrow(&registry.markets, market_id);
        let queue = if (side == BID) &market.bids else &market.asks;
        let access_key = clob_avl_queue::head_access_key(queue);
        let total = 0;
        let n = 0;
        while (access_key != 0 && n < limit) {
            total = total + clob_avl_queue::borrow(queue, access_key).size;
            access_key = clob_avl_queue::next_access_key(queue, access_key);
            n = n + 1;
        };
        total
    }

    #[view]
    /// Available and locked base, then available and locked quote.
    public fun account_state(
        user: address, market_id: u64
    ): (u64, u64, u64, u64) acquires MarketAccounts {
        let account = table::borrow(
            &borrow_global<MarketAccounts>(user).accounts, market_id);
        (
            account.base_available,
            account.base_locked,
            account.quote_available,
            account.quote_locked,
        )
    }

    #[view]
    /// Base and quote actually held in the market account's stores.
    public fun store_balances(
        user: address, market_id: u64
    ): (u64, u64) acquires MarketAccounts {
        let (base_store, quote_store) = account_stores(user, market_id);
        (clob_mock_fa::balance(base_store),
            clob_mock_fa::balance(quote_store))
    }

    public fun bid_side(): bool { BID }

    public fun ask_side(): bool { ASK }
}
