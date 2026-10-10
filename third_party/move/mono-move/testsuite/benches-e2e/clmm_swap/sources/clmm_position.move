/// Liquidity owned by an address over a tick range, and the fees it has
/// earned.
///
/// A position stores the fee growth it last settled against. Fees owed are the
/// difference against the current growth inside its range, so a position that
/// is never touched costs the pool nothing.
module bench::clmm_position {
    use aptos_std::table::{Self, Table};

    /// Burning more liquidity than the position holds.
    const ELIQUIDITY_UNDERFLOW: u64 = 1;
    /// Poking a position that holds nothing and is being given nothing.
    const EEMPTY_POSITION: u64 = 2;
    /// Position does not exist.
    const EUNKNOWN_POSITION: u64 = 3;

    const MAX_U128: u128 = 340282366920938463463374607431768211455;
    const MAX_U64: u128 = 18446744073709551615;

    /// Fee growth per unit of liquidity is Q64.64.
    const FEE_GROWTH_SHIFT: u8 = 64;

    struct PositionKey has copy, drop, store {
        owner: address,
        tick_lower: i32,
        tick_upper: i32,
    }

    struct PositionInfo has copy, drop, store {
        liquidity: u128,
        fee_growth_inside_a_last: u128,
        fee_growth_inside_b_last: u128,
        tokens_owed_a: u64,
        tokens_owed_b: u64,
    }

    struct Positions has store {
        map: Table<PositionKey, PositionInfo>,
    }

    public fun new(): Positions {
        Positions { map: table::new() }
    }

    public fun key(owner: address, tick_lower: i32, tick_upper: i32): PositionKey {
        PositionKey { owner, tick_lower, tick_upper }
    }

    fun wrapping_sub(a: u128, b: u128): u128 {
        if (a >= b) {
            a - b
        } else {
            (MAX_U128 - b) + a + 1
        }
    }

    fun empty_info(): PositionInfo {
        PositionInfo {
            liquidity: 0,
            fee_growth_inside_a_last: 0,
            fee_growth_inside_b_last: 0,
            tokens_owed_a: 0,
            tokens_owed_b: 0,
        }
    }

    public fun info_at(self: &Positions, key: PositionKey): PositionInfo {
        if (table::contains(&self.map, key)) {
            *table::borrow(&self.map, key)
        } else {
            empty_info()
        }
    }

    public fun exists_at(self: &Positions, key: PositionKey): bool {
        table::contains(&self.map, key)
    }

    public fun liquidity(self: &Positions, key: PositionKey): u128 {
        info_at(self, key).liquidity
    }

    public fun tokens_owed(self: &Positions, key: PositionKey): (u64, u64) {
        let info = info_at(self, key);
        (info.tokens_owed_a, info.tokens_owed_b)
    }

    public fun fee_growth_inside_last(self: &Positions, key: PositionKey): (u128, u128) {
        let info = info_at(self, key);
        (info.fee_growth_inside_a_last, info.fee_growth_inside_b_last)
    }

    /// Settle fees earned since the last update, then apply a liquidity change.
    public fun update(
        self: &mut Positions,
        key: PositionKey,
        liquidity_delta: i128,
        fee_growth_inside_a: u128,
        fee_growth_inside_b: u128
    ) {
        let info = info_at(self, key);
        let liquidity_after =
            if (liquidity_delta == 0i128) {
                assert!(info.liquidity > 0, EEMPTY_POSITION);
                info.liquidity
            } else if (liquidity_delta < 0i128) {
                let magnitude = (-liquidity_delta) as u128;
                assert!(info.liquidity >= magnitude, ELIQUIDITY_UNDERFLOW);
                info.liquidity - magnitude
            } else {
                info.liquidity + (liquidity_delta as u128)
            };

        // Settle against the liquidity that was in place while the fees
        // accrued, not the amount after this change.
        let owed_a = fees_earned(
            info.liquidity, fee_growth_inside_a, info.fee_growth_inside_a_last
        );
        let owed_b = fees_earned(
            info.liquidity, fee_growth_inside_b, info.fee_growth_inside_b_last
        );

        info.liquidity = liquidity_after;
        info.fee_growth_inside_a_last = fee_growth_inside_a;
        info.fee_growth_inside_b_last = fee_growth_inside_b;
        // Uncollected fees saturate rather than abort, so a position left
        // alone for a long time stays usable.
        info.tokens_owed_a = saturating_add(info.tokens_owed_a, owed_a);
        info.tokens_owed_b = saturating_add(info.tokens_owed_b, owed_b);
        table::upsert(&mut self.map, key, info);
    }

