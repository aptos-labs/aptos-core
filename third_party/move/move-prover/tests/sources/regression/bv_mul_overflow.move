/// Title: Bitvector multiplication returns 2 instead of reporting a u8 overflow
///
/// Description:
/// The public function multiplies a u8 value by 129. Its precondition fixes
/// the input to 2, so the mathematical product is 258. Checked u8 arithmetic
/// must abort because 258 is greater than 255. The prover instead accepts
/// both `aborts_if false` and the direct postcondition `result == 2u8`.
/// The identity `x | 0u8` selects the bitvector path without changing x.
///
/// Reproduction:
/// `cargo test -p move-prover --test testsuite bv_mul_overflow -- --test-threads=1`
///
/// Observed behavior:
/// The test passes because the prover accepts BvMulOverflow::multiply_incorrect.
/// It proves that the call does not abort and returns the wrapped value 2.
///
/// Concrete counterexample:
/// Set x to 2. The expression `x | 0u8` is 2, and 2 * 129 is 258.
/// There is no valid u8 result for this checked multiplication, so execution
/// must abort. The accepted contract instead describes a normal return of 2.
///
/// Expected behavior:
/// Verification must fail because `aborts_if false` does not cover the
/// required overflow. The result postcondition should be unreachable.
///
/// Root cause:
/// `BitOr` marks its result as `Bitwise`, and the `Mul` transfer rule merges
/// that classification into the multiplication result:
/// third_party/move/move-prover/bytecode-pipeline/src/number_operation_analysis.rs:1153-1172.
/// The backend emits `$OrBv8` for `|` and selects `$MulBv8` for `*`:
/// third_party/move/move-prover/boogie-backend/src/bytecode_translator.rs:7674-7687,7930-8010.
/// `$MulBv8` is defined in:
/// third_party/move/move-prover/boogie-backend/src/prelude/prelude.bpl:327-334.
/// Its overflow check compares the wrapped product with the first operand.
/// Here 258 wraps to 2, so the check becomes 2 < 2 and misses the abort.
///
/// A correct check needs the maximum value of the bitvector width. That value
/// comes from `BvInfo::max`, which `bv_helper` populates per width, and the
/// bv32 entry holds the maximum of a *signed* 32-bit integer, 2147483647,
/// where every other width holds `2^n - 1`:
/// third_party/move/move-prover/boogie-backend/src/lib.rs:244-247.
/// The same understated bound is unsound on its own: `$IsValid'bv32'` is an
/// assumption, not a check, so a bv32 literal above 2147483647 assumes false
/// and makes everything after it vacuous
/// (third_party/move/move-prover/boogie-backend/src/prelude/prelude.bpl:396-398),
/// while `$CastBv{n}to32` and `$int2bv32` abort above the same bound
/// (prelude.bpl:721-726,404-411) and the `$bv2int`/`$int2bv` round-trip axiom
/// is guarded by it (prelude.bpl:420-422).
/// Both defects are therefore fixed together: `bv_helper` gets the unsigned
/// bv32 maximum, and `$MulBv{n}` compares `src1` against `MAX / src2`.
module 0x42::BvMulOverflow {
    public fun multiply_incorrect(x: u8): u8 {
        (x | 0u8) * 129u8
    }
    spec multiply_incorrect {
        requires x == 2;
        aborts_if false;
        ensures result <= 255u8;
        ensures result == 2u8;
        ensures result  == (2 * 129);
    }
}
