/// A sparse bitmap over initialized ticks, one `u256` word per 256 ticks.
///
/// A swap uses it to find the next tick that carries liquidity without reading
/// every tick in between, which is what makes the read set of a swap depend on
/// how far the price moves.
module bench::clmm_tick_bitmap {
    use aptos_std::table::{Self, Table};
    use bench::clmm_full_math;

    /// Tick is not a multiple of the pool's tick spacing.
    const EINVALID_TICK_SPACING: u64 = 1;
    /// Tick spacing of zero would divide by zero.
    const EZERO_TICK_SPACING: u64 = 2;

    const MAX_U256: u256 =
        115792089237316195423570985008687907853269984665640564039457584007913129639935;

    struct BitMap has store {
        map: Table<i16, u256>,
    }

    public fun new(): BitMap {
        BitMap { map: table::new() }
    }

    /// `a / b` rounded toward negative infinity, for `b > 0`.
    ///
    /// Move truncates toward zero, so a negative quotient with a remainder
    /// comes out one too high.
    fun floor_div(a: i32, b: i32): i32 {
        let quotient = a / b;
        if (a < 0 && quotient * b != a) {
            quotient - 1
        } else {
            quotient
        }
    }

    /// Tick index in units of `tick_spacing`, rounded toward negative infinity.
    public fun compress(tick: i32, tick_spacing: u32): i32 {
        assert!(tick_spacing != 0, EZERO_TICK_SPACING);
        floor_div(tick, tick_spacing as i32)
    }

    /// Word index and bit offset holding `compressed`.
    ///
    /// Flooring the word index is what keeps the bit offset in `[0, 255]`:
    /// truncating would file ticks -255..-1 in word 0 next to 0..255 and leave
    /// a negative offset behind.
    public fun position(compressed: i32): (i16, u8) {
        let word = floor_div(compressed, 256);
        let bit = compressed - word * 256;
        ((word as i16), (bit as u8))
    }

    fun word_at(self: &BitMap, word: i16): u256 {
        *table::borrow_with_default(&self.map, word, &0)
    }

    /// Toggle the initialized flag of `tick`.
    public fun flip_tick(self: &mut BitMap, tick: i32, tick_spacing: u32) {
        assert!(tick_spacing != 0, EZERO_TICK_SPACING);
        assert!(tick % (tick_spacing as i32) == 0, EINVALID_TICK_SPACING);
        let (word, bit) = position(compress(tick, tick_spacing));
        let slot = table::borrow_mut_with_default(&mut self.map, word, 0);
        *slot = *slot ^ (1u256 << bit);
    }

    public fun is_initialized(self: &BitMap, tick: i32, tick_spacing: u32): bool {
        let (word, bit) = position(compress(tick, tick_spacing));
        word_at(self, word) & (1u256 << bit) != 0
    }

    /// Nearest initialized tick within the word holding `tick`, searching down
    /// when `lte` and up otherwise.
    ///
    /// Returns the boundary of the search when the word holds nothing, along
    /// with `false`, so a caller can step to the next word and continue.
    public fun next_initialized_tick_within_one_word(
        self: &BitMap, tick: i32, tick_spacing: u32, lte: bool
    ): (i32, bool) {
        let compressed = compress(tick, tick_spacing);
        let spacing = tick_spacing as i32;
        if (lte) {
            let (word, bit) = position(compressed);
            // Every bit at or below `bit`. Written as two terms because
            // `1 << 255 << 1` would leave `u256`.
            let mask = ((1u256 << bit) - 1) + (1u256 << bit);
            let masked = word_at(self, word) & mask;
            if (masked != 0) {
                let offset = bit - clmm_full_math::most_significant_bit(masked);
                ((compressed - (offset as i32)) * spacing, true)
            } else {
                ((compressed - (bit as i32)) * spacing, false)
            }
        } else {
            let (word, bit) = position(compressed + 1);
            let mask = MAX_U256 ^ ((1u256 << bit) - 1);
            let masked = word_at(self, word) & mask;
            if (masked != 0) {
                let offset = clmm_full_math::least_significant_bit(masked) - bit;
                ((compressed + 1 + (offset as i32)) * spacing, true)
            } else {
                ((compressed + 1 + ((255 - bit) as i32)) * spacing, false)
            }
        }
    }

