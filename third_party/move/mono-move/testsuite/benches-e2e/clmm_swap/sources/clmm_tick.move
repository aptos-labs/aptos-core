/// Per-tick liquidity and fee bookkeeping.
///
/// Each initialized tick records how much liquidity starts or ends there and
/// how much fee growth accrued on the far side of it from the current price.
/// Crossing a tick flips that far side, which is what makes fee accounting
/// cost a constant amount per tick instead of per position.
module bench::clmm_tick {
    use aptos_std::table::{Self, Table};
    use bench::clmm_liquidity_math;
    use bench::clmm_tick_math;

    /// Tick would hold more liquidity than the spacing allows.
    const ELIQUIDITY_OVERFLOW: u64 = 1;
    /// Tick spacing of zero would divide by zero.
    const EZERO_TICK_SPACING: u64 = 2;
    /// Crossing a tick that was never initialized.
    const ETICK_NOT_INITIALIZED: u64 = 3;

    const MAX_U128: u128 = 340282366920938463463374607431768211455;

    struct TickInfo has copy, drop, store {
        liquidity_gross: u128,
        liquidity_net: i128,
        fee_growth_outside_a: u128,
        fee_growth_outside_b: u128,
        initialized: bool,
    }

    struct Ticks has store {
        map: Table<i32, TickInfo>,
    }

    public fun new(): Ticks {
        Ticks { map: table::new() }
    }

    public fun empty_info(): TickInfo {
        TickInfo {
            liquidity_gross: 0,
            liquidity_net: 0i128,
            fee_growth_outside_a: 0,
            fee_growth_outside_b: 0,
            initialized: false,
        }
    }

    /// Fee growth accumulators wrap on purpose, so only their differences are
    /// meaningful.
    fun wrapping_sub(a: u128, b: u128): u128 {
        if (a >= b) {
            a - b
        } else {
            (MAX_U128 - b) + a + 1
        }
    }

    public fun info_at(self: &Ticks, tick: i32): TickInfo {
        if (table::contains(&self.map, tick)) {
            *table::borrow(&self.map, tick)
        } else {
            empty_info()
        }
    }

    public fun is_initialized(self: &Ticks, tick: i32): bool {
        table::contains(&self.map, tick) && table::borrow(&self.map, tick).initialized
    }

    public fun liquidity_gross(self: &Ticks, tick: i32): u128 {
        info_at(self, tick).liquidity_gross
    }

    public fun liquidity_net(self: &Ticks, tick: i32): i128 {
        info_at(self, tick).liquidity_net
    }

    public fun fee_growth_outside(self: &Ticks, tick: i32): (u128, u128) {
        let info = info_at(self, tick);
        (info.fee_growth_outside_a, info.fee_growth_outside_b)
    }

    /// Most liquidity a single tick may carry, so the pool total cannot
    /// overflow no matter how the ticks are filled.
    public fun max_liquidity_per_tick(tick_spacing: u32): u128 {
        assert!(tick_spacing != 0, EZERO_TICK_SPACING);
        let spacing = tick_spacing as i32;
        let min_usable = (clmm_tick_math::min_tick() / spacing) * spacing;
        let max_usable = (clmm_tick_math::max_tick() / spacing) * spacing;
        let count = (((max_usable - min_usable) / spacing) + 1) as u128;
        MAX_U128 / count
    }

