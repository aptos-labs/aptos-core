# Move numeric representation

Verex does not model the Move Prover's bit-vector representation; see G14 in
[`prover-test-problems.md`](../status/prover-test-problems.md).

## Semantics

The Prover encodes some integers as SMT bit vectors: the arguments, results and
fields that `pragma bv`/`bv_ret` name, values that bitwise operations reach, and
the bodies of `bv_internal` functions. A side effect is that specification
arithmetic over encoded values wraps at their width.

Verex keeps one semantics:

* Specification arithmetic is mathematical. Bitwise operations are exact on
  integers. Executable arithmetic is Move's checked arithmetic.
* `int2bv(e)` wraps `e` into its fixed-width result type: modulo `2^w` for an
  unsigned type, two's complement for a signed one. The compiler types the
  conversion of an arithmetic operand at `num`, which has no width; that one
  wraps at `u64`, the type Move gives an integer nothing else constrains. A
  type-parameter result has no width and is rejected.
* `bv2int(e)` reads the value back unchanged.
* A specification cast keeps its value.

The importer lowers `int2bv` to its operand where that is in range and to a
mathematical residue otherwise (`Frontend/BitVectorConversion.lean`, checked
against `BitVec` at all twelve widths by `Tests.BitVectorConversion`). As in the
Prover, `pragma bv`/`bv_ret` are efficiency hints: they select the bit-vector
decision procedure (`Denote/BitLift.lean`) for a function's leaves, which does
not change their meaning. `bv_internal` is ignored.

## Differences from the Prover

The verifiers differ only on specifications whose truth depends on the
encoding's wrapping or on its unspecified narrowing casts.
`SourceVerify/bv_encoding.move` holds claims Verex proves and the Prover
rejects, `bv_encoding_false.move` the reverse; `bv_conversion{,_false}.move`
hold the conversions, on which both agree. A package guard rejecting every
package with a representation seed was rejected: it made six benchmark
modules fail to import.
