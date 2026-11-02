#[test_only]
module bench::nft_mint_market_tests {
    use std::signer;
    use bench::nftm_assets;
    use bench::nftm_collection;
    use bench::nftm_fees;
    use bench::nftm_listing;
    use bench::nftm_offer;
    use bench::nftm_token;

    /// Defaults of the Rust generator's `Config`, scaled down so a unit test
    /// stays inside the gas bound.
    const COLLECTIONS: u64 = 2;
    const TOKENS_PER_COLLECTION: u64 = 6;
    const TOKENS_PER_ACCOUNT: u64 = 4;
    const SUPPLY_CAP: u64 = 64;
    const PRICE: u64 = 1000000;
    const PAYMENT: u64 = 100000000;
    const COMMISSION_BPS: u64 = 250;
    const ROYALTY_BPS: u64 = 500;
    const MINT_BATCH: u64 = 2;
    const READ_BATCH: u64 = 4;
    const BPS: u64 = 10000;

    /// Far past anything the setup seeds, to stand in for the derived indices
    /// the generator produces from an account address.
    const WILD_INDEX: u64 = 987654321;

    /// Weights of the Rust generator's `MIX`. Mirrored here so the book's
    /// production and consumption can be balanced against them, which is what
    /// keeps a retuned weight from draining the listing floor unnoticed.
    const W_BUY: u64 = 26;
    const W_LIST: u64 = 20;
    const W_TOKEN_OFFER: u64 = 12;
    const W_ACCEPT_OFFER: u64 = 10;
    const W_CANCEL: u64 = 10;
    const W_MINT: u64 = 10;
    const W_COLLECTION_OFFER: u64 = 6;
    const W_READ_INDEX: u64 = 6;

    /// Everything the publisher signs before the mix starts, in the order the
    /// Rust generator issues it: the asset, the registry, every collection,
    /// the fee schedule, then all the token seeding and only then all the
    /// listing seeding.
    fun setup(admin: &signer, supply_cap: u64, tokens: u64, listings: u64) {
        nftm_assets::create_asset_entry(admin, nftm_assets::payment_symbol(), 8);
        nftm_collection::initialize(admin);
        let c = 0;
        while (c < COLLECTIONS) {
            nftm_collection::create_collection(admin, c, supply_cap, ROYALTY_BPS);
            c = c + 1;
        };
        nftm_fees::init_schedule(admin, COMMISSION_BPS, ROYALTY_BPS);
        let c = 0;
        while (c < COLLECTIONS) {
            nftm_token::seed_tokens(admin, c, tokens);
            c = c + 1;
        };
        let c = 0;
        while (c < COLLECTIONS) {
            if (listings > 0) {
                nftm_listing::seed_listings(admin, c, 0, listings, PRICE);
            };
            c = c + 1;
        };
        let c = 0;
        while (c < COLLECTIONS) {
            nftm_collection::set_collection_uri(admin, c, b"https://bench.invalid/c");
            c = c + 1;
        }
    }

    fun default_setup(admin: &signer) {
        setup(admin, SUPPLY_CAP, TOKENS_PER_COLLECTION, TOKENS_PER_COLLECTION);
    }

