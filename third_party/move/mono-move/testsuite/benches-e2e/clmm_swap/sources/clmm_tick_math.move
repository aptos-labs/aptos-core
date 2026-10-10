/// Conversions between a tick and its square-root price.
///
/// A tick `t` names the price `1.0001^t`, so the square-root price is
/// `sqrt(1.0001)^t` held as an unsigned Q64.96 fixed-point number: the integer
/// `x` stands for `x / 2^96`.
module bench::clmm_tick_math {
    use std::vector;
    use bench::clmm_full_math;

    /// Tick outside `[MIN_TICK, MAX_TICK]`.
    const ETICK_OUT_OF_BOUNDS: u64 = 1;
    /// Square-root price outside `[MIN_SQRT_PRICE, MAX_SQRT_PRICE]`.
    const EPRICE_OUT_OF_BOUNDS: u64 = 2;
    /// Tick is not a multiple of the pool's tick spacing.
    const EINVALID_TICK_SPACING: u64 = 3;
    /// Range bounds are not ordered, or fall outside the tick bounds.
    const EINVALID_TICK_RANGE: u64 = 4;
    /// Tick spacing of zero would divide by zero.
    const EZERO_TICK_SPACING: u64 = 5;

    /// The widest tick whose price `1.0001^t` still fits a Q64.64 price, which
    /// is the same bound the Cetus/Hyperion family of pools uses.
    const MAX_TICK: i32 = 443636;
    const MIN_TICK: i32 = -443636;

    const MIN_SQRT_PRICE: u128 = 18447090764788882727;
    const MAX_SQRT_PRICE: u128 = 340275971719517849884101479065584693833;

    const Q96: u128 = 79228162514264337593543950336;
    const Q128: u256 = 340282366920938463463374607431768211456;
    const Q224: u256 =
        26959946667150639794667015087019630673637144422540572481103610249216;

    /// `floor(sqrt(1.0001)^-(2^i) * 2^128)` for `i` in `0..=18`. Nineteen
    /// entries cover `abs_tick <= 524287`, which contains the tick bound.
    /// Halving the price each step keeps the running ratio at or below `2^128`,
    /// so `ratio * M_k` never leaves `u256`.
    const M_1: u256 = 340265354078544963557816517032075149313;
    const M_2: u256 = 340248342086729790484326174814286782777;
    const M_4: u256 = 340214320654664324051920982716015181259;
    const M_8: u256 = 340146287995602323631171512101879684303;
    const M_16: u256 = 340010263488231146823593991679159461443;
    const M_32: u256 = 339738377640345403697157401104375502015;
    const M_64: u256 = 339195258003219555707034227454543997024;
    const M_128: u256 = 338111622100601834656805679988414885970;
    const M_256: u256 = 335954724994790223023589805789778977699;
    const M_512: u256 = 331682121138379247127172139078559817299;
    const M_1024: u256 = 323299236684853023288211250268160618738;
    const M_2048: u256 = 307163716377032989948697243942600083928;
    const M_4096: u256 = 277268403626896220162999269216087595045;
    const M_8192: u256 = 225923453940442621947126027127485391332;
    const M_16384: u256 = 149997214084966997727330242082538205942;
    const M_32768: u256 = 66119101136024775622716233608466517925;
    const M_65536: u256 = 12847376061809297530290974190478138312;
    const M_131072: u256 = 485053260817066172746253684029974020;
    const M_262144: u256 = 691415978906521570653435304214167;

    /// `round(2^64 / log2(sqrt(1.0001)))`: turns a Q64.64 binary logarithm into
    /// a Q64.64 tick, which is why the product is then scaled down by `2^128`.
    const LOG2_TO_TICK: i256 = 255738958999603826347141;
    const Q64_SIGNED: i256 = 18446744073709551616;
    const Q128_SIGNED: i256 =
        340282366920938463463374607431768211456;
    const MAX_TICK_SIGNED: i256 = 443636;
    const MIN_TICK_SIGNED: i256 = -443636;

    /// Fractional bits recovered by the squaring loop. Thirty-two bits put the
    /// tick estimate within one tick, which the correction walk then closes.
    const LOG2_FRACTION_BITS: u64 = 32;

    public fun min_tick(): i32 {
        MIN_TICK
    }

    public fun max_tick(): i32 {
        MAX_TICK
    }

    public fun min_sqrt_price(): u128 {
        MIN_SQRT_PRICE
    }

    public fun max_sqrt_price(): u128 {
        MAX_SQRT_PRICE
    }

    public fun q96(): u128 {
        Q96
    }

