#[test_only]
module bench::cdp_liquidation_tests {
    use std::signer;
    use std::vector;
    use bench::cdp_assets;
    use bench::cdp_auction;
    use bench::cdp_oracle;
    use bench::cdp_sorted;
    use bench::cdp_stability;
    use bench::cdp_vault;

    // The default `Config` the Rust generator runs with, except for the list
    // length, which a unit test shrinks to stay inside the gas bound.
    const LIST_LENGTH: u64 = 20;
    const WALK_LIMIT: u64 = 8;
    const LIQUIDATIONS_PER_TXN: u64 = 4;
    const HINT_STRIDE: u64 = 4;
    const MCR_BPS: u64 = 11000;
    const PRICE_STEP: u64 = 37;
    const AUCTION_LOTS: u64 = 8;

    const START_PRICE: u64 = 2000;
    const COLL_DECIMALS: u8 = 8;
    const STABLE_DECIMALS: u8 = 6;

    const ONBOARD_COLL: u64 = 150;
    const ONBOARD_ICR_BPS: u64 = 25000;
    const POOL_FUNDING: u64 = 1000000000000;
    const DEPOSIT_AMOUNT: u64 = 1000000;
    const ADJUST_COLL: u64 = 4;
    const ADJUST_DEBT: u64 = 20000;

    /// Debt at a 250% ratio on 150 collateral at the start price.
    const ONBOARD_DEBT: u64 = 120000;

    /// A walk cap no test list is long enough to reach.
    const UNCAPPED: u64 = 1000;

    /// Prices a band sweep stops at, evenly spaced over the oracle's band.
    const BAND_STOPS: u64 = 8;

    /// Sweeps of the band the renewal test runs. Long enough that the head of
    /// the list has turned over several times, short enough to stay inside
    /// the unit test gas bound.
    const RENEWAL_ROUNDS: u64 = 32;

    /// Every publisher-signed call the Rust generator issues, in order.
    fun init_package(admin: &signer) {
        cdp_assets::create_asset_entry(admin, b"CDPC", COLL_DECIMALS);
        cdp_assets::create_asset_entry(admin, b"CDPS", STABLE_DECIMALS);
        cdp_vault::initialize(admin, MCR_BPS);
        cdp_oracle::set_price(admin, START_PRICE);
        cdp_sorted::seed_vaults(admin, 0, LIST_LENGTH);
        cdp_stability::fund(admin, POOL_FUNDING);
        cdp_auction::initialize(admin, AUCTION_LOTS);
    }

    /// The node the generator hands an account whose slot is `slot`.
    fun hint(slot: u64): address {
        cdp_vault::vault_address(@bench, slot * HINT_STRIDE)
    }

    fun vault_of(user: &signer): address {
        cdp_vault::vault_address(
            signer::address_of(user), cdp_sorted::user_vault_index())
    }

    /// One call of every mix branch, in the order the weights are declared.
    /// The generator relies on the list keeping its length, so the length is
    /// checked after each individual branch rather than after the round: no
    /// branch adds or drops a node, however the price and the positions move.
    fun run_mix(user: &signer, grow: bool, hint: address) {
        let len = cdp_sorted::len();
        cdp_sorted::bench_walk(user, hint, WALK_LIMIT);
        assert!(cdp_sorted::len() == len, 0);
        cdp_sorted::bench_adjust(user, ADJUST_COLL, ADJUST_DEBT, grow, hint);
        assert!(cdp_sorted::len() == len, 0);
        cdp_stability::bench_liquidate(user, LIQUIDATIONS_PER_TXN, WALK_LIMIT);
        assert!(cdp_sorted::len() == len, 0);
        cdp_stability::bench_deposit(user, DEPOSIT_AMOUNT);
        assert!(cdp_sorted::len() == len, 0);
        cdp_oracle::bench_price_tick(user, PRICE_STEP);
        assert!(cdp_sorted::len() == len, 0);
        cdp_auction::bench_auction_step(user, AUCTION_LOTS);
        assert!(cdp_sorted::len() == len, 0);
        cdp_sorted::bench_close_reopen(user, hint);
        assert!(cdp_sorted::len() == len, 0);
    }

