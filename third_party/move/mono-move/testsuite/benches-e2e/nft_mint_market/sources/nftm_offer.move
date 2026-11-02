/// Bids, shaped after Topaz on Aptos: an offer is its own object holding the
/// buyer's payment in a `FungibleStore`, and filling one deletes the object.
///
/// A token offer names one token. A collection offer names none and is filled
/// against whichever token of that collection the accepting account holds, so
/// a fill costs one extra table read either way.
module bench::nftm_offer {
    use std::signer;
    use aptos_framework::fungible_asset::{Self, FungibleStore};
    use aptos_framework::object::{Self, DeleteRef, Object};
    use aptos_framework::primary_fungible_store;
    use aptos_std::table::{Self, Table};
    use bench::nftm_assets;
    use bench::nftm_collection;
    use bench::nftm_events;
    use bench::nftm_fees;
    use bench::nftm_listing;
    use bench::nftm_token;

    /// Offers standing at once. Placing past the cap refunds the oldest one,
    /// so a run of any length leaves the book the same size and the benchmark
    /// measures the marketplace rather than a growing table.
    const OFFER_RING_CAP: u64 = 4096;

    /// A collection offer carries no token, and no offer stands on a token
    /// whose slot reads back as this.
    const NONE: address = @0x0;

    /// One collection offer stands per bidder per collection, so the book is
    /// as wide as the account pool rather than as wide as the collection
    /// count.
    struct CollectionKey has copy, drop, store {
        collection_index: u64,
        bidder: address,
    }

    /// Dense index over the bidders standing on one collection, so an account
    /// selling into a collection offer picks a bidder in a single read.
    struct CollectionSlot has copy, drop, store {
        collection_index: u64,
        slot: u64,
    }

    struct Offer has key {
        token: address,
        collection_index: u64,
        buyer: address,
        price: u64,
        /// Position in the slot table, so a removal is a swap rather than a
        /// scan.
        slot: u64,
        /// Position among the collection's bidders. Only a collection offer
        /// occupies one.
        collection_slot: u64,
        store: Object<FungibleStore>,
        delete_ref: DeleteRef,
    }

    struct OfferBook has key {
        by_token: Table<address, address>,
        by_collection: Table<CollectionKey, address>,
        collection_bidders: Table<CollectionSlot, address>,
        collection_counts: Table<u64, u64>,
        slots: Table<u64, address>,
        n_offers: u64,
    }

    /// The book lives at the collection-creator object, which every module can
    /// obtain a signer for, so no entry point depends on a separate admin
    /// setup call having run first.
    fun book_address(): address {
        nftm_collection::creator_address()
    }

    fun ensure_book() {
        if (!exists<OfferBook>(book_address())) {
            move_to(
                &nftm_collection::creator_signer(),
                OfferBook {
                    by_token: table::new(),
                    by_collection: table::new(),
                    collection_bidders: table::new(),
                    collection_counts: table::new(),
                    slots: table::new(),
                    n_offers: 0,
                },
            );
        }
    }

    // Placing.

    /// Escrow `price` from `user` in a fresh object and stand it on `token`, or
    /// on `collection_index` for `user` when `token` is `NONE`. One bid stands
    /// per key, so whatever was there is refunded first.
    fun place(
        user: &signer, token: address, collection_index: u64, price: u64
    ) acquires Offer, OfferBook {
        ensure_book();
        let buyer = signer::address_of(user);
        let asset = nftm_assets::payment_asset();
        nftm_assets::ensure_balance(asset, buyer, price);

        if (token == NONE) {
            close_collection_offer(collection_index, buyer);
        } else {
            close_token_offer(token);
        };
        evict_if_full();

        let constructor_ref = object::create_object(buyer);
        let offer_addr = object::address_from_constructor_ref(&constructor_ref);
        let store = fungible_asset::create_store(&constructor_ref, asset);
        nftm_assets::deposit(
            asset, store, primary_fungible_store::withdraw(user, asset, price));

        let book = borrow_global_mut<OfferBook>(book_address());
        let slot = book.n_offers;
        table::add(&mut book.slots, slot, offer_addr);
        book.n_offers = slot + 1;
        let collection_slot = 0;
        if (token == NONE) {
            table::add(
                &mut book.by_collection,
                CollectionKey { collection_index, bidder: buyer },
                offer_addr,
            );
            collection_slot = bidder_count(book, collection_index);
            table::add(
                &mut book.collection_bidders,
                CollectionSlot { collection_index, slot: collection_slot },
                buyer,
            );
            table::upsert(
                &mut book.collection_counts,
                collection_index,
                collection_slot + 1,
            );
        } else {
            table::add(&mut book.by_token, token, offer_addr);
        };

        move_to(
            &object::generate_signer(&constructor_ref),
            Offer {
                token,
                collection_index,
                buyer,
                price,
                slot,
                collection_slot,
                store,
                delete_ref: object::generate_delete_ref(&constructor_ref),
            },
        );
        nftm_events::emit_offer_placed(
            offer_addr, buyer, token, collection_index, price);
    }

