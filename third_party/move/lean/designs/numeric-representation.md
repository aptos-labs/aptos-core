# Move numeric representation

Status: decided 2026-10-07. Leaner does not model the Move Prover's bit-vector
representation; see G14 in [`prover-test-problems.md`](prover-test-problems.md).

## Semantics

The Prover encodes some integers as SMT bit vectors: the arguments, results and
fields that `pragma bv`/`bv_ret` name, values that bitwise operations reach, and
the bodies of `bv_internal` functions. The spec-language book introduces this
encoding for efficiency. A side effect is that specification arithmetic over
encoded values wraps at their width.

Leaner keeps one semantics instead:

* Specification arithmetic is mathematical. Bitwise operations are exact on
  integers. Executable arithmetic is Move's checked arithmetic.
* `int2bv(e)` wraps `e` into its fixed-width result type: modulo `2^w` for an
  unsigned type, two's complement for a signed one. A generic or `num` result
  type has no width and is rejected.
* `bv2int(e)` reads the value back unchanged.
* A specification cast keeps its value.

The importer lowers `int2bv` to its operand where that is in range and to a
mathematical residue otherwise (`Frontend/BitVectorConversion.lean`, checked
against `BitVec` at all twelve widths by `Tests.BitVectorConversion`). As in the
Prover, `pragma bv`/`bv_ret` remain efficiency hints: they select Leaner's
bit-vector decision procedure (`Denote/BitLift.lean`) for a function's leaves,
which does not change their meaning. `bv_internal` is ignored.

## Differences from the Prover

The verifiers differ only on specifications whose truth depends on the
encoding's wrapping or on its unspecified narrowing casts.
`SourceVerify/bv_encoding.move` holds claims Leaner proves and the Prover
rejects, `bv_encoding_false.move` the reverse; `bv_conversion{,_false}.move`
hold the conversions, on which both agree.

## History

An earlier 2026-10-07 checkpoint rejected every package containing a
representation seed, then lowered closed scalar packages to the Prover's
wrapping arithmetic. Both were retired with this decision: the package guard
had made six benchmark modules fail to import, including framework
`ordered_map`, which uses no bit-vector representation itself. XAST version 10
still carries the `defaulted_num` literal marker that analysis used; the
importer does not read it.
