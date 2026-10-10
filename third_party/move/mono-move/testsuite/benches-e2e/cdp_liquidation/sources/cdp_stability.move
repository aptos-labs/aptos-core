/// Stability pool and the liquidation sweep that draws on it, following
/// Liquity's offset mechanism.
///
/// Depositors put stable into the pool and are paid in the collateral of the
/// vaults the pool absorbs. A deposit tracks its share with a snapshot of two
/// running quantities: the gain per unit staked, which pays it, and the
/// product, which shrinks it by whatever share of the pool the last
/// cancellation spent. Absorbing a vault therefore costs one write rather than
/// one per depositor, and what the depositors hold still adds up to what the
/// pool holds.
///
/// A liquidated vault is reopened at a ratio measured against the oracle's
/// reference price instead of being deleted. The list therefore keeps a
/// constant length, an empty candidate set is a no-op, and a reopened vault
/// rejoins the band of ratios the list was seeded at, so it is a candidate
/// again the next time the price sweeps the bottom of its band.
module bench::cdp_stability {
    use std::signer;
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::object::Object;
    use aptos_framework::primary_fungible_store;
    use bench::cdp_assets;
    use bench::cdp_oracle;
    use bench::cdp_sorted;
    use bench::cdp_vault;

    /// Only the package address funds the pool.
    const E_NOT_BENCH: u64 = 1;

    const STABLE_SYMBOL: vector<u8> = b"CDPS";

    /// Fixed point scale of the two running quantities a deposit snapshots.
    const SCALE: u128 = 1000000000000;

    /// Product floor. A long run of absorbs drives the product towards zero,
    /// and below this the ratio against a snapshot has lost too much precision
    /// to pay anyone accurately. The pool then starts a new epoch, which is
    /// what Liquity does when a cancellation empties it.
    const PRODUCT_FLOOR: u128 = 1000000;

    /// Ratios a liquidated vault reopens at, in basis points against the
    /// oracle's reference price. Measuring against the reference rather than
    /// the spot price is what keeps the branch alive. A sweep only fires
    /// while the price is low, so a target read off the spot price pins the
    /// vault to a ratio the band can never take back below the minimum, and
    /// the candidate pool then retires one vault at a time. Against the
    /// reference the band is the same one the list is seeded at: safe at the
    /// top of it, a candidate again at the bottom. The spread is taken from
    /// the vault's collateral, so reopened vaults do not all collapse onto a
    /// single nominal ratio.
    const REOPEN_ICR_MIN_BPS: u64 = 11600;
    const REOPEN_ICR_STEP_BPS: u64 = 5;
    const REOPEN_ICR_SPREAD: u64 = 97;

    /// Nodes a reinsert may walk. A reopened vault is healthier than the head
    /// of the list, so it starts from the tail hint and walks back a few.
    const REINSERT_WALK_CAP: u64 = 32;

    struct Pool has key {
        /// Stable staked and not yet used to cancel debt.
        total: u64,
        /// Collateral seized from liquidated vaults, cumulative.
        coll_absorbed: u64,
        /// Running collateral gain per unit staked, scaled by `product`.
        gain_per_unit: u128,
        /// Running product every stake is compounded by, scaled by `SCALE`.
        /// Cancelling `c` of `s` staked multiplies it by `(s - c) / s`, which
        /// is how one write takes the spent stake off every deposit at once.
        product: u128,
        /// Bumped whenever the product is reset.
        epoch: u64,
    }

    struct Deposit has key {
        /// Stake as of the snapshots below, before any later absorb.
        amount: u64,
        /// Value of `gain_per_unit` when this deposit was last resized.
        snapshot: u128,
        /// Value of `product` when this deposit was last resized.
        snapshot_product: u128,
        /// Value of the pool's epoch when this deposit was last resized.
        epoch: u64,
        coll_gain: u64,
    }

    /// Admin-only. Creates the pool on the first call, which is how the
    /// publisher's setup sequence initializes it.
    public entry fun fund(admin: &signer, amount: u64) acquires Pool {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        if (!exists<Pool>(@bench)) {
            move_to(
                admin,
                Pool {
                    total: 0,
                    coll_absorbed: 0,
                    gain_per_unit: 0,
                    product: SCALE,
                    epoch: 0,
                },
            );
        };
        cdp_assets::faucet(stable_asset(), @bench, amount);
        let pool = borrow_global_mut<Pool>(@bench);
        pool.total = pool.total + amount;
    }

    /// Stake more stable. The stake is faucet-backed, so a depositor never
    /// runs short, and a first deposit creates the depositor's record.
    public entry fun bench_deposit(
        user: &signer, amount: u64
    ) acquires Deposit, Pool {
        if (!exists<Pool>(@bench)) return;
        let owner = signer::address_of(user);
        let stable = stable_asset();
        cdp_assets::faucet(stable, owner, amount);
        primary_fungible_store::transfer(user, stable, @bench, amount);
        let pool = borrow_global<Pool>(@bench);
        let gain_per_unit = pool.gain_per_unit;
        let product = pool.product;
        let epoch = pool.epoch;
        if (!exists<Deposit>(owner)) {
            move_to(
                user,
                Deposit {
                    amount: 0,
                    snapshot: gain_per_unit,
                    snapshot_product: product,
                    epoch,
                    coll_gain: 0,
                },
            );
        };
        let deposit = borrow_global_mut<Deposit>(owner);
        // Take the absorbs since the last resize off the old stake, and settle
        // what that stake earned, before the new one changes the weighting.
        let (staked, gain) = compounded(deposit, gain_per_unit, product, epoch);
        deposit.coll_gain = deposit.coll_gain + gain;
        deposit.amount = staked + amount;
        deposit.snapshot = gain_per_unit;
        deposit.snapshot_product = product;
        deposit.epoch = epoch;
        let pool = borrow_global_mut<Pool>(@bench);
        pool.total = pool.total + amount;
    }