    /// The `i`th of `BAND_STOPS` prices spread over the oracle's band.
    fun band_stop(i: u64): u64 {
        cdp_oracle::price_min() + cdp_oracle::price_span() * i / BAND_STOPS
    }

    /// Colls of the list from the head, which is enough to identify order
    /// when every test vault has a distinct one.
    fun colls_in_order(): vector<u64> {
        let out = vector[];
        let cur = cdp_sorted::head();
        let n = 0;
        while (cur != @0x0 && n < UNCAPPED) {
            let (coll, _) = cdp_vault::coll_debt_of(cur);
            vector::push_back(&mut out, coll);
            cur = cdp_vault::next_of(cur);
            n = n + 1;
        };
        out
    }

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_harness_onboard_then_mix(
        admin: &signer, alice: &signer, bob: &signer
    ) {
        init_package(admin);
        assert!(cdp_sorted::len() == LIST_LENGTH, 0);
        // The hints the generator derives have to land on seeded vaults.
        assert!(cdp_vault::exists_vault(hint(0)), 0);
        assert!(cdp_vault::exists_vault(hint(2)), 0);

        cdp_sorted::bench_onboard(
            alice, ONBOARD_COLL, ONBOARD_ICR_BPS, hint(0));
        cdp_sorted::bench_onboard(bob, ONBOARD_COLL, ONBOARD_ICR_BPS, hint(2));
        assert!(cdp_sorted::len() == LIST_LENGTH + 2, 0);
        let (coll, debt) = cdp_vault::coll_debt_of(vault_of(alice));
        assert!(coll == ONBOARD_COLL && debt == ONBOARD_DEBT, 0);

        // Every branch keeps the length, at every price the band reaches.
        let len = cdp_sorted::len();
        let (lots, _) = cdp_auction::auction_state();
        let i = 0;
        while (i < BAND_STOPS) {
            cdp_oracle::set_price(admin, band_stop(i));
            run_mix(alice, true, hint(0));
            run_mix(bob, false, hint(2));
            assert!(cdp_sorted::len() == len, 0);
            i = i + 1;
        };

        // The bottom of the band is below the ratio the list is seeded at, so
        // the sweep finds candidates somewhere in it.
        let (_, absorbed) = cdp_stability::pool_state();
        assert!(absorbed > 0, 0);
        let (lots_after, proceeds) = cdp_auction::auction_state();
        assert!(lots_after == lots && proceeds > 0, 0);
        let (staked, _) = cdp_stability::deposit_state(
            signer::address_of(alice));
        assert!(staked == DEPOSIT_AMOUNT * BAND_STOPS, 0);
        assert!(cdp_vault::exists_vault(vault_of(alice)), 0);
        assert!(cdp_sorted::in_list(vault_of(bob)), 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_liquidate_tolerates_empty_candidates(
        admin: &signer, alice: &signer
    ) {
        init_package(admin);
        cdp_sorted::bench_onboard(
            alice, ONBOARD_COLL, ONBOARD_ICR_BPS, hint(0));
        // Every seeded vault sits above the minimum at the start price, so
        // the sweep finds nothing and has to leave the list alone.
        let len = cdp_sorted::len();
        let (staked, absorbed) = cdp_stability::pool_state();
        cdp_stability::bench_liquidate(alice, LIQUIDATIONS_PER_TXN, WALK_LIMIT);
        let (staked_after, absorbed_after) = cdp_stability::pool_state();
        assert!(cdp_sorted::len() == len, 0);
        assert!(staked_after == staked && absorbed_after == absorbed, 0);

        // Halving the price makes every one of them a candidate, and the
        // sweep still leaves the length alone.
        cdp_oracle::set_price(admin, START_PRICE / 2);
        cdp_stability::bench_liquidate(alice, LIQUIDATIONS_PER_TXN, WALK_LIMIT);
        let (staked_swept, absorbed_swept) = cdp_stability::pool_state();
        assert!(cdp_sorted::len() == len, 0);
        assert!(absorbed_swept > absorbed && staked_swept < staked, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_candidate_pool_renews(
        admin: &signer, alice: &signer, bob: &signer
    ) {
        init_package(admin);
        cdp_sorted::bench_onboard(
            alice, ONBOARD_COLL, ONBOARD_ICR_BPS, hint(0));
        cdp_sorted::bench_onboard(bob, ONBOARD_COLL, ONBOARD_ICR_BPS, hint(2));
        let mcr = cdp_vault::mcr_bps();
        let lo = cdp_oracle::price_min();
        let hi = cdp_oracle::price_max();

        // A liquidated vault reopens against the reference price, not the
        // price it happened to be liquidated at, so the band can always take
        // it back below the minimum. Both ends of that have to hold: safe at
        // the top of the band, a candidate again at the bottom. If either
        // goes, the sweep retires the list one vault at a time and the
        // liquidation branch stops doing any work at all.
        cdp_oracle::set_price(admin, lo);
        let victim = cdp_sorted::head();
        cdp_stability::bench_liquidate(alice, 1, WALK_LIMIT);
        assert!(cdp_vault::icr_of(victim, hi) >= mcr, 0);
        assert!(cdp_vault::icr_of(victim, lo) < mcr, 0);

        // The same over a run. The pool has to absorb as much in the second
        // half of a repeated band sweep as in the first, which it can only do
        // if the vaults the earlier sweeps reopened have come back.
        let (_, base) = cdp_stability::pool_state();
        let first = 0;
        let r = 0;
        while (r < RENEWAL_ROUNDS) {
            cdp_oracle::set_price(admin, band_stop(r % BAND_STOPS));
            cdp_sorted::bench_adjust(
                alice, ADJUST_COLL, ADJUST_DEBT, true, hint(0));
            cdp_sorted::bench_adjust(
                bob, ADJUST_COLL, ADJUST_DEBT, false, hint(2));
            cdp_stability::bench_liquidate(
                alice, LIQUIDATIONS_PER_TXN, WALK_LIMIT);
            r = r + 1;
            if (r == RENEWAL_ROUNDS / 2) {
                let (_, absorbed) = cdp_stability::pool_state();
                first = absorbed - base;
            }
        };
        let (_, total) = cdp_stability::pool_state();
        let second = total - base - first;
        assert!(first > 0 && second > 0, 0);
        assert!(second * 2 >= first && first * 2 >= second, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_walk_tolerates_limit_past_length(
        admin: &signer, alice: &signer
    ) {
        init_package(admin);
        let len = cdp_sorted::len();
        // A limit past the end of the list folds the whole list and stops.
        let whole = cdp_sorted::walk_checksum(cdp_sorted::head(), len);
        assert!(cdp_sorted::walk_checksum(@0x0, len * 1000) == whole, 0);
        assert!(whole != 0, 0);
        cdp_sorted::bench_walk(alice, @0x0, 18446744073709551615);
        assert!(cdp_sorted::len() == len, 0);

        // A walk from a node that holds no vault at all starts at the head.
        assert!(cdp_sorted::walk_checksum(@0xdead, len) == whole, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_insert_tolerates_stale_hint(
        admin: &signer, alice: &signer, bob: &signer
    ) {
        init_package(admin);
        let len = cdp_sorted::len();
        // Detach the node the hint names, which is what happens when another
        // transaction moves that vault between generation and execution.
        cdp_sorted::remove(hint(1));
        assert!(!cdp_sorted::in_list(hint(1)), 0);
        cdp_sorted::bench_onboard(
            alice, ONBOARD_COLL, ONBOARD_ICR_BPS, hint(1));
        assert!(cdp_sorted::in_list(vault_of(alice)), 0);
        assert!(cdp_sorted::len() == len, 0);

        // A hint that was never a vault is the same fallback.
        cdp_sorted::bench_onboard(bob, ONBOARD_COLL, ONBOARD_ICR_BPS, @0xdead);
        assert!(cdp_sorted::in_list(vault_of(bob)), 0);
        assert!(cdp_sorted::len() == len + 1, 0);

        // And so is a hint naming the vault being placed.
        cdp_sorted::bench_adjust(
            alice, ADJUST_COLL, ADJUST_DEBT, true, vault_of(alice));
        assert!(cdp_sorted::in_list(vault_of(alice)), 0);
        assert!(cdp_sorted::len() == len + 1, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_adjust_tolerates_debt_below_mcr(
        admin: &signer, alice: &signer
    ) {
        init_package(admin);
        cdp_sorted::bench_onboard(
            alice, ONBOARD_COLL, ONBOARD_ICR_BPS, hint(0));
        let vault = vault_of(alice);
        let len = cdp_sorted::len();

        // A debt increase that would drop the vault under the minimum lands
        // on the boundary instead.
        cdp_sorted::bench_adjust(alice, 0, 1000000, true, hint(0));
        let (_, debt) = cdp_vault::coll_debt_of(vault);
        assert!(debt < ONBOARD_DEBT + 1000000, 0);
        assert!(cdp_vault::icr_of(vault, START_PRICE) >= MCR_BPS, 0);

        // So does a collateral withdrawal that would do the same, and the
        // collateral floor keeps the ratio well defined.
        cdp_sorted::bench_adjust(alice, 1000000, 0, false, hint(0));
        let (coll, _) = cdp_vault::coll_debt_of(vault);
        assert!(coll > 0, 0);
        assert!(cdp_vault::icr_of(vault, START_PRICE) >= MCR_BPS, 0);
        assert!(cdp_sorted::len() == len, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_close_reopen_tolerates_repeat(
        admin: &signer, alice: &signer, bob: &signer
    ) {
        init_package(admin);
        cdp_sorted::bench_onboard(
            alice, ONBOARD_COLL, ONBOARD_ICR_BPS, hint(0));
        let vault = vault_of(alice);
        let len = cdp_sorted::len();
        let (coll, debt) = cdp_vault::coll_debt_of(vault);

        // Reopening goes through the extend ref the close left behind, so it
        // can run any number of times without colliding with the original
        // named object.
        cdp_sorted::bench_close_reopen(alice, hint(0));
        cdp_sorted::bench_close_reopen(alice, hint(0));
        cdp_sorted::bench_close_reopen(alice, hint(0));
        let (coll_after, debt_after) = cdp_vault::coll_debt_of(vault);
        assert!(coll_after == coll && debt_after == debt, 0);
        assert!(cdp_sorted::in_list(vault), 0);
        assert!(cdp_sorted::len() == len, 0);

        // An account that never onboarded gets a vault here rather than a
        // silent no-op.
        cdp_sorted::bench_close_reopen(bob, hint(0));
        assert!(cdp_sorted::in_list(vault_of(bob)), 0);
        assert!(cdp_sorted::len() == len + 1, 0);
    }

    #[test(admin = @bench)]
    fun test_sorted_insert_remove_ordering(admin: &signer) {
        cdp_vault::initialize(admin, MCR_BPS);
        cdp_oracle::set_price(admin, START_PRICE);
        // An empty batch creates the list and nothing else.
        cdp_sorted::seed_vaults(admin, 0, 0);
        assert!(cdp_sorted::len() == 0, 0);

        // A debt of exactly the ratio scale makes the nominal ratio equal to
        // the collateral, so the expected order is readable.
        let scale_debt = 1000000000;
        let colls = vector[50u64, 10, 30, 40, 20];
        let i = 0;
        while (i < 5) {
            let addr = cdp_vault::create(
                admin, 100 + i, *std::vector::borrow(&colls, i), scale_debt);
            cdp_sorted::insert(addr, cdp_sorted::tail(), UNCAPPED);
            i = i + 1;
        };
        assert!(cdp_sorted::len() == 5, 0);
        assert!(colls_in_order() == vector[10u64, 20, 30, 40, 50], 0);
        assert!(cdp_vault::nicr_of(cdp_sorted::head()) == 10, 0);
        assert!(cdp_vault::nicr_of(cdp_sorted::tail()) == 50, 0);

        // Dropping an interior node stitches its neighbours together.
        cdp_sorted::remove(cdp_vault::vault_address(@bench, 102));
        assert!(colls_in_order() == vector[10u64, 20, 40, 50], 0);
        assert!(cdp_sorted::len() == 4, 0);

        // The head and the tail move when their node goes.
        cdp_sorted::remove(cdp_sorted::head());
        cdp_sorted::remove(cdp_sorted::tail());
        assert!(colls_in_order() == vector[20u64, 40], 0);
        assert!(cdp_sorted::len() == 2, 0);

        // Putting the interior node back finds its place again.
        cdp_sorted::insert(
            cdp_vault::vault_address(@bench, 102), cdp_sorted::head(), UNCAPPED);
        assert!(colls_in_order() == vector[20u64, 30, 40], 0);
        assert!(cdp_sorted::len() == 3, 0);

        // Removing every node empties the list rather than corrupting it.
        cdp_sorted::remove(cdp_sorted::head());
        cdp_sorted::remove(cdp_sorted::head());
        cdp_sorted::remove(cdp_sorted::head());
        assert!(cdp_sorted::len() == 0, 0);
        assert!(cdp_sorted::head() == @0x0 && cdp_sorted::tail() == @0x0, 0);

        // Removing what is not linked is a no-op, not an underflow.
        cdp_sorted::remove(cdp_vault::vault_address(@bench, 102));
        assert!(cdp_sorted::len() == 0, 0);
    }

    #[test]
    fun test_icr_math() {
        let max = cdp_vault::max_ratio();
        // Collateral per unit of debt, scaled, and independent of price.
        assert!(cdp_vault::nicr(100, 1000000000) == 100, 0);
        assert!(cdp_vault::nicr(100, 1000) == 100000000, 0);
        // No debt is healthier and sorts later than any debt.
        assert!(cdp_vault::nicr(100, 0) == max, 0);
        assert!(cdp_vault::nicr(0, 1000) == 0, 0);

        // 150 collateral at 2000 against 120000 debt is a 250% ratio.
        assert!(cdp_vault::icr_bps(150, 120000, 2000) == 25000, 0);
        // And the same position halves when the price does.
        assert!(cdp_vault::icr_bps(150, 120000, 1000) == 12500, 0);
        assert!(cdp_vault::icr_bps(150, 0, 2000) == max, 0);

        // The debt ceiling is the inverse: it is exactly what puts the ratio
        // on the target.
        assert!(cdp_vault::max_debt_at(150, 2000, 25000) == 120000, 0);
        assert!(
            cdp_vault::icr_bps(
                150, cdp_vault::max_debt_at(150, 2000, MCR_BPS), 2000)
                >= MCR_BPS,
            0,
        );
        assert!(cdp_vault::max_debt_at(150, 2000, 0) == 0, 0);

        // Collateral is clamped before it enters any product, so the ratios
        // cannot overflow on an oversized position.
        let over = cdp_vault::max_coll() + 1;
        assert!(cdp_vault::clamp_coll(over) == cdp_vault::max_coll(), 0);
        assert!(
            cdp_vault::nicr(over, 1000000000)
                == cdp_vault::nicr(cdp_vault::max_coll(), 1000000000),
            0,
        );
    }
}