    //
    // Tests.
    //

    #[test]
    fun test_position_on_positive_ticks() {
        let (w, b) = position(0);
        assert!(w == 0i16 && b == 0, 0);
        let (w, b) = position(1);
        assert!(w == 0i16 && b == 1, 0);
        let (w, b) = position(255);
        assert!(w == 0i16 && b == 255, 0);
        let (w, b) = position(256);
        assert!(w == 1i16 && b == 0, 0);
        let (w, b) = position(511);
        assert!(w == 1i16 && b == 255, 0);
        let (w, b) = position(443636);
        assert!(w == 1732i16 && b == 244, 0);
    }

    // Truncation toward zero is the trap here: -1 belongs in word -1 at bit
    // 255, not word 0 at bit -1.
    #[test]
    fun test_position_on_negative_ticks() {
        let (w, b) = position(-1);
        assert!(w == -1i16 && b == 255, 0);
        let (w, b) = position(-2);
        assert!(w == -1i16 && b == 254, 0);
        let (w, b) = position(-255);
        assert!(w == -1i16 && b == 1, 0);
        let (w, b) = position(-256);
        assert!(w == -1i16 && b == 0, 0);
        let (w, b) = position(-257);
        assert!(w == -2i16 && b == 255, 0);
        let (w, b) = position(-512);
        assert!(w == -2i16 && b == 0, 0);
        let (w, b) = position(-443636);
        assert!(w == -1733i16 && b == 12, 0);
    }

    // Word and bit reconstruct the tick they came from, on both signs.
    #[test]
    fun test_position_round_trip() {
        let t = -1000i32;
        while (t <= 1000) {
            let (w, b) = position(t);
            assert!((w as i32) * 256 + (b as i32) == t, 0);
            t = t + 1;
        };
    }

    #[test]
    fun test_compress_floors_on_negative_ticks() {
        assert!(compress(120, 60) == 2, 0);
        assert!(compress(119, 60) == 1, 0);
        assert!(compress(0, 60) == 0, 0);
        assert!(compress(-1, 60) == -1, 0);
        assert!(compress(-60, 60) == -1, 0);
        assert!(compress(-61, 60) == -2, 0);
        assert!(compress(-119, 60) == -2, 0);
        assert!(compress(-120, 60) == -2, 0);
        assert!(compress(-121, 60) == -3, 0);
    }

    #[test]
    fun test_flip_tick_toggles() {
        let bm = new();
        assert!(!is_initialized(&bm, 60, 60), 0);
        flip_tick(&mut bm, 60, 60);
        assert!(is_initialized(&bm, 60, 60), 0);
        flip_tick(&mut bm, 60, 60);
        assert!(!is_initialized(&bm, 60, 60), 0);
        destroy(bm);
    }

    #[test]
    fun test_flip_tick_is_independent_across_ticks() {
        let bm = new();
        flip_tick(&mut bm, -120, 60);
        flip_tick(&mut bm, 120, 60);
        assert!(is_initialized(&bm, -120, 60), 0);
        assert!(is_initialized(&bm, 120, 60), 0);
        assert!(!is_initialized(&bm, 0, 60), 0);
        assert!(!is_initialized(&bm, 60, 60), 0);
        assert!(!is_initialized(&bm, -60, 60), 0);
        destroy(bm);
    }

    #[test]
    #[expected_failure(abort_code = EINVALID_TICK_SPACING, location = Self)]
    fun test_flip_tick_rejects_misaligned() {
        let bm = new();
        flip_tick(&mut bm, 61, 60);
        destroy(bm);
    }

    #[test]
    fun test_next_initialized_finds_a_flipped_tick_downward() {
        let bm = new();
        flip_tick(&mut bm, 120, 60);
        let (next, found) = next_initialized_tick_within_one_word(&bm, 300, 60, true);
        assert!(found, 0);
        assert!(next == 120, 0);
        // Starting exactly on the tick still finds it, since the search is
        // inclusive at or below.
        let (next, found) = next_initialized_tick_within_one_word(&bm, 120, 60, true);
        assert!(found && next == 120, 0);
        destroy(bm);
    }