    /// Record a liquidity change at `tick`, returning whether the tick just
    /// became initialized or just emptied out.
    public fun update(
        self: &mut Ticks,
        tick: i32,
        current_tick: i32,
        liquidity_delta: i128,
        fee_growth_global_a: u128,
        fee_growth_global_b: u128,
        upper: bool,
        max_liquidity: u128
    ): bool {
        let info = info_at(self, tick);
        let gross_before = info.liquidity_gross;
        let gross_after = clmm_liquidity_math::add_delta(gross_before, liquidity_delta);
        assert!(gross_after <= max_liquidity, ELIQUIDITY_OVERFLOW);
        if (gross_before == 0) {
            // The accumulators are defined to have started entirely below the
            // price, so a tick born under the price inherits all of them.
            if (tick <= current_tick) {
                info.fee_growth_outside_a = fee_growth_global_a;
                info.fee_growth_outside_b = fee_growth_global_b;
            };
            info.initialized = true;
        };
        info.liquidity_gross = gross_after;
        // Crossing upward adds the lower tick's liquidity and removes the
        // upper tick's, so an upper tick carries the negated delta.
        info.liquidity_net = if (upper) {
            info.liquidity_net - liquidity_delta
        } else {
            info.liquidity_net + liquidity_delta
        };
        table::upsert(&mut self.map, tick, info);
        (gross_after == 0) != (gross_before == 0)
    }

    /// Move the price past `tick`, returning the liquidity change to apply.
    public fun cross(
        self: &mut Ticks, tick: i32, fee_growth_global_a: u128, fee_growth_global_b: u128
    ): i128 {
        assert!(table::contains(&self.map, tick), ETICK_NOT_INITIALIZED);
        let info = table::borrow_mut(&mut self.map, tick);
        info.fee_growth_outside_a = wrapping_sub(fee_growth_global_a, info.fee_growth_outside_a);
        info.fee_growth_outside_b = wrapping_sub(fee_growth_global_b, info.fee_growth_outside_b);
        info.liquidity_net
    }

    public fun clear(self: &mut Ticks, tick: i32) {
        if (table::contains(&self.map, tick)) {
            table::remove(&mut self.map, tick);
        };
    }

    /// Fee growth accrued strictly between the two ticks.
    ///
    /// Global growth minus the part below the range minus the part above it.
    public fun get_fee_growth_inside(
        self: &Ticks,
        tick_lower: i32,
        tick_upper: i32,
        current_tick: i32,
        fee_growth_global_a: u128,
        fee_growth_global_b: u128
    ): (u128, u128) {
        let lower = info_at(self, tick_lower);
        let upper = info_at(self, tick_upper);

        let (below_a, below_b) = if (current_tick >= tick_lower) {
            (lower.fee_growth_outside_a, lower.fee_growth_outside_b)
        } else {
            (
                wrapping_sub(fee_growth_global_a, lower.fee_growth_outside_a),
                wrapping_sub(fee_growth_global_b, lower.fee_growth_outside_b)
            )
        };
        let (above_a, above_b) = if (current_tick < tick_upper) {
            (upper.fee_growth_outside_a, upper.fee_growth_outside_b)
        } else {
            (
                wrapping_sub(fee_growth_global_a, upper.fee_growth_outside_a),
                wrapping_sub(fee_growth_global_b, upper.fee_growth_outside_b)
            )
        };
        (
            wrapping_sub(wrapping_sub(fee_growth_global_a, below_a), above_a),
            wrapping_sub(wrapping_sub(fee_growth_global_b, below_b), above_b)
        )
    }

    //
    // Tests.
    //

    #[test_only]
    fun destroy(self: Ticks) {
        let Ticks { map } = self;
        table::drop_unchecked(map);
    }

    #[test]
    fun test_max_liquidity_per_tick() {
        // Spacing 1 makes every tick in range usable, so each may carry the
        // least.
        let one = max_liquidity_per_tick(1);
        let wide = max_liquidity_per_tick(200);
        assert!(one < wide, 0);
        assert!(one == MAX_U128 / 887273, 0);
        // 443636 / 200 truncates to 2218 on both sides, so 4437 usable ticks.
        assert!(wide == MAX_U128 / 4437, 0);
        // Every tick carrying its maximum still fits in a `u128`.
        assert!(one * 887273 <= MAX_U128, 0);
    }