    fun bidder_count(book: &OfferBook, collection_index: u64): u64 {
        if (!table::contains(&book.collection_counts, collection_index)) 0
        else *table::borrow(&book.collection_counts, collection_index)
    }

    fun evict_if_full() acquires Offer, OfferBook {
        let book = borrow_global<OfferBook>(book_address());
        if (book.n_offers < OFFER_RING_CAP) return;
        let victim = *table::borrow(&book.slots, 0);
        refund_and_close(victim);
    }

    // Closing.

    fun close_token_offer(token: address) acquires Offer, OfferBook {
        let offer_addr = offer_on_token(token);
        if (offer_addr != NONE) {
            refund_and_close(offer_addr);
        }
    }

    fun close_collection_offer(
        collection_index: u64, bidder: address
    ) acquires Offer, OfferBook {
        let offer_addr = offer_on_collection(collection_index, bidder);
        if (offer_addr != NONE) {
            refund_and_close(offer_addr);
        }
    }

    /// Hand the escrow back to its buyer and delete the offer object.
    fun refund_and_close(offer_addr: address) acquires Offer, OfferBook {
        let offer = move_from<Offer>(offer_addr);
        let buyer = offer.buyer;
        drain_to_primary(&offer, buyer);
        deregister(&offer);
        destroy(offer);
        nftm_events::emit_offer_closed(offer_addr, buyer, true);
    }

    /// Empty the escrow store into `to`'s primary store. The store has to be
    /// empty before it can be removed, so every close path runs this.
    fun drain_to_primary(offer: &Offer, to: address) {
        let balance = nftm_assets::balance(offer.store);
        if (balance > 0) {
            primary_fungible_store::deposit(
                to,
                nftm_assets::withdraw(
                    nftm_assets::payment_asset(), offer.store, balance),
            );
        }
    }

    /// Drop the offer out of the slot ring and its lookup table, closing the
    /// hole it leaves with the last entry.
    fun deregister(offer: &Offer) acquires Offer, OfferBook {
        if (offer.token == NONE) {
            deregister_bidder(offer);
        };
        let book = borrow_global_mut<OfferBook>(book_address());
        let last = book.n_offers - 1;
        let moved = table::remove(&mut book.slots, last);
        book.n_offers = last;
        if (offer.token == NONE) {
            table::remove(
                &mut book.by_collection,
                CollectionKey {
                    collection_index: offer.collection_index,
                    bidder: offer.buyer,
                },
            );
        } else {
            table::remove(&mut book.by_token, offer.token);
        };
        if (offer.slot != last) {
            table::upsert(&mut book.slots, offer.slot, moved);
            patch_slot(moved, offer.slot);
        }
    }

    /// Take the bidder out of its collection's dense bidder index, closing the
    /// hole with the last bidder standing on that collection.
    fun deregister_bidder(offer: &Offer) acquires Offer, OfferBook {
        let collection_index = offer.collection_index;
        let book = borrow_global_mut<OfferBook>(book_address());
        let last = bidder_count(book, collection_index) - 1;
        let moved = table::remove(
            &mut book.collection_bidders,
            CollectionSlot { collection_index, slot: last },
        );
        table::upsert(&mut book.collection_counts, collection_index, last);
        if (offer.collection_slot == last) return;
        table::upsert(
            &mut book.collection_bidders,
            CollectionSlot { collection_index, slot: offer.collection_slot },
            moved,
        );
        let moved_offer = *table::borrow(
            &book.by_collection,
            CollectionKey { collection_index, bidder: moved },
        );
        patch_collection_slot(moved_offer, offer.collection_slot);
    }

    fun patch_slot(offer_addr: address, slot: u64) acquires Offer {
        borrow_global_mut<Offer>(offer_addr).slot = slot;
    }

    fun patch_collection_slot(offer_addr: address, slot: u64) acquires Offer {
        borrow_global_mut<Offer>(offer_addr).collection_slot = slot;
    }

    fun destroy(offer: Offer) {
        let Offer {
            token: _,
            collection_index: _,
            buyer: _,
            price: _,
            slot: _,
            collection_slot: _,
            store: _,
            delete_ref,
        } = offer;
        fungible_asset::remove_store(&delete_ref);
        object::delete(delete_ref);
    }

    // Mix entry points.

    /// Bid on a token somebody is already asking for, which is what a real
    /// marketplace sees. With nothing listed, bid on a derived address.
    public entry fun bench_token_offer(
        user: &signer, collection_index: u64, token_index: u64, price: u64
    ) acquires Offer, OfferBook {
        let token = nftm_listing::active_token(token_index);
        if (token == NONE) {
            token = nftm_token::resolve(collection_index, token_index);
        };
        if (!nftm_token::token_exists(token)) return;
        place(user, token, nftm_token::collection_of(token), price);
    }