    /// Abort unless `tick` is a multiple of `tick_spacing`.
    public fun check_tick_spacing(tick: i32, tick_spacing: u32) {
        assert!(tick_spacing != 0, EZERO_TICK_SPACING);
        assert!(tick % (tick_spacing as i32) == 0, EINVALID_TICK_SPACING);
    }

    /// Abort unless `[tick_lower, tick_upper)` is a usable position range.
    public fun check_tick_range(tick_lower: i32, tick_upper: i32, tick_spacing: u32) {
        assert!(tick_lower < tick_upper, EINVALID_TICK_RANGE);
        assert!(tick_lower >= MIN_TICK, EINVALID_TICK_RANGE);
        assert!(tick_upper <= MAX_TICK, EINVALID_TICK_RANGE);
        check_tick_spacing(tick_lower, tick_spacing);
        check_tick_spacing(tick_upper, tick_spacing);
    }

    /// `floor(sqrt(1.0001)^tick * 2^96)`.
    ///
    /// The magnitude of `tick` is decomposed into powers of two and the
    /// corresponding Q128.128 factors are multiplied together, so the cost is
    /// one multiply per set bit rather than one per tick.
    public fun get_sqrt_price_at_tick(tick: i32): u128 {
        assert!(tick >= MIN_TICK && tick <= MAX_TICK, ETICK_OUT_OF_BOUNDS);
        let abs_tick = (if (tick < 0) { -tick } else { tick }) as u32;

        let ratio = if (abs_tick & 1 != 0) { M_1 } else { Q128 };
        if (abs_tick & 2 != 0) { ratio = (ratio * M_2) >> 128; };
        if (abs_tick & 4 != 0) { ratio = (ratio * M_4) >> 128; };
        if (abs_tick & 8 != 0) { ratio = (ratio * M_8) >> 128; };
        if (abs_tick & 16 != 0) { ratio = (ratio * M_16) >> 128; };
        if (abs_tick & 32 != 0) { ratio = (ratio * M_32) >> 128; };
        if (abs_tick & 64 != 0) { ratio = (ratio * M_64) >> 128; };
        if (abs_tick & 128 != 0) { ratio = (ratio * M_128) >> 128; };
        if (abs_tick & 256 != 0) { ratio = (ratio * M_256) >> 128; };
        if (abs_tick & 512 != 0) { ratio = (ratio * M_512) >> 128; };
        if (abs_tick & 1024 != 0) { ratio = (ratio * M_1024) >> 128; };
        if (abs_tick & 2048 != 0) { ratio = (ratio * M_2048) >> 128; };
        if (abs_tick & 4096 != 0) { ratio = (ratio * M_4096) >> 128; };
        if (abs_tick & 8192 != 0) { ratio = (ratio * M_8192) >> 128; };
        if (abs_tick & 16384 != 0) { ratio = (ratio * M_16384) >> 128; };
        if (abs_tick & 32768 != 0) { ratio = (ratio * M_32768) >> 128; };
        if (abs_tick & 65536 != 0) { ratio = (ratio * M_65536) >> 128; };
        if (abs_tick & 131072 != 0) { ratio = (ratio * M_131072) >> 128; };
        if (abs_tick & 262144 != 0) { ratio = (ratio * M_262144) >> 128; };

        // `ratio` holds `sqrt(1.0001)^-abs_tick` in Q128.128. Positive ticks
        // want the reciprocal, and `2^224 / ratio` produces it already scaled
        // to Q64.96.
        if (tick > 0) {
            (Q224 / ratio) as u128
        } else {
            (ratio >> 32) as u128
        }
    }