    fun fees_earned(liquidity: u128, growth_now: u128, growth_last: u128): u64 {
        if (liquidity == 0) {
            return 0
        };
        let growth = (wrapping_sub(growth_now, growth_last) as u256);
        let earned = ((liquidity as u256) * growth) >> FEE_GROWTH_SHIFT;
        if (earned > (MAX_U64 as u256)) {
            (MAX_U64 as u64)
        } else {
            (earned as u64)
        }
    }

    fun saturating_add(a: u64, b: u64): u64 {
        let sum = (a as u128) + (b as u128);
        if (sum > MAX_U64) { (MAX_U64 as u64) } else { (sum as u64) }
    }

    /// Add to what a position may withdraw, for principal returned by a burn.
    public fun credit(
        self: &mut Positions, key: PositionKey, amount_a: u64, amount_b: u64
    ) {
        assert!(table::contains(&self.map, key), EUNKNOWN_POSITION);
        let info = table::borrow_mut(&mut self.map, key);
        info.tokens_owed_a = saturating_add(info.tokens_owed_a, amount_a);
        info.tokens_owed_b = saturating_add(info.tokens_owed_b, amount_b);
    }

    /// Take up to `requested_a` / `requested_b` out of what is owed, returning
    /// what was actually taken.
    public fun collect(
        self: &mut Positions, key: PositionKey, requested_a: u64, requested_b: u64
    ): (u64, u64) {
        assert!(table::contains(&self.map, key), EUNKNOWN_POSITION);
        let info = table::borrow_mut(&mut self.map, key);
        let taken_a = if (requested_a > info.tokens_owed_a) { info.tokens_owed_a }
            else { requested_a };
        let taken_b = if (requested_b > info.tokens_owed_b) { info.tokens_owed_b }
            else { requested_b };
        info.tokens_owed_a = info.tokens_owed_a - taken_a;
        info.tokens_owed_b = info.tokens_owed_b - taken_b;
        (taken_a, taken_b)
    }

    /// Convert a fee amount into growth per unit of liquidity, in Q64.64.
    public fun fee_growth_delta(fee: u64, liquidity: u128): u128 {
        if (liquidity == 0 || fee == 0) {
            return 0
        };
        ((((fee as u256) << FEE_GROWTH_SHIFT) / (liquidity as u256)) as u128)
    }

    //
    // Tests.
    //

    #[test_only]
    fun destroy(self: Positions) {
        let Positions { map } = self;
        table::drop_unchecked(map);
    }

    #[test_only]
    const OWNER: address = @0xB0;

    #[test]
    fun test_update_creates_and_grows_a_position() {
        let positions = new();
        let k = key(OWNER, -60, 60);
        assert!(!exists_at(&positions, k), 0);

        update(&mut positions, k, 1000i128, 0, 0);
        assert!(exists_at(&positions, k), 0);
        assert!(liquidity(&positions, k) == 1000, 0);

        update(&mut positions, k, 500i128, 0, 0);
        assert!(liquidity(&positions, k) == 1500, 0);

        update(&mut positions, k, -1500i128, 0, 0);
        assert!(liquidity(&positions, k) == 0, 0);
        destroy(positions);
    }

    #[test]
    #[expected_failure(abort_code = ELIQUIDITY_UNDERFLOW, location = Self)]
    fun test_update_rejects_burning_too_much() {
        let positions = new();
        let k = key(OWNER, -60, 60);
        update(&mut positions, k, 1000i128, 0, 0);
        update(&mut positions, k, -1001i128, 0, 0);
        destroy(positions);
    }

    #[test]
    #[expected_failure(abort_code = EEMPTY_POSITION, location = Self)]
    fun test_poking_an_empty_position_fails() {
        let positions = new();
        update(&mut positions, key(OWNER, -60, 60), 0i128, 0, 0);
        destroy(positions);
    }

    // Fees accrue in proportion to liquidity, and only for growth that
    // happened after the position was opened.
    #[test]
    fun test_fees_accrue_on_growth_since_the_last_update() {
        let positions = new();
        let k = key(OWNER, -60, 60);
        // Opened when growth inside was already 1 << 64.
        update(&mut positions, k, 1000i128, 1u128 << 64, 0);
        let (a, b) = tokens_owed(&positions, k);
        assert!(a == 0 && b == 0, 0);

        // Another 1 << 64 of growth: one unit of fee per unit of liquidity.
        update(&mut positions, k, 0i128, 2u128 << 64, 0);
        let (a, b) = tokens_owed(&positions, k);
        assert!(a == 1000 && b == 0, 0);

        // Poking again with no further growth adds nothing.
        update(&mut positions, k, 0i128, 2u128 << 64, 0);
        let (a, _) = tokens_owed(&positions, k);
        assert!(a == 1000, 0);
        destroy(positions);
    }