    /// `(stake, gain)` a deposit holds against the pool's current running
    /// quantities: what is left of its stake after every absorb since it was
    /// last resized, and the collateral those absorbs owe it. An absorb that
    /// closed the epoch spent the stake, so both come back zero.
    fun compounded(
        deposit: &Deposit, gain_per_unit: u128, product: u128, epoch: u64
    ): (u64, u64) {
        if (deposit.epoch != epoch || deposit.snapshot_product == 0) {
            return (0, 0)
        };
        let amount = (deposit.amount as u128);
        let stake = amount * product / deposit.snapshot_product;
        let gain = amount * (gain_per_unit - deposit.snapshot)
            / deposit.snapshot_product;
        ((stake as u64), (gain as u64))
    }

    /// Sweep the riskiest end of the list. Walks at most `walk_limit` nodes,
    /// clamped to the list length, and liquidates at most `n` of them.
    public entry fun bench_liquidate(
        _user: &signer, n: u64, walk_limit: u64
    ) acquires Pool {
        if (!exists<Pool>(@bench)) return;
        let price = cdp_oracle::price();
        let mcr = cdp_vault::mcr_bps();
        let len = cdp_sorted::len();
        let cap = if (walk_limit > len) len else walk_limit;
        let cur = cdp_sorted::head();
        let steps = 0;
        let done = 0;
        while (cur != @0x0 && steps < cap && done < n) {
            // Read the successor before the node moves, so the sweep keeps
            // its place whatever the reinsert does with the current vault.
            let next = cdp_vault::next_of(cur);
            let (coll, debt) = cdp_vault::coll_debt_of(cur);
            if (cdp_vault::icr_bps(coll, debt, price) < mcr) {
                // The pool seizes the whole position. The vault reopens on a
                // freshly posted position of the same size rather than being
                // deleted, so the list keeps a constant length.
                absorb(coll, debt);
                cdp_sorted::remove(cur);
                cdp_vault::set_coll_debt(
                    cur,
                    coll,
                    cdp_vault::max_debt_at(
                        coll,
                        cdp_oracle::reference_price(),
                        reopen_icr_bps(coll),
                    ),
                );
                cdp_sorted::insert(
                    cur, cdp_sorted::tail(), REINSERT_WALK_CAP);
                done = done + 1;
            };
            cur = next;
            steps = steps + 1;
        }
    }

    /// Cancel `debt` against the pool and credit `coll` to its depositors.
    /// The pool cancels only what it actually holds, so a drained pool turns
    /// a liquidation into bookkeeping instead of an abort.
    fun absorb(coll: u64, debt: u64) acquires Pool {
        let stable = stable_asset();
        let held = cdp_assets::primary_balance(@bench, stable);
        let pool = borrow_global_mut<Pool>(@bench);
        let staked = pool.total;
        let cancelled = if (debt > staked) staked else debt;
        let cancelled = if (cancelled > held) held else cancelled;
        if (staked > 0) {
            let product = pool.product;
            pool.gain_per_unit = pool.gain_per_unit
                + ((coll as u128) * product / (staked as u128));
            // The cancellation spent the same share of every stake, so decay
            // the product they are all measured against. Spending the pool
            // outright, or wearing the product down to its floor, closes the
            // epoch instead: the stakes recorded under it are gone.
            let left = product * ((staked - cancelled) as u128)
                / (staked as u128);
            if (left < PRODUCT_FLOOR) {
                pool.product = SCALE;
                pool.epoch = pool.epoch + 1;
            } else {
                pool.product = left;
            };
        };
        pool.total = staked - cancelled;
        pool.coll_absorbed = pool.coll_absorbed + coll;
        if (cancelled > 0) {
            cdp_assets::burn_from(stable, @bench, cancelled);
        }
    }

    /// Ratio a vault holding `coll` reopens at, in basis points against the
    /// oracle's reference price.
    public fun reopen_icr_bps(coll: u64): u64 {
        REOPEN_ICR_MIN_BPS + (coll % REOPEN_ICR_SPREAD) * REOPEN_ICR_STEP_BPS
    }

    fun stable_asset(): Object<Metadata> {
        cdp_assets::asset(@bench, STABLE_SYMBOL)
    }

    #[view]
    /// Staked stable, absorbed collateral.
    public fun pool_state(): (u64, u64) acquires Pool {
        if (!exists<Pool>(@bench)) return (0, 0);
        let pool = borrow_global<Pool>(@bench);
        (pool.total, pool.coll_absorbed)
    }

    #[view]
    /// Staked stable and collateral gain of one depositor, both taken up to
    /// the absorbs that have happened since its last deposit.
    public fun deposit_state(owner: address): (u64, u64) acquires Deposit, Pool {
        if (!exists<Deposit>(owner) || !exists<Pool>(@bench)) return (0, 0);
        let pool = borrow_global<Pool>(@bench);
        let deposit = borrow_global<Deposit>(owner);
        let (staked, gain) = compounded(
            deposit, pool.gain_per_unit, pool.product, pool.epoch);
        (staked, deposit.coll_gain + gain)
    }
}
