/// Test cases designed to highlight differences between the ValueShape (value-enumeration)
/// and Maranget (matrix-usefulness) algorithms for match coverage analysis.
///
/// Key differences visible in these tests:
///
/// 1. **Bool tuples**: ValueShape enumerates all missing concrete values individually
///    (e.g. `(true,false)`, `(false,true)`, `(false,false)`), while Maranget produces
///    more concise witnesses (e.g. `(true,false)`, `(false,_)`).
///
/// 2. **Deeper tuples**: The difference grows with width. For a 3-tuple of bools with
///    one arm, ValueShape lists 7 missing values while Maranget lists 3 concise witnesses.
///
/// 3. **Enum + primitive tuples**: Combines constructor splitting with literal analysis,
///    showing how each algorithm handles mixed types.
module 0xc0ffee::match_algo_cmp {

    enum Color {
        Red,
        Green,
        Blue,
    }

    // -------------------------------------------------------------------------
    // Case 1: Tuple of two bools, only (true, true) covered.
    //
    // ValueShape reports 3 missing values:
    //   missing `(true,false)`
    //   missing `(false,true)`
    //   missing `(false,false)`
    //
    // Maranget reports 2 witnesses:
    //   missing `(true,false)`
    //   missing `(false,_)`
    //
    // The Maranget result is more concise: once the first column is `false`,
    // no second-column value is covered, so `_` suffices.
    // -------------------------------------------------------------------------
    fun case1_bool_tuple(a: bool, b: bool): u8 {
        match ((a, b)) {
            (true, true) => 1,
        }
    }

    // -------------------------------------------------------------------------
    // Case 2: Triple of bools, only (true, true, true) covered.
    //
    // ValueShape reports 7 missing values (all 8 combinations minus the one covered):
    //   missing `(true,true,false)`
    //   missing `(true,false,true)`
    //   missing `(true,false,false)`
    //   missing `(false,true,true)`
    //   missing `(false,true,false)`
    //   missing `(false,false,true)`
    //   missing `(false,false,false)`
    //
    // Maranget reports 3 witnesses:
    //   missing `(true,true,false)`
    //   missing `(true,false,_)`
    //   missing `(false,_,_)`
    //
    // The conciseness grows exponentially: N bools with one arm gives
    // 2^N - 1 values from ValueShape but only N from Maranget.
    // -------------------------------------------------------------------------
    fun case2_bool_triple(a: bool, b: bool, c: bool): u8 {
        match ((a, b, c)) {
            (true, true, true) => 1,
        }
    }

    // -------------------------------------------------------------------------
    // Case 3: Enum tuple — partial coverage of Color × bool.
    //
    // Only Red+true and Green+true are covered. Missing:
    //
    // ValueShape:
    //   missing `(Red,false)`
    //   missing `(Green,false)`
    //   missing `(Blue,true)`
    //   missing `(Blue,false)`
    //
    // Maranget:
    //   missing `(Red,false)`
    //   missing `(Green,false)`
    //   missing `(Blue,_)`
    //
    // For Blue, no bool value is covered, so Maranget uses `_`.
    // -------------------------------------------------------------------------
    fun case3_enum_bool_tuple(c: Color, b: bool): u8 {
        match ((c, b)) {
            (Color::Red, true) => 1,
            (Color::Green, true) => 2,
        }
    }

    // -------------------------------------------------------------------------
    // Case 4: Deeply nested — two columns, each is (Color × bool).
    // Only one arm: both are (Red, true).
    //
    // ValueShape enumerates the full 36-element cross-product minus 1 = 35 missing.
    // Maranget reports far fewer witnesses.
    // -------------------------------------------------------------------------
    fun case4_deep_tuple(c1: Color, b1: bool, c2: Color, b2: bool): u8 {
        match ((c1, b1, c2, b2)) {
            (Color::Red, true, Color::Red, true) => 1,
        }
    }

    // -------------------------------------------------------------------------
    // Case 5: Reachability — both algorithms agree on unreachable arms.
    // This tests that the Maranget reachability check works correctly.
    // -------------------------------------------------------------------------
    fun case5_unreachable(c: Color): u8 {
        match (c) {
            Color::Red => 1,
            Color::Green => 2,
            Color::Blue => 3,
            _ => 4,  // unreachable: all variants already covered
        }
    }

    // -------------------------------------------------------------------------
    // Case 6: Integer literals with wildcard — both algorithms produce
    // equivalent results here (wildcard covers the infinite remainder).
    // -------------------------------------------------------------------------
    fun case6_int_nonexhaustive(x: u64): u64 {
        match (x) {
            0 => 100,
            1 => 200,
        }
    }
}