    /// The greatest tick whose square-root price is at or below `sqrt_price`.
    ///
    /// A binary logarithm by repeated squaring lands within a tick of the
    /// answer; the walk that follows makes it exact.
    public fun get_tick_at_sqrt_price(sqrt_price: u128): i32 {
        assert!(
            sqrt_price >= MIN_SQRT_PRICE && sqrt_price <= MAX_SQRT_PRICE,
            EPRICE_OUT_OF_BOUNDS
        );

        let msb = clmm_full_math::most_significant_bit(sqrt_price as u256);
        // Rescale to `[1, 2)` in Q1.127, where squaring stays inside `u256`.
        let norm = if (msb >= 127) {
            (sqrt_price as u256) >> (msb - 127)
        } else {
            (sqrt_price as u256) << (127 - msb)
        };

        // Each squaring exposes one more fractional bit of the logarithm: the
        // square leaves `[1, 2)` exactly when that bit is set.
        let fraction = 0u128;
        let bit = 63u8;
        let i = 0;
        while (i < LOG2_FRACTION_BITS) {
            norm = (norm * norm) >> 127;
            let carry = (norm >> 128) as u8;
            fraction = fraction | ((carry as u128) << bit);
            norm = norm >> carry;
            bit = bit - 1;
            i = i + 1;
        };

        // log2(sqrt_price / 2^96) in Q64.64.
        let log_2 = ((msb as i256) - 96) * Q64_SIGNED + (fraction as i256);
        let estimate = (log_2 * LOG2_TO_TICK) / Q128_SIGNED;
        if (estimate > MAX_TICK_SIGNED) { estimate = MAX_TICK_SIGNED; };
        if (estimate < MIN_TICK_SIGNED) { estimate = MIN_TICK_SIGNED; };

        let tick = estimate as i32;
        while (tick < MAX_TICK && get_sqrt_price_at_tick(tick + 1) <= sqrt_price) {
            tick = tick + 1;
        };
        while (tick > MIN_TICK && get_sqrt_price_at_tick(tick) > sqrt_price) {
            tick = tick - 1;
        };
        tick
    }

    //
    // Tests.
    //
    // The expected prices come from an independent derivation of
    // `floor(sqrt(1.0001)^tick * 2^96)`: a Python script evaluating
    // `Decimal(1.0001).sqrt() ** tick` at 200 significant digits. Everything
    // through +/-200000 agrees with that value exactly; the entries past it
    // carry the fixed-point drift noted beside them, under 1e-29 relative.
    //

    #[test]
    fun test_sqrt_price_at_reference_ticks() {
        assert!(get_sqrt_price_at_tick(0) == 79228162514264337593543950336, 0);
        assert!(get_sqrt_price_at_tick(1) == 79232123823359799118286999567, 0);
        assert!(get_sqrt_price_at_tick(-1) == 79224201403219477170569942573, 0);
        assert!(get_sqrt_price_at_tick(100) == 79625275426524748796330556127, 0);
        assert!(get_sqrt_price_at_tick(-100) == 78833030112140176575862854578, 0);
        assert!(get_sqrt_price_at_tick(10000) == 130621891405341611593710811005, 0);
        assert!(get_sqrt_price_at_tick(-10000) == 48055510970269007215549348796, 0);
        // +2378741554 above the exact value, 7.0e-30 relative.
        assert!(
            get_sqrt_price_at_tick(443636) == 340275971719517849884101479065584693833,
            0
        );
        assert!(get_sqrt_price_at_tick(-443636) == 18447090764788882727, 0);
    }

    #[test]
    fun test_sqrt_price_at_more_reference_ticks() {
        assert!(get_sqrt_price_at_tick(2) == 79236085330515764027303304731, 0);
        assert!(get_sqrt_price_at_tick(-2) == 79220240490215316061937756560, 0);
        assert!(get_sqrt_price_at_tick(3) == 79240047035742135098198828267, 0);
        assert!(get_sqrt_price_at_tick(-3) == 79216279775241952975272415331, 0);
        assert!(get_sqrt_price_at_tick(1000) == 83290069058676223003182343269, 0);
        assert!(get_sqrt_price_at_tick(-1000) == 75364347830767020784054125654, 0);
        assert!(get_sqrt_price_at_tick(50000) == 965075977353221155028623082915, 0);
        assert!(get_sqrt_price_at_tick(-50000) == 6504256538020985011912221506, 0);
        assert!(get_sqrt_price_at_tick(100000) == 11755562826496067164730007768449, 0);
        assert!(get_sqrt_price_at_tick(-100000) == 533968626430936354154228407, 0);
        assert!(
            get_sqrt_price_at_tick(200000) == 1744244129640337381386292603617837,
            0
        );
        assert!(get_sqrt_price_at_tick(-200000) == 3598751819609688046946418, 0);
        // +26425710, 6.9e-31 relative.
        assert!(
            get_sqrt_price_at_tick(400000) == 38400329974042030913961448288742562463,
            0
        );
        assert!(get_sqrt_price_at_tick(-400000) == 163464786360687385625, 0);
        // +2347358271, 6.9e-30 relative.
        assert!(
            get_sqrt_price_at_tick(443635) == 340258959196860441002220642289651527915,
            0
        );
        assert!(get_sqrt_price_at_tick(-443635) == 18448013096269411586, 0);
    }

    #[test]
    fun test_bounds_agree_with_the_extreme_ticks() {
        assert!(get_sqrt_price_at_tick(MAX_TICK) == max_sqrt_price(), 0);
        assert!(get_sqrt_price_at_tick(MIN_TICK) == min_sqrt_price(), 0);
        assert!(get_sqrt_price_at_tick(0) == Q96, 0);
    }

