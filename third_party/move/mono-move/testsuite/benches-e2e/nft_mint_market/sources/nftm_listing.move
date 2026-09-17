/// Fixed-price listings, shaped after Topaz on Aptos: a listing is a record
/// keyed by the token's derived address, and a sale moves the token object and
/// splits the price three ways.
///
/// Active listings are also held in a dense slot table, so a buy that misses
/// can pick another listing in one table read instead of scanning. The slot
/// table is what lets `bench_buy` fall through instead of aborting.
module bench::nftm_listing {
    use std::signer;
    use aptos_framework::primary_fungible_store;
    use aptos_std::table::{Self, Table};
    use bench::nftm_assets;
    use bench::nftm_collection;
    use bench::nftm_events;
    use bench::nftm_fees;
    use bench::nftm_token;

    /// Only the package address can seed listings.
    const E_NOT_BENCH: u64 = 1;

    /// Listings standing at once. The mix lists faster than it cancels, so the
    /// book fills to this and holds there: a run of any length leaves it the
    /// same size, and a buy always has a floor to fill against.
    const LISTING_RING_CAP: u64 = 4096;

    /// Inventory entries `bench_list` looks at before it gives up and mints.
    const LIST_PROBES: u64 = 4;

    struct Listing has store, drop {
        seller: address,
        price: u64,
        /// Position in the slot table, so a removal is a swap rather than a
        /// scan.
        slot: u64,
    }

    struct Book has key {
        listings: Table<address, Listing>,
        slots: Table<u64, address>,
        n_active: u64,
        n_sold: u64,
        cap: u64,
        /// Slot the next eviction takes, so eviction walks the whole book
        /// instead of retiring the same end of it over and over.
        evict_cursor: u64,
    }

    /// The book lives at the collection-creator object, which every module can
    /// obtain a signer for, so no entry point depends on a separate admin
    /// setup call having run first.
    fun book_address(): address {
        nftm_collection::creator_address()
    }

    fun ensure_book() {
        if (!exists<Book>(book_address())) {
            move_to(
                &nftm_collection::creator_signer(),
                Book {
                    listings: table::new(),
                    slots: table::new(),
                    n_active: 0,
                    n_sold: 0,
                    cap: LISTING_RING_CAP,
                    evict_cursor: 0,
                },
            );
        }
    }

    // Book maintenance.

    /// List `token_addr` for `seller`, or hand an existing listing to `seller`
    /// at `price` when the token is already listed.
    fun list_internal(token_addr: address, seller: address, price: u64) acquires Book {
        if (!is_listed(token_addr)) {
            evict_if_full();
        };
        let book = borrow_global_mut<Book>(book_address());
        if (table::contains(&book.listings, token_addr)) {
            let listing = table::borrow_mut(&mut book.listings, token_addr);
            listing.seller = seller;
            listing.price = price;
        } else {
            let slot = book.n_active;
            table::add(&mut book.slots, slot, token_addr);
            table::add(
                &mut book.listings, token_addr, Listing { seller, price, slot });
            book.n_active = slot + 1;
        };
        nftm_events::emit_list(token_addr, seller, price);
    }

    /// Retire one listing when the book is at its cap, so that a new one has
    /// room without the book ever growing.
    fun evict_if_full() acquires Book {
        let victim = next_victim();
        if (victim != @0x0) {
            delist_if_listed(victim);
        }
    }

    fun next_victim(): address acquires Book {
        let book = borrow_global_mut<Book>(book_address());
        if (book.n_active < book.cap) return @0x0;
        let slot = book.evict_cursor % book.n_active;
        book.evict_cursor = book.evict_cursor + 1;
        *table::borrow(&book.slots, slot)
    }

    /// Drop a listing and close the hole it leaves in the slot table by
    /// moving the last entry into it.
    public fun delist_if_listed(token_addr: address) acquires Book {
        if (!exists<Book>(book_address())) return;
        let book = borrow_global_mut<Book>(book_address());
        if (!table::contains(&book.listings, token_addr)) return;
        let slot = table::borrow(&book.listings, token_addr).slot;
        let last = book.n_active - 1;
        let moved = table::remove(&mut book.slots, last);
        if (slot != last) {
            table::upsert(&mut book.slots, slot, moved);
            table::borrow_mut(&mut book.listings, moved).slot = slot;
        };
        table::remove(&mut book.listings, token_addr);
        book.n_active = last;
    }