    // Fees settle against the liquidity in place while they accrued, so
    // burning first does not forfeit them.
    #[test]
    fun test_fees_settle_before_the_liquidity_change() {
        let positions = new();
        let k = key(OWNER, -60, 60);
        update(&mut positions, k, 1000i128, 0, 0);
        update(&mut positions, k, -1000i128, 1u128 << 64, 1u128 << 64);
        let (a, b) = tokens_owed(&positions, k);
        assert!(a == 1000 && b == 1000, 0);
        assert!(liquidity(&positions, k) == 0, 0);
        destroy(positions);
    }

    #[test]
    fun test_collect_clamps_to_what_is_owed() {
        let positions = new();
        let k = key(OWNER, -60, 60);
        update(&mut positions, k, 1000i128, 0, 0);
        update(&mut positions, k, 0i128, 1u128 << 64, 2u128 << 64);
        let (a, b) = tokens_owed(&positions, k);
        assert!(a == 1000 && b == 2000, 0);

        let (took_a, took_b) = collect(&mut positions, k, 400, 18446744073709551615);
        assert!(took_a == 400 && took_b == 2000, 0);
        let (a, b) = tokens_owed(&positions, k);
        assert!(a == 600 && b == 0, 0);

        let (took_a, took_b) = collect(&mut positions, k, 18446744073709551615, 1);
        assert!(took_a == 600 && took_b == 0, 0);
        destroy(positions);
    }

    #[test]
    #[expected_failure(abort_code = EUNKNOWN_POSITION, location = Self)]
    fun test_collect_rejects_an_unknown_position() {
        let positions = new();
        collect(&mut positions, key(OWNER, -60, 60), 1, 1);
        destroy(positions);
    }

    // Two positions on the same range but different owners are independent.
    #[test]
    fun test_positions_are_keyed_by_owner_and_range() {
        let positions = new();
        let k1 = key(@0xB0, -60, 60);
        let k2 = key(@0xB1, -60, 60);
        let k3 = key(@0xB0, -120, 60);
        update(&mut positions, k1, 100i128, 0, 0);
        update(&mut positions, k2, 200i128, 0, 0);
        update(&mut positions, k3, 300i128, 0, 0);
        assert!(liquidity(&positions, k1) == 100, 0);
        assert!(liquidity(&positions, k2) == 200, 0);
        assert!(liquidity(&positions, k3) == 300, 0);
        destroy(positions);
    }

    // Global growth wraps past the top of `u128`; the difference across the
    // wrap is still the right amount.
    #[test]
    fun test_fees_survive_an_accumulator_wrap() {
        let positions = new();
        let k = key(OWNER, -60, 60);
        let near_top = MAX_U128 - (1u128 << 63);
        update(&mut positions, k, 1000i128, near_top, 0);
        // Growth of exactly 1 << 64 past `near_top` lands here once it has
        // wrapped, so the settled difference has to come back around.
        update(&mut positions, k, 0i128, (1u128 << 63) - 1, 0);
        let (a, _) = tokens_owed(&positions, k);
        assert!(a == 1000, 0);
        destroy(positions);
    }

    #[test]
    fun test_fee_growth_delta() {
        assert!(fee_growth_delta(0, 1000) == 0, 0);
        assert!(fee_growth_delta(1000, 0) == 0, 0);
        // One fee unit spread over one liquidity unit is exactly 1.0 in Q64.64.
        assert!(fee_growth_delta(1, 1) == (1u128 << 64), 0);
        assert!(fee_growth_delta(1, 2) == (1u128 << 63), 0);
        assert!(fee_growth_delta(3000, 1000) == 3 * (1u128 << 64), 0);
    }

    // The fee a position collects matches the fee the pool booked, up to the
    // rounding the pool keeps.
    #[test]
    fun test_fee_growth_round_trips_through_a_position() {
        let positions = new();
        let k = key(OWNER, -60, 60);
        let l = 1000000u128;
        update(&mut positions, k, (l as i128), 0, 0);
        let growth = fee_growth_delta(12345, l);
        update(&mut positions, k, 0i128, growth, 0);
        let (a, _) = tokens_owed(&positions, k);
        assert!(a <= 12345 && a + 1 >= 12345, 0);
        destroy(positions);
    }
}