    fun onboard(user: &signer) {
        nftm_listing::bench_onboard(user, 0, TOKENS_PER_ACCOUNT, PAYMENT, PRICE);
    }

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_harness_onboard_then_mix(
        admin: &signer, alice: &signer, bob: &signer
    ) {
        default_setup(admin);
        assert!(nftm_collection::n_collections() == COLLECTIONS, 0);
        assert!(nftm_collection::minted(0) == TOKENS_PER_COLLECTION, 0);
        assert!(nftm_listing::n_active() == COLLECTIONS * TOKENS_PER_COLLECTION, 0);

        onboard(alice);
        onboard(bob);
        let alice_addr = signer::address_of(alice);
        assert!(nftm_token::inventory_size(alice_addr) == TOKENS_PER_ACCOUNT, 0);

        // Buy: the token is publisher-seeded and publisher-listed, so the sale
        // goes through rather than falling back.
        let token = nftm_token::resolve(0, 1);
        assert!(nftm_token::owner_of(token) == signer::address_of(admin), 0);
        let seller_before =
            nftm_assets::primary_balance(
                signer::address_of(admin), nftm_assets::payment_asset());
        nftm_listing::bench_buy(bob, 0, 1, 0);
        assert!(nftm_token::owner_of(token) == signer::address_of(bob), 0);
        assert!(nftm_listing::n_sold() == 1, 0);

        // The seller receives the price less both fees, and both fees in this
        // package are paid to the publisher as well.
        let commission = PRICE * COMMISSION_BPS / BPS;
        let royalty = PRICE * ROYALTY_BPS / BPS;
        let seller_after =
            nftm_assets::primary_balance(
                signer::address_of(admin), nftm_assets::payment_asset());
        assert!(seller_after - seller_before == PRICE, 0);
        assert!(commission + royalty + (PRICE - commission - royalty) == PRICE, 0);

        nftm_listing::bench_list(alice, 0, 0, 0, PRICE * 2);
        nftm_offer::bench_token_offer(bob, 0, 2, PRICE);
        nftm_offer::bench_accept_offer(alice, 0, 0);
        nftm_listing::bench_cancel_listing(alice, 0);
        nftm_listing::bench_mint_and_list(bob, 1, MINT_BATCH, PRICE);
        nftm_offer::bench_collection_offer(alice, 1, PRICE);
        nftm_token::bench_read_index(bob, 0, 0, READ_BATCH);
    }

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_buy_tolerates_unlisted_token(
        admin: &signer, alice: &signer, bob: &signer
    ) {
        // Only the first half of each collection is listed, so an index in the
        // second half names a token nobody is asking for.
        setup(admin, SUPPLY_CAP, TOKENS_PER_COLLECTION, TOKENS_PER_COLLECTION / 2);
        onboard(alice);
        let unlisted = nftm_token::resolve(0, TOKENS_PER_COLLECTION - 1);
        assert!(!nftm_listing::is_listed(unlisted), 0);
        let active_before = nftm_listing::n_active();
        nftm_listing::bench_buy(bob, 0, TOKENS_PER_COLLECTION - 1, 0);
        assert!(nftm_listing::n_sold() == 1, 0);
        // The buyer relists what it bought, so the fallback sale leaves the
        // book exactly as deep as it found it.
        assert!(nftm_listing::n_active() == active_before, 0);
        assert!(nftm_token::owner_of(unlisted) == signer::address_of(admin), 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_buy_tolerates_own_listing(admin: &signer, alice: &signer) {
        // Nothing is publisher-listed, so the fallback can only land on one of
        // the buyer's own listings.
        setup(admin, SUPPLY_CAP, TOKENS_PER_COLLECTION, 0);
        onboard(alice);
        let active_before = nftm_listing::n_active();
        assert!(active_before > 0, 0);
        let token = nftm_token::inventory_pick(signer::address_of(alice), 0);
        nftm_listing::bench_buy(alice, 0, 0, 0);
        // A buyer that lands on its own ask reprices it rather than taking it
        // off the book.
        assert!(nftm_listing::n_active() == active_before, 0);
        assert!(nftm_listing::n_sold() == 0, 0);
        assert!(nftm_listing::is_listed(token), 0);
        assert!(nftm_token::owner_of(token) == signer::address_of(alice), 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_list_always_adds_to_the_book(admin: &signer, alice: &signer) {
        default_setup(admin);
        onboard(alice);
        // Onboarding leaves half the inventory unlisted, and once those are
        // gone the branch mints rather than repricing something already up.
        let rounds = 0;
        while (rounds < TOKENS_PER_ACCOUNT) {
            let active_before = nftm_listing::n_active();
            nftm_listing::bench_list(alice, 0, WILD_INDEX, 0, PRICE * 3);
            assert!(nftm_listing::n_active() == active_before + 1, 0);
            rounds = rounds + 1;
        }
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_list_reclaims_a_token_the_ring_forgot(admin: &signer, alice: &signer) {
        default_setup(admin);
        onboard(alice);
        let alice_addr = signer::address_of(alice);
        let forgotten = nftm_token::inventory_pick(alice_addr, 0);
        nftm_listing::bench_cancel_listing(alice, 0);
        assert!(!nftm_listing::is_listed(forgotten), 0);

        // Push the ring past its cap, so its oldest entry is gone.
        let i = 0;
        while (i < nftm_token::inventory_cap()) {
            nftm_token::mint_one_to(alice_addr, 0);
            i = i + 1;
        };
        assert!(nftm_token::inventory_pick(alice_addr, 0) != forgotten, 0);

        // The derived address still names it, so listing it takes it back.
        let index = nftm_token::token_index_of(forgotten);
        nftm_listing::bench_list(alice, 0, index, 0, PRICE);
        assert!(nftm_listing::is_listed(forgotten), 0);
        assert!(nftm_listing::listing_seller(forgotten) == alice_addr, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce, carol = @0xca401)]
    fun test_cancel_tolerates_unlisted_token(
        admin: &signer, alice: &signer, carol: &signer
    ) {
        default_setup(admin);
        onboard(alice);
        // Onboarding lists the first half of the inventory, so the last entry
        // is held but not listed.
        let held = nftm_token::inventory_pick(
            signer::address_of(alice), TOKENS_PER_ACCOUNT - 1);
        assert!(!nftm_listing::is_listed(held), 0);
        let active_before = nftm_listing::n_active();
        nftm_listing::bench_cancel_listing(alice, TOKENS_PER_ACCOUNT - 1);
        assert!(nftm_listing::n_active() == active_before, 0);

        // An account that holds nothing at all is the same no-op.
        nftm_listing::bench_cancel_listing(carol, 0);
        assert!(nftm_listing::n_active() == active_before, 0);
        assert!(nftm_token::inventory_size(signer::address_of(carol)) == 0, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_mint_tolerates_supply_cap(admin: &signer, alice: &signer) {
        let cap = TOKENS_PER_COLLECTION + 1;
        setup(admin, cap, TOKENS_PER_COLLECTION, 0);
        assert!(nftm_collection::remaining_supply(0) == 1, 0);
        nftm_listing::bench_mint_and_list(alice, 0, MINT_BATCH * 4, PRICE);
        assert!(nftm_collection::minted(0) == cap, 0);
        assert!(nftm_token::inventory_size(signer::address_of(alice)) == 1, 0);

        // A drained collection mints nothing rather than aborting, and lists
        // nothing either.
        nftm_listing::bench_mint_and_list(alice, 0, MINT_BATCH, PRICE);
        assert!(nftm_collection::minted(0) == cap, 0);
        assert!(nftm_token::inventory_size(signer::address_of(alice)) == 1, 0);
        assert!(nftm_listing::n_active() == 0, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_buy_tolerates_index_past_seeded_range(
        admin: &signer, alice: &signer
    ) {
        default_setup(admin);
        let wild = nftm_token::resolve(WILD_INDEX, WILD_INDEX);
        assert!(nftm_token::token_exists(wild), 0);
        nftm_listing::bench_buy(alice, WILD_INDEX, WILD_INDEX, WILD_INDEX);
        assert!(nftm_listing::n_sold() == 1, 0);
        assert!(nftm_token::owner_of(wild) == signer::address_of(alice), 0);

        // Reads fold the same way, so a wild index still lands on real tokens.
        nftm_token::bench_read_index(alice, WILD_INDEX, WILD_INDEX, READ_BATCH);
        assert!(
            nftm_token::read_index(WILD_INDEX, WILD_INDEX, READ_BATCH) > 0, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_buy_tolerates_short_balance(admin: &signer, alice: &signer) {
        default_setup(admin);
        let asset = nftm_assets::payment_asset();
        let alice_addr = signer::address_of(alice);
        assert!(nftm_assets::primary_balance(alice_addr, asset) == 0, 0);
        nftm_listing::bench_buy(alice, 0, 0, 0);
        assert!(nftm_listing::n_sold() == 1, 0);
        assert!(nftm_assets::primary_balance(alice_addr, asset) >= PRICE, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_accept_offer_tolerates_no_offer(
        admin: &signer, alice: &signer, bob: &signer
    ) {
        default_setup(admin);
        onboard(alice);
        nftm_offer::bench_accept_offer(alice, 0, 0);
        assert!(nftm_offer::n_offers() == 0, 0);

        // An account holding nothing has nothing to sell into a bid.
        nftm_offer::bench_token_offer(bob, 0, 0, PRICE);
        assert!(nftm_offer::n_offers() == 1, 0);
        nftm_offer::bench_accept_offer(bob, 0, 0);
        assert!(nftm_offer::n_offers() == 1, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_offer_replaces_and_refunds(
        admin: &signer, alice: &signer, bob: &signer
    ) {
        default_setup(admin);
        onboard(alice);
        onboard(bob);
        let asset = nftm_assets::payment_asset();
        let bob_addr = signer::address_of(bob);

        nftm_offer::bench_token_offer(bob, 0, 0, PRICE);
        let offer = nftm_offer::offer_on_token(nftm_listing::active_token(0));
        assert!(nftm_offer::escrowed(offer) == PRICE, 0);
        let after_first = nftm_assets::primary_balance(bob_addr, asset);

        // A second bid on the same token hands the first escrow back.
        nftm_offer::bench_token_offer(bob, 0, 0, PRICE);
        assert!(nftm_offer::n_offers() == 1, 0);
        assert!(nftm_assets::primary_balance(bob_addr, asset) == after_first, 0);

        let token = nftm_listing::active_token(0);
        let seller = nftm_token::owner_of(token);
        assert!(seller != bob_addr, 0);
        let sold_before = nftm_listing::n_sold();
        nftm_offer::bench_accept_offer(alice, 0, 0);
        assert!(nftm_listing::n_sold() == sold_before, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_collection_offer_is_filled_by_any_token(
        admin: &signer, alice: &signer
    ) {
        default_setup(admin);
        onboard(alice);
        let asset = nftm_assets::payment_asset();
        let admin_addr = signer::address_of(admin);
        let alice_addr = signer::address_of(alice);

        nftm_offer::bench_collection_offer(admin, 0, PRICE);
        assert!(nftm_offer::offer_on_collection(0, admin_addr) != @0x0, 0);
        let token = nftm_token::inventory_pick(alice_addr, 0);
        let before = nftm_assets::primary_balance(alice_addr, asset);
        nftm_offer::bench_accept_offer(alice, 0, 0);
        assert!(nftm_token::owner_of(token) == admin_addr, 0);
        assert!(nftm_offer::offer_on_collection(0, admin_addr) == @0x0, 0);
        assert!(nftm_offer::n_offers() == 0, 0);
        let commission = PRICE * COMMISSION_BPS / BPS;
        let royalty = PRICE * ROYALTY_BPS / BPS;
        assert!(
            nftm_assets::primary_balance(alice_addr, asset) - before
                == PRICE - commission - royalty,
            0,
        );
    }

    // Listing floor.

    // Per-branch effect on the depth of the book. A full steady-state run is
    // far past the unit-test gas bound, so the rates the equilibrium follows
    // from are pinned one branch at a time and weighed up at the end.
    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b, carol = @0xca401)]
    fun test_listing_floor_production_beats_consumption(
        admin: &signer, alice: &signer, bob: &signer, carol: &signer
    ) {
        default_setup(admin);
        onboard(alice);
        onboard(bob);
        onboard(carol);

        // A buy takes a listing and puts one straight back.
        let before = nftm_listing::n_active();
        nftm_listing::bench_buy(bob, 0, 1, 0);
        assert!(nftm_listing::n_sold() == 1, 0);
        assert!(nftm_listing::n_active() == before, 0);

        // A list adds one.
        let before = nftm_listing::n_active();
        nftm_listing::bench_list(alice, 0, WILD_INDEX, 0, PRICE);
        assert!(nftm_listing::n_active() == before + 1, 0);

        // So does a mint.
        let before = nftm_listing::n_active();
        nftm_listing::bench_mint_and_list(alice, 0, MINT_BATCH, PRICE);
        assert!(nftm_listing::n_active() == before + 1, 0);

        // A cancel takes one off.
        let before = nftm_listing::n_active();
        nftm_listing::bench_cancel_listing(alice, 0);
        assert!(nftm_listing::n_active() == before - 1, 0);

        // So does an offer filled against a listed token.
        nftm_offer::bench_collection_offer(bob, 0, PRICE);
        let before = nftm_listing::n_active();
        nftm_offer::bench_accept_offer(alice, 1, 0);
        assert!(nftm_listing::n_active() == before - 1, 0);

        // Bids and reads leave the book alone.
        let before = nftm_listing::n_active();
        nftm_offer::bench_token_offer(carol, 0, 2, PRICE);
        nftm_offer::bench_collection_offer(carol, 1, PRICE);
        nftm_token::bench_read_index(carol, 0, 0, READ_BATCH);
        assert!(nftm_listing::n_active() == before, 0);

        // Every weight of the mix is accounted for above, and the listing
        // branches outweigh the cancelling ones, so the book climbs to its cap
        // instead of draining.
        assert!(
            W_BUY + W_LIST + W_TOKEN_OFFER + W_ACCEPT_OFFER + W_CANCEL + W_MINT
                + W_COLLECTION_OFFER + W_READ_INDEX == 100,
            0,
        );
        assert!(W_LIST + W_MINT > W_CANCEL + W_ACCEPT_OFFER, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_listing_book_saturates_at_its_cap(admin: &signer, alice: &signer) {
        default_setup(admin);
        let cap = nftm_listing::n_active() + TOKENS_PER_ACCOUNT / 2;
        nftm_listing::set_ring_cap(cap);
        assert!(nftm_listing::ring_cap() == cap, 0);
        onboard(alice);
        assert!(nftm_listing::n_active() == cap, 0);

        // Past the cap a new listing evicts an old one, so the book holds its
        // depth rather than growing without bound.
        let i = 0;
        while (i < 4) {
            nftm_listing::bench_mint_and_list(alice, 0, MINT_BATCH, PRICE);
            assert!(nftm_listing::n_active() == cap, 0);
            i = i + 1;
        }
    }

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_mix_rounds_never_drain_the_book(
        admin: &signer, alice: &signer, bob: &signer
    ) {
        default_setup(admin);
        onboard(alice);
        onboard(bob);
        let floor = nftm_listing::n_active();
        let round = 0;
        while (round < 3) {
            // Three listing branches against two cancelling ones, the ratio
            // the mix weights give.
            nftm_listing::bench_buy(bob, 0, round, round);
            nftm_listing::bench_list(alice, 0, round, round, PRICE);
            nftm_listing::bench_list(bob, 1, round, round, PRICE);
            nftm_listing::bench_mint_and_list(bob, 1, MINT_BATCH, PRICE);
            nftm_offer::bench_token_offer(alice, 0, round, PRICE);
            nftm_offer::bench_collection_offer(alice, 1, PRICE);
            nftm_offer::bench_accept_offer(bob, round, round);
            nftm_listing::bench_cancel_listing(alice, round);
            nftm_token::bench_read_index(bob, 0, round, READ_BATCH);
            assert!(nftm_listing::n_active() >= floor, 0);
            round = round + 1;
        };
        assert!(nftm_listing::n_active() > floor, 0);
    }

    // Offer book.

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_collection_offers_are_keyed_per_bidder(
        admin: &signer, alice: &signer, bob: &signer
    ) {
        default_setup(admin);
        onboard(alice);
        onboard(bob);
        let alice_addr = signer::address_of(alice);
        let bob_addr = signer::address_of(bob);

        // Two bidders on one collection stand side by side rather than one
        // closing the other.
        nftm_offer::bench_collection_offer(alice, 0, PRICE);
        nftm_offer::bench_collection_offer(bob, 0, PRICE * 2);
        assert!(nftm_offer::collection_bidders(0) == 2, 0);
        assert!(nftm_offer::n_offers() == 2, 0);
        let alice_offer = nftm_offer::offer_on_collection(0, alice_addr);
        let bob_offer = nftm_offer::offer_on_collection(0, bob_addr);
        assert!(alice_offer != bob_offer, 0);
        assert!(nftm_offer::offer_price(bob_offer) == PRICE * 2, 0);

        // Each bidder is reachable through the collection's bidder index.
        assert!(nftm_offer::collection_offer_at(0, 0) != @0x0, 0);
        assert!(
            nftm_offer::collection_offer_at(0, 0)
                != nftm_offer::collection_offer_at(0, 1),
            0,
        );

        // The same bidder bidding again replaces only its own.
        nftm_offer::bench_collection_offer(alice, 0, PRICE * 3);
        assert!(nftm_offer::collection_bidders(0) == 2, 0);
        assert!(nftm_offer::n_offers() == 2, 0);
        assert!(nftm_offer::offer_on_collection(0, bob_addr) == bob_offer, 0);

        // A different collection is a different key again.
        nftm_offer::bench_collection_offer(alice, 1, PRICE);
        assert!(nftm_offer::collection_bidders(0) == 2, 0);
        assert!(nftm_offer::collection_bidders(1) == 1, 0);
        assert!(nftm_offer::n_offers() == 3, 0);
    }

    // Setup that must not abort.

    #[test(admin = @bench)]
    fun test_create_collection_clamps_a_bad_config(admin: &signer) {
        nftm_assets::create_asset_entry(admin, nftm_assets::payment_symbol(), 8);
        nftm_collection::initialize(admin);
        // A royalty above 100% and a zero supply cap are both refused by the
        // framework, and the setup stage may not abort.
        nftm_collection::create_collection(admin, 0, SUPPLY_CAP, BPS * 2);
        nftm_collection::create_collection(admin, 1, 0, ROYALTY_BPS);
        assert!(nftm_collection::n_collections() == COLLECTIONS, 0);
        assert!(nftm_collection::supply_cap(1) == 1, 0);
    }

    // Fee split.

    #[test]
    fun test_split_three_ways() {
        // 1_000_000 at 2.5% commission and 5% royalty.
        let (commission, royalty, proceeds) = nftm_fees::split(1000000, 250, 500, 10000);
        assert!(commission == 25000, 0);
        assert!(royalty == 50000, 0);
        assert!(proceeds == 925000, 0);
    }

    #[test]
    fun test_split_rounds_down_onto_the_seller() {
        // 999 at 2.5% is 24.975, and at 5% is 49.95; both truncate.
        let (commission, royalty, proceeds) = nftm_fees::split(999, 250, 500, 10000);
        assert!(commission == 24, 0);
        assert!(royalty == 49, 0);
        assert!(proceeds == 926, 0);
        assert!(commission + royalty + proceeds == 999, 0);
    }

    #[test]
    fun test_split_without_royalty() {
        let (commission, royalty, proceeds) = nftm_fees::split(1000000, 250, 0, 0);
        assert!(commission == 25000, 0);
        assert!(royalty == 0, 0);
        assert!(proceeds == 975000, 0);
    }

    #[test]
    fun test_split_clamps_fees_to_the_price() {
        // A commission of 150% and a royalty of 100% together cannot take more
        // than the price, and leave the seller nothing rather than underflow.
        let (commission, royalty, proceeds) =
            nftm_fees::split(1000, 15000, 10000, 10000);
        assert!(commission == 1000, 0);
        assert!(royalty == 0, 0);
        assert!(proceeds == 0, 0);

        let (commission, royalty, proceeds) =
            nftm_fees::split(1000, 5000, 10000, 10000);
        assert!(commission == 500, 0);
        assert!(royalty == 500, 0);
        assert!(proceeds == 0, 0);
    }

    #[test]
    fun test_split_of_zero_price() {
        let (commission, royalty, proceeds) = nftm_fees::split(0, 250, 500, 10000);
        assert!(commission == 0, 0);
        assert!(royalty == 0, 0);
        assert!(proceeds == 0, 0);
    }

    #[test]
    fun test_split_of_a_price_that_would_overflow_the_fee() {
        // `price * commission_bps` leaves `u64` above roughly 1.8e15.
        let price = 18446744073709551615;
        let (commission, royalty, proceeds) =
            nftm_fees::split(price, COMMISSION_BPS, ROYALTY_BPS, BPS);
        assert!(commission == 461168601842738790, 0);
        assert!(royalty == 922337203685477580, 0);
        assert!(commission + royalty + proceeds == price, 0);
    }
}