    /// Whether `token_addr` is listed, and by whom at what price.
    fun listing_of(token_addr: address): (bool, address, u64) acquires Book {
        if (!exists<Book>(book_address())) return (false, @0x0, 0);
        let book = borrow_global<Book>(book_address());
        if (!table::contains(&book.listings, token_addr)) {
            return (false, @0x0, 0)
        };
        let listing = table::borrow(&book.listings, token_addr);
        (true, listing.seller, listing.price)
    }

    /// The token listed in slot `hint` modulo the number of active listings,
    /// or `@0x0` when nothing is listed.
    public fun active_token(hint: u64): address acquires Book {
        if (!exists<Book>(book_address())) return @0x0;
        let book = borrow_global<Book>(book_address());
        if (book.n_active == 0) return @0x0;
        *table::borrow(&book.slots, hint % book.n_active)
    }

    fun note_sold() acquires Book {
        if (!exists<Book>(book_address())) return;
        let book = borrow_global_mut<Book>(book_address());
        book.n_sold = book.n_sold + 1;
    }

    // Settlement.

    /// Pay `price` from `buyer`'s primary store out to the commission payee,
    /// the royalty payee and `seller`, then hand the token over. The buyer is
    /// topped up first, so a long run never runs a payment short.
    public fun settle(
        buyer: &signer, seller: address, token_addr: address, price: u64
    ) acquires Book {
        let buyer_addr = signer::address_of(buyer);
        let asset = nftm_assets::payment_asset();
        nftm_assets::ensure_balance(asset, buyer_addr, price);
        let payment = primary_fungible_store::withdraw(buyer, asset, price);
        let (commission, royalty, _) =
            nftm_fees::payout(payment, price, token_addr, seller);
        nftm_token::transfer(token_addr, buyer_addr);
        delist_if_listed(token_addr);
        note_sold();
        nftm_events::emit_sale(
            token_addr, seller, buyer_addr, price, commission, royalty);
    }

    // Publisher-signed setup.

    /// List `count` publisher-owned tokens starting at `start`, so a buy has
    /// something to fill from the first mix transaction onwards.
    public entry fun seed_listings(
        admin: &signer,
        collection_index: u64,
        start: u64,
        count: u64,
        price: u64,
    ) acquires Book {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        ensure_book();
        let admin_addr = signer::address_of(admin);
        let i = 0;
        while (i < count) {
            let token_addr = nftm_token::resolve(collection_index, start + i);
            if (nftm_token::token_exists(token_addr)
                && nftm_token::owner_of(token_addr) == admin_addr) {
                list_internal(token_addr, admin_addr, price);
            };
            i = i + 1;
        }
    }

    // Mix entry points.

    /// Bring a fresh account to the state the mix assumes: funded with the
    /// payment asset, holding `n_tokens` of its own, half of them listed.
    public entry fun bench_onboard(
        user: &signer,
        collection_index: u64,
        n_tokens: u64,
        payment: u64,
        price: u64,
    ) acquires Book {
        let user_addr = signer::address_of(user);
        nftm_assets::faucet(nftm_assets::payment_asset(), user_addr, payment);
        nftm_token::ensure_inventory(user);
        ensure_book();
        let minted = nftm_token::mint_to(user_addr, collection_index, n_tokens);
        let half = minted / 2;
        let i = 0;
        while (i < half) {
            let token_addr = nftm_token::owned_pick(user_addr, i);
            if (token_addr != @0x0) {
                list_internal(token_addr, user_addr, price);
            };
            i = i + 1;
        }
    }

    /// Buy the token at a derived address, falling through to whatever is
    /// listed at `fallback_hint` when that one is not for sale.
    ///
    /// Every branch leaves the listing standing: the buyer flips what it just
    /// bought, a buyer that lands on its own ask reprices it, and a stale ask
    /// is re-pointed at whoever holds the token now. A buy is therefore the
    /// heaviest branch of the mix without being a drain on the book.
    public entry fun bench_buy(
        user: &signer,
        collection_index: u64,
        token_index: u64,
        fallback_hint: u64,
    ) acquires Book {
        let buyer = signer::address_of(user);
        nftm_token::ensure_inventory(user);
        let token_addr = nftm_token::resolve(collection_index, token_index);
        if (!is_listed(token_addr)) {
            token_addr = active_token(fallback_hint);
            if (token_addr == @0x0) return;
        };
        let (listed, seller, price) = listing_of(token_addr);
        if (!listed) return;
        if (seller == buyer) {
            list_internal(token_addr, buyer, price);
            return
        };
        let owner = nftm_token::owner_of(token_addr);
        if (owner != seller) {
            list_internal(token_addr, owner, price);
            return
        };
        settle(user, seller, token_addr, price);
        list_internal(token_addr, buyer, price);
    }

