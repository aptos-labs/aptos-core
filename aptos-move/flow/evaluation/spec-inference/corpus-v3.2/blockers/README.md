# Blockers found while building corpus V3

Both were found by screening V3 targets and are prover-side, not corpus-side.
Neither is fixed here: another agent owns Flow and the prover.

## 1. Enum-variant match behind a reference produces ill-typed Boogie — FIXED

`enum-ref-match-boogie.move` is an 18-line reproduction. Prove
`repro::repro::uses_it`:

```
Error: branches of if-then-else have incompatible types
       $42_repro_State and $Mutation $42_repro_State
Error: is-constructor must be applied to a datatype,
       type_checking_error$proxy#0 is not a datatype
```

Matching on `&Config` binds `state` as `$Mutation State` in the `V2` arm while
the `V1` arm yields a plain `State`, so the generated if-then-else is ill-typed.
The trigger is a *specification* referencing the helper; the module compiles and
proves vacuously without one.

**Fixed** in `spec_translator.rs::translate_select_variant`. The function
already dereferenced the *struct* type before `require_struct()`; it took the
*result* type verbatim, so selecting a field through `&Config` built the
fallback at `$Mutation State` while the arms above emit `$l -> field`, which is
the field's own type. Regression test:
`move-prover/tests/sources/functional/enum_ref_variant_select.move`.
All four `vault_share_math` targets now verify with a WP-inferred contract.

## 2. `calculate_pnl`: signed division disagrees between code and specification

WP infers a complete-looking contract for `extracted_pnl_math::calculate_pnl`,
including the `MIN_I64` negation overflow and both `MAX_I128` cast bounds, but
its own `ensures` then fails:

```
error: post-condition does not hold
  ensures [inferred] is_long ==> result == (((... as i128) - (... as i128))
                                            / (size_multiplier as i128)) as i64
```

Reduced to `signed-div-narrowing.move`. Each operation verifies **in
isolation** with complete abort conditions -- signed division (including the
`MIN / -1` overflow), the `i128 -> i64` narrowing, and the `u128 -> i128`
subtraction. Composed, the narrowing abort condition is rejected:

```
error: function does not abort under this condition
  aborts_if ((hi as i128) - (lo as i128)) / (m as i128) < MIN_I64
         || ((hi as i128) - (lo as i128)) / (m as i128) > MAX_I64;
```

The prover therefore finds a state where the specification's quotient is out of
`i64` range but the executable code does not abort, which is what a
truncation-versus-floor disagreement between the spec-level `/` and the code
would look like near the boundary. Not chased further here.

**Impact:** `PN-pnl-005` is excluded from the pilot and marked
`blocked_signed_div`. The target is otherwise good material -- WP finds the
`MIN_I64` negation overflow and both `MAX_I128` cast bounds -- so it should
return once this is resolved.