    /// Bid on every token of a collection at once. The bid is the caller's
    /// own, so the book holds one per bidder per collection.
    public entry fun bench_collection_offer(
        user: &signer, collection_index: u64, price: u64
    ) acquires Offer, OfferBook {
        place(user, NONE, nftm_collection::wrap_collection(collection_index), price);
    }

    /// Sell one of the caller's tokens into the bid standing on it, or failing
    /// that into one of the bids standing on its collection. No bid, no token,
    /// and the caller's own bid are all handled without aborting.
    public entry fun bench_accept_offer(
        user: &signer, hint: u64, offer_hint: u64
    ) acquires Offer, OfferBook {
        let seller = signer::address_of(user);
        nftm_token::ensure_inventory(user);
        let token = nftm_token::owned_pick(seller, hint);
        if (token == NONE) return;
        if (!exists<OfferBook>(book_address())) return;
        let offer_addr = lookup(token, offer_hint);
        if (offer_addr == NONE) return;
        if (buyer_of(offer_addr) == seller) {
            refund_and_close(offer_addr);
            return
        };
        settle(offer_addr, token);
    }

    /// The bid standing on `token`, else one of the bids standing on its
    /// collection.
    fun lookup(token: address, offer_hint: u64): address acquires OfferBook {
        let on_token = offer_on_token(token);
        if (on_token != NONE) return on_token;
        collection_offer_at(nftm_token::collection_of(token), offer_hint)
    }

    /// Pay the escrow out three ways, hand the token to the bidder and delete
    /// the offer object.
    fun settle(offer_addr: address, token: address) acquires Offer, OfferBook {
        let offer = move_from<Offer>(offer_addr);
        let buyer = offer.buyer;
        let seller = nftm_token::owner_of(token);
        // The escrow rather than the quoted price, so that the store is left
        // empty and can be removed with the object.
        let amount = nftm_assets::balance(offer.store);
        let payment = nftm_assets::withdraw(
            nftm_assets::payment_asset(), offer.store, amount);
        let (commission, royalty, _) =
            nftm_fees::payout(payment, amount, token, seller);
        nftm_token::transfer(token, buyer);
        nftm_listing::delist_if_listed(token);
        deregister(&offer);
        destroy(offer);
        nftm_events::emit_sale(
            token, seller, buyer, amount, commission, royalty);
        nftm_events::emit_offer_closed(offer_addr, buyer, false);
    }

    // Views.

    #[view]
    public fun n_offers(): u64 acquires OfferBook {
        if (!exists<OfferBook>(book_address())) 0
        else borrow_global<OfferBook>(book_address()).n_offers
    }

    #[view]
    public fun offer_on_token(token: address): address acquires OfferBook {
        if (!exists<OfferBook>(book_address())) return NONE;
        let book = borrow_global<OfferBook>(book_address());
        if (!table::contains(&book.by_token, token)) return NONE;
        *table::borrow(&book.by_token, token)
    }

    #[view]
    public fun offer_on_collection(
        collection_index: u64, bidder: address
    ): address acquires OfferBook {
        if (!exists<OfferBook>(book_address())) return NONE;
        let book = borrow_global<OfferBook>(book_address());
        let key = CollectionKey { collection_index, bidder };
        if (!table::contains(&book.by_collection, key)) return NONE;
        *table::borrow(&book.by_collection, key)
    }

    #[view]
    /// The bid of the collection's `hint`-th bidder, or `NONE` when nobody is
    /// bidding on it.
    public fun collection_offer_at(
        collection_index: u64, hint: u64
    ): address acquires OfferBook {
        if (!exists<OfferBook>(book_address())) return NONE;
        let book = borrow_global<OfferBook>(book_address());
        let count = bidder_count(book, collection_index);
        if (count == 0) return NONE;
        let bidder = *table::borrow(
            &book.collection_bidders,
            CollectionSlot { collection_index, slot: hint % count },
        );
        *table::borrow(
            &book.by_collection, CollectionKey { collection_index, bidder })
    }

    #[view]
    public fun collection_bidders(collection_index: u64): u64 acquires OfferBook {
        if (!exists<OfferBook>(book_address())) 0
        else bidder_count(borrow_global<OfferBook>(book_address()), collection_index)
    }

    #[view]
    public fun buyer_of(offer_addr: address): address acquires Offer {
        borrow_global<Offer>(offer_addr).buyer
    }

    #[view]
    public fun offer_price(offer_addr: address): u64 acquires Offer {
        borrow_global<Offer>(offer_addr).price
    }

    #[view]
    public fun escrowed(offer_addr: address): u64 acquires Offer {
        nftm_assets::balance(borrow_global<Offer>(offer_addr).store)
    }

    public fun ring_cap(): u64 {
        OFFER_RING_CAP
    }
}