    #[test]
    #[expected_failure(abort_code = ETICK_OUT_OF_BOUNDS, location = Self)]
    fun test_sqrt_price_above_max_tick() {
        get_sqrt_price_at_tick(MAX_TICK + 1);
    }

    #[test]
    #[expected_failure(abort_code = ETICK_OUT_OF_BOUNDS, location = Self)]
    fun test_sqrt_price_below_min_tick() {
        get_sqrt_price_at_tick(MIN_TICK - 1);
    }

    #[test]
    fun test_sqrt_price_is_strictly_increasing() {
        let t = -1000i32;
        let previous = get_sqrt_price_at_tick(t - 1);
        while (t <= 1000) {
            let price = get_sqrt_price_at_tick(t);
            assert!(price > previous, 0);
            previous = price;
            t = t + 1;
        };
    }

    #[test]
    fun test_tick_at_sqrt_price_round_trip() {
        let ticks = vector[
            0i32, 1, -1, 2, -2, 3, -3, 59, -59, 60, -60, 887, -887,
            1000, -1000, 4321, -4321, 10000, -10000, 50000, -50000,
            100000, -100000, 200000, -200000, 400000, -400000,
            443635, -443635, 443636, -443636
        ];
        let i = 0;
        while (i < vector::length(&ticks)) {
            let t = *vector::borrow(&ticks, i);
            assert!(get_tick_at_sqrt_price(get_sqrt_price_at_tick(t)) == t, i);
            i = i + 1;
        };
    }

    #[test]
    fun test_tick_at_sqrt_price_round_trip_dense() {
        let t = -600i32;
        while (t <= 600) {
            assert!(get_tick_at_sqrt_price(get_sqrt_price_at_tick(t)) == t, 0);
            t = t + 7;
        };
    }

    // A price strictly between two ticks resolves to the lower one, and one ULP
    // below a tick's own price resolves to the tick beneath it.
    #[test]
    fun test_tick_at_sqrt_price_floors() {
        let t = 12345i32;
        let low = get_sqrt_price_at_tick(t);
        let high = get_sqrt_price_at_tick(t + 1);
        assert!(get_tick_at_sqrt_price(low) == t, 0);
        assert!(get_tick_at_sqrt_price(low + 1) == t, 0);
        assert!(get_tick_at_sqrt_price(high - 1) == t, 0);
        assert!(get_tick_at_sqrt_price(high) == t + 1, 0);

        let n = -12345i32;
        let low_n = get_sqrt_price_at_tick(n);
        let high_n = get_sqrt_price_at_tick(n + 1);
        assert!(get_tick_at_sqrt_price(low_n) == n, 0);
        assert!(get_tick_at_sqrt_price(high_n - 1) == n, 0);
        assert!(get_tick_at_sqrt_price(high_n) == n + 1, 0);
    }

    #[test]
    #[expected_failure(abort_code = EPRICE_OUT_OF_BOUNDS, location = Self)]
    fun test_tick_at_sqrt_price_below_min() {
        get_tick_at_sqrt_price(MIN_SQRT_PRICE - 1);
    }

    #[test]
    #[expected_failure(abort_code = EPRICE_OUT_OF_BOUNDS, location = Self)]
    fun test_tick_at_sqrt_price_above_max() {
        get_tick_at_sqrt_price(MAX_SQRT_PRICE + 1);
    }

    #[test]
    fun test_check_tick_spacing() {
        check_tick_spacing(0, 60);
        check_tick_spacing(60, 60);
        check_tick_spacing(-60, 60);
        check_tick_spacing(-443580, 60);
        check_tick_spacing(7, 1);
    }

    #[test]
    #[expected_failure(abort_code = EINVALID_TICK_SPACING, location = Self)]
    fun test_check_tick_spacing_rejects_misaligned() {
        check_tick_spacing(61, 60);
    }

    #[test]
    #[expected_failure(abort_code = EINVALID_TICK_SPACING, location = Self)]
    fun test_check_tick_spacing_rejects_misaligned_negative() {
        check_tick_spacing(-61, 60);
    }

    #[test]
    #[expected_failure(abort_code = EINVALID_TICK_RANGE, location = Self)]
    fun test_check_tick_range_rejects_inverted() {
        check_tick_range(60, -60, 60);
    }

    #[test]
    #[expected_failure(abort_code = EINVALID_TICK_RANGE, location = Self)]
    fun test_check_tick_range_rejects_empty() {
        check_tick_range(60, 60, 60);
    }
}