    #[test]
    #[expected_failure(abort_code = EZERO_TICK_SPACING, location = Self)]
    fun test_max_liquidity_per_tick_rejects_zero_spacing() {
        max_liquidity_per_tick(0);
    }

    #[test]
    fun test_update_initializes_and_flips() {
        let ticks = new();
        let max = max_liquidity_per_tick(60);
        assert!(!is_initialized(&ticks, 60), 0);

        let flipped = update(&mut ticks, 60, 0, 1000i128, 0, 0, false, max);
        assert!(flipped, 0);
        assert!(is_initialized(&ticks, 60), 0);
        assert!(liquidity_gross(&ticks, 60) == 1000, 0);
        assert!(liquidity_net(&ticks, 60) == 1000i128, 0);

        // Adding to a live tick does not flip it.
        let flipped = update(&mut ticks, 60, 0, 500i128, 0, 0, false, max);
        assert!(!flipped, 0);
        assert!(liquidity_gross(&ticks, 60) == 1500, 0);

        // Emptying it flips it back.
        let flipped = update(&mut ticks, 60, 0, -1500i128, 0, 0, false, max);
        assert!(flipped, 0);
        assert!(liquidity_gross(&ticks, 60) == 0, 0);
        assert!(liquidity_net(&ticks, 60) == 0i128, 0);
        destroy(ticks);
    }

    #[test]
    fun test_upper_tick_carries_the_negated_delta() {
        let ticks = new();
        let max = max_liquidity_per_tick(60);
        update(&mut ticks, 60, 0, 1000i128, 0, 0, true, max);
        assert!(liquidity_gross(&ticks, 60) == 1000, 0);
        assert!(liquidity_net(&ticks, 60) == -1000i128, 0);
        destroy(ticks);
    }

    // A tick born below the price takes the accumulators with it; one born
    // above starts at zero.
    #[test]
    fun test_first_update_seeds_the_outside_accumulators() {
        let ticks = new();
        let max = max_liquidity_per_tick(60);
        update(&mut ticks, -60, 0, 1000i128, 777, 888, false, max);
        let (a, b) = fee_growth_outside(&ticks, -60);
        assert!(a == 777 && b == 888, 0);

        update(&mut ticks, 60, 0, 1000i128, 777, 888, true, max);
        let (a, b) = fee_growth_outside(&ticks, 60);
        assert!(a == 0 && b == 0, 0);

        // A tick sitting exactly on the current tick counts as below.
        update(&mut ticks, 0, 0, 1000i128, 777, 888, false, max);
        let (a, b) = fee_growth_outside(&ticks, 0);
        assert!(a == 777 && b == 888, 0);
        destroy(ticks);
    }

    // Only the first update seeds; later ones leave the accumulators alone.
    #[test]
    fun test_later_updates_leave_the_accumulators_alone() {
        let ticks = new();
        let max = max_liquidity_per_tick(60);
        update(&mut ticks, -60, 0, 1000i128, 777, 888, false, max);
        update(&mut ticks, -60, 0, 1000i128, 9999, 9999, false, max);
        let (a, b) = fee_growth_outside(&ticks, -60);
        assert!(a == 777 && b == 888, 0);
        destroy(ticks);
    }

    #[test]
    #[expected_failure(abort_code = ELIQUIDITY_OVERFLOW, location = Self)]
    fun test_update_rejects_too_much_liquidity() {
        let ticks = new();
        update(&mut ticks, 60, 0, 1000i128, 0, 0, false, 999);
        destroy(ticks);
    }

    #[test]
    fun test_cross_flips_the_accumulators() {
        let ticks = new();
        let max = max_liquidity_per_tick(60);
        update(&mut ticks, -60, 0, 1000i128, 100, 200, false, max);
        let (a, b) = fee_growth_outside(&ticks, -60);
        assert!(a == 100 && b == 200, 0);

        let net = cross(&mut ticks, -60, 500, 900);
        assert!(net == 1000i128, 0);
        let (a, b) = fee_growth_outside(&ticks, -60);
        assert!(a == 400 && b == 700, 0);

        // Crossing back restores what was there.
        cross(&mut ticks, -60, 500, 900);
        let (a, b) = fee_growth_outside(&ticks, -60);
        assert!(a == 100 && b == 200, 0);
        destroy(ticks);
    }