    /// Put one more token on the book. The caller's own unlisted tokens come
    /// first, and an account holding none mints one, so the branch adds a
    /// listing whatever state it finds the caller in.
    ///
    /// A token the caller owns but its inventory ring has since dropped is
    /// still named by `token_index`, and the first arm takes it back.
    public entry fun bench_list(
        user: &signer,
        collection_index: u64,
        token_index: u64,
        hint: u64,
        price: u64,
    ) acquires Book {
        let user_addr = signer::address_of(user);
        nftm_token::ensure_inventory(user);
        ensure_book();

        let derived = nftm_token::resolve(collection_index, token_index);
        if (nftm_token::token_exists(derived)
            && nftm_token::owner_of(derived) == user_addr
            && !is_listed(derived)) {
            nftm_token::inventory_push(user_addr, derived);
            list_internal(derived, user_addr, price);
            return
        };

        let i = 0;
        while (i < LIST_PROBES) {
            let token_addr = nftm_token::owned_pick(user_addr, hint + i);
            if (token_addr != @0x0 && !is_listed(token_addr)) {
                list_internal(token_addr, user_addr, price);
                return
            };
            i = i + 1;
        };

        let minted = nftm_token::mint_one_to(user_addr, collection_index);
        if (minted != @0x0) {
            list_internal(minted, user_addr, price);
        }
    }

    /// Mint `n` tokens to the caller and put the last one up for sale. A
    /// drained collection mints nothing rather than aborting.
    public entry fun bench_mint_and_list(
        user: &signer, collection_index: u64, n: u64, price: u64
    ) acquires Book {
        let user_addr = signer::address_of(user);
        nftm_token::ensure_inventory(user);
        ensure_book();
        if (n > 1) {
            nftm_token::mint_to(user_addr, collection_index, n - 1);
        };
        let minted = nftm_token::mint_one_to(user_addr, collection_index);
        if (minted != @0x0) {
            list_internal(minted, user_addr, price);
        }
    }

    /// Cancel the caller's listing on one of its own tokens. A token that is
    /// not listed, or is listed by somebody else, is left alone.
    public entry fun bench_cancel_listing(user: &signer, hint: u64) acquires Book {
        let user_addr = signer::address_of(user);
        nftm_token::ensure_inventory(user);
        let token_addr = nftm_token::inventory_pick(user_addr, hint);
        if (token_addr == @0x0) return;
        let (listed, seller, _) = listing_of(token_addr);
        if (!listed || seller != user_addr) return;
        delist_if_listed(token_addr);
        nftm_events::emit_cancel(token_addr, user_addr);
    }

    // Views.

    #[view]
    public fun is_listed(token_addr: address): bool acquires Book {
        let (listed, _, _) = listing_of(token_addr);
        listed
    }

    #[view]
    public fun listing_price(token_addr: address): u64 acquires Book {
        let (_, _, price) = listing_of(token_addr);
        price
    }

    #[view]
    public fun listing_seller(token_addr: address): address acquires Book {
        let (_, seller, _) = listing_of(token_addr);
        seller
    }

    #[view]
    public fun n_active(): u64 acquires Book {
        if (!exists<Book>(book_address())) 0
        else borrow_global<Book>(book_address()).n_active
    }

    #[view]
    public fun n_sold(): u64 acquires Book {
        if (!exists<Book>(book_address())) 0
        else borrow_global<Book>(book_address()).n_sold
    }

    #[view]
    public fun ring_cap(): u64 acquires Book {
        if (!exists<Book>(book_address())) LISTING_RING_CAP
        else borrow_global<Book>(book_address()).cap
    }

    #[test_only]
    /// Shrink the book so a test can reach saturation without listing
    /// `LISTING_RING_CAP` tokens.
    public fun set_ring_cap(cap: u64) acquires Book {
        ensure_book();
        borrow_global_mut<Book>(book_address()).cap = cap;
    }
}