    #[test]
    fun test_next_initialized_finds_a_flipped_tick_upward() {
        let bm = new();
        flip_tick(&mut bm, 120, 60);
        let (next, found) = next_initialized_tick_within_one_word(&bm, 0, 60, false);
        assert!(found, 0);
        assert!(next == 120, 0);
        // The upward search starts strictly above, so standing on the tick
        // skips past it.
        let (next, found) = next_initialized_tick_within_one_word(&bm, 120, 60, false);
        assert!(!found, 0);
        assert!(next > 120, 0);
        destroy(bm);
    }

    #[test]
    fun test_next_initialized_across_negative_ticks() {
        let bm = new();
        flip_tick(&mut bm, -120, 60);
        let (next, found) = next_initialized_tick_within_one_word(&bm, -60, 60, true);
        assert!(found && next == -120, 0);
        let (next, found) = next_initialized_tick_within_one_word(&bm, -600, 60, false);
        assert!(found && next == -120, 0);
        destroy(bm);
    }

    // A word with nothing in it reports the edge of the word it searched, so
    // the caller can resume from there.
    #[test]
    fun test_next_initialized_reports_none_within_the_word() {
        let bm = new();
        let (next, found) = next_initialized_tick_within_one_word(&bm, 300, 60, true);
        assert!(!found, 0);
        // Word 0 covers compressed 0..255, so the floor of the downward search
        // from compressed 5 is compressed 0.
        assert!(next == 0, 0);

        let (next, found) = next_initialized_tick_within_one_word(&bm, 300, 60, false);
        assert!(!found, 0);
        assert!(next == 255 * 60, 0);

        // Below zero the search walks word -1, which covers compressed
        // -256..-1.
        let (next, found) = next_initialized_tick_within_one_word(&bm, -300, 60, true);
        assert!(!found, 0);
        assert!(next == -256 * 60, 0);

        let (next, found) = next_initialized_tick_within_one_word(&bm, -300, 60, false);
        assert!(!found, 0);
        assert!(next == -60, 0);
        destroy(bm);
    }

    // A tick sitting on a word boundary is not visible from the neighbouring
    // word, which is exactly why the pool loop has to step word by word.
    #[test]
    fun test_next_initialized_stops_at_the_word_edge() {
        let bm = new();
        flip_tick(&mut bm, 0, 60);
        let (next, found) = next_initialized_tick_within_one_word(&bm, 256 * 60, 60, true);
        assert!(!found, 0);
        assert!(next == 256 * 60, 0);
        let (next, found) = next_initialized_tick_within_one_word(&bm, 255 * 60, 60, true);
        assert!(found && next == 0, 0);
        destroy(bm);
    }

    #[test]
    fun test_next_initialized_picks_the_closest() {
        let bm = new();
        flip_tick(&mut bm, 0, 60);
        flip_tick(&mut bm, 60, 60);
        flip_tick(&mut bm, 600, 60);
        let (next, found) = next_initialized_tick_within_one_word(&bm, 300, 60, true);
        assert!(found && next == 60, 0);
        let (next, found) = next_initialized_tick_within_one_word(&bm, 300, 60, false);
        assert!(found && next == 600, 0);
        destroy(bm);
    }

    #[test]
    fun test_spacing_one_at_the_tick_bound() {
        let bm = new();
        flip_tick(&mut bm, -443636, 1);
        flip_tick(&mut bm, 443636, 1);
        assert!(is_initialized(&bm, -443636, 1), 0);
        assert!(is_initialized(&bm, 443636, 1), 0);
        let (next, found) = next_initialized_tick_within_one_word(&bm, -443600, 1, true);
        assert!(found && next == -443636, 0);
        let (next, found) = next_initialized_tick_within_one_word(&bm, 443600, 1, false);
        assert!(found && next == 443636, 0);
        destroy(bm);
    }

    // `BitMap` owns a `Table`, which has no `drop`, so tests dismantle it
    // explicitly rather than letting it fall out of scope.
    #[test_only]
    fun destroy(self: BitMap) {
        let BitMap { map } = self;
        table::drop_unchecked(map);
    }
}