    // Accumulators are only ever read as differences, so wrapping past the top
    // of `u128` still yields the right growth.
    #[test]
    fun test_cross_wraps_the_accumulators() {
        let ticks = new();
        let max = max_liquidity_per_tick(60);
        update(&mut ticks, -60, 0, 1000i128, MAX_U128, 0, false, max);
        cross(&mut ticks, -60, 9, 0);
        let (a, _) = fee_growth_outside(&ticks, -60);
        assert!(a == 10, 0);
        destroy(ticks);
    }

    #[test]
    #[expected_failure(abort_code = ETICK_NOT_INITIALIZED, location = Self)]
    fun test_cross_rejects_an_unknown_tick() {
        let ticks = new();
        cross(&mut ticks, 60, 0, 0);
        destroy(ticks);
    }

    #[test]
    fun test_clear_removes_the_tick() {
        let ticks = new();
        let max = max_liquidity_per_tick(60);
        update(&mut ticks, 60, 0, 1000i128, 0, 0, false, max);
        assert!(is_initialized(&ticks, 60), 0);
        clear(&mut ticks, 60);
        assert!(!is_initialized(&ticks, 60), 0);
        assert!(liquidity_gross(&ticks, 60) == 0, 0);
        // Clearing twice is harmless.
        clear(&mut ticks, 60);
        destroy(ticks);
    }

    // With the price inside the range, all growth counts as inside, and taking
    // the price back out does not retract it.
    #[test]
    fun test_fee_growth_inside() {
        let ticks = new();
        let max = max_liquidity_per_tick(60);
        update(&mut ticks, -60, 0, 1000i128, 0, 0, false, max);
        update(&mut ticks, 60, 0, 1000i128, 0, 0, true, max);

        let (a, b) = get_fee_growth_inside(&ticks, -60, 60, 0, 1000, 2000);
        assert!(a == 1000 && b == 2000, 0);

        cross(&mut ticks, -60, 1000, 2000);
        let (a, b) = get_fee_growth_inside(&ticks, -60, 60, -120, 1000, 2000);
        assert!(a == 1000 && b == 2000, 0);

        // A further 500 accrues below the range and none of it counts.
        let (a, b) = get_fee_growth_inside(&ticks, -60, 60, -120, 1500, 2500);
        assert!(a == 1000 && b == 2000, 0);
        destroy(ticks);
    }

    // Growth that accrues while the price sits outside the range does not
    // count toward it.
    #[test]
    fun test_fee_growth_inside_excludes_growth_from_elsewhere() {
        let ticks = new();
        let max = max_liquidity_per_tick(60);
        // The range is opened after 500 of growth already happened.
        update(&mut ticks, -60, 0, 1000i128, 500, 500, false, max);
        update(&mut ticks, 60, 0, 1000i128, 500, 500, true, max);
        let (a, b) = get_fee_growth_inside(&ticks, -60, 60, 0, 500, 500);
        assert!(a == 0 && b == 0, 0);
        // Another 300 accrues with the price inside.
        let (a, b) = get_fee_growth_inside(&ticks, -60, 60, 0, 800, 800);
        assert!(a == 300 && b == 300, 0);
        destroy(ticks);
    }

    #[test]
    fun test_info_at_is_empty_for_an_unknown_tick() {
        let ticks = new();
        let info = info_at(&ticks, 12345);
        assert!(info.liquidity_gross == 0, 0);
        assert!(info.liquidity_net == 0i128, 0);
        assert!(!info.initialized, 0);
        destroy(ticks);
    }
}
