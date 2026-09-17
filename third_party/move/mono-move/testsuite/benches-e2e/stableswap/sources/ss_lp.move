/// Liquidity flows that solve the invariant more than once: an imbalanced
/// deposit and a single-coin withdrawal, both shaped after Curve's
/// `add_liquidity` and `remove_liquidity_one_coin`.
///
/// A balanced deposit is cheap because the pool's shape does not move. These
/// two shift it, so the second solve starts from a different place than the
/// first and the Newton trip counts diverge.
///
/// Both hold the pool's own composition. A flow that deposited evenly would
/// pull a lopsided pool back to balance over a long run, and a balanced pool
/// solves the invariant in a single round.
module bench::ss_lp {
    use std::signer;
    use std::vector;
    use bench::ss_pool;

    const BPS: u64 = 10000;

    /// Skew ceiling. At the ceiling every coin gets the same amount, which is
    /// a balanced deposit.
    const MAX_IMBALANCE_BP: u64 = 10000;

    /// Largest whole-token deposit one transaction may make.
    const MAX_UNITS: u64 = 1000000000;

    /// Deposit `units` whole tokens of the pool's lead coin and `imbalance_bp`
    /// of that into every other coin. A deposit may be more lopsided than the
    /// pool it lands in but never more balanced, so the mix's own traffic
    /// cannot average a lopsided pool back to balance and cheap solves.
    public entry fun bench_add_imbalanced(
        user: &signer, pool_id: u64, units: u64, imbalance_bp: u64
    ) {
        let n = ss_pool::num_coins(pool_id);
        if (n == 0) {
            return
        };
        let skew = if (imbalance_bp > MAX_IMBALANCE_BP) {
            MAX_IMBALANCE_BP
        } else {
            imbalance_bp
        };
        let target = ss_pool::deposit_skew_bp(pool_id);
        if (skew > target) {
            skew = target;
        };
        let capped = if (units > MAX_UNITS) { MAX_UNITS } else { units };
        let amounts = vector::empty<u64>();
        let k = 0;
        while (k < n) {
            let share = if (k == 0) { capped } else { capped * skew / BPS };
            vector::push_back(
                &mut amounts, ss_pool::raw_amount(pool_id, k, share));
            k = k + 1;
        };
        ss_pool::add_liquidity(signer::address_of(user), pool_id, amounts);
    }

    /// Burn `lp_bps` basis points of the caller's LP into coin `j`. A share
    /// rather than an absolute amount, so a caller holding nothing is a no-op
    /// and one asking for everything gets exactly what it holds.
    public entry fun bench_remove_one(
        user: &signer, pool_id: u64, j: u64, lp_bps: u64
    ) {
        ss_pool::remove_liquidity_one_coin(
            signer::address_of(user), pool_id, j, lp_bps);
    }
}
