# Conversion-free integer specifications

Status: **implemented design**.

## Objective

Leaner specifications should use Move Specification Language's mathematical
integer domain without requiring authors or the transpiler to spell
representation conversions:

```lean
invariant index = start_index ∨ is_index_set self (index - 1)
invariant index = start_index ∨ index - 1 < self.bit_field.length
invariant ∀ (j : Int), start_index ≤ j ∧ j < index → is_index_set self j
```

The specification surface should not contain routine `.toInt`, `.toNat`,
`.toList`, or `MoveInt.ofInt` applications. Those are representation details,
not specification concepts.

## Decision

Executable Move integers remain exactly as they are. `MoveInt`, its bounded
operators, checked arithmetic, abort behavior, compiler markers, and runtime
lowering are not redesigned.

Instead, specification elaboration provides a logical layer:

- Spec-function bodies are rewritten to mathematical operations over `Int`
  before Lean elaborates them.
- Contracts and invariants enter the same mathematical layer where the clean
  surface demands it: at integer spec calls, explicit `Int` binders, shifts,
  and representation-free length/size expressions. Homogeneous legacy
  expressions with explicit representation views retain their existing Lean
  meaning.
- Direct integer parameters and results of the specification version of a
  Move function are mapped to `Int`.
- Calls to specification functions project bounded Move arguments to `Int`
  at the elaboration boundary. The projection is never printed in the source.
- Vector length, indexing, membership, update, and append have a clean scoped
  specification surface; their list representation remains internal.
- Uninterpreted functions use `spec opaque`, which records the same logical
  integer signature and call behavior as a defined `spec fun`.

There is deliberately no bounded re-packing inside a spec-function call. A
mathematical expression passed to a spec function stays mathematical.

## Spec-function typing rule

A derived specification version maps each direct Move integer parameter and
result to `Int`. For example:

```lean
fun double (value : U64) : U64 := value + value
```

has a logical specification version with the effective signature:

```lean
double.specFun : Int → Int
```

Its body is read using mathematical arithmetic. A call such as
`double (index - 1)` in a clause therefore needs neither `ofInt` nor `toInt`.

An authored specification function must state this rule explicitly:

```lean
spec fun distance (left : Int) (right : Int) : Int := right - left
spec opaque enabled (feature : Int) : Prop
```

Using `U8` through `U256` or `I8` through `I256` as a direct parameter or
result of `spec fun`/`spec opaque` is a compilation error. This prevents a
bounded type from silently re-entering the logical layer. Nested integers in
Move data structures are not rewritten structurally: for example,
`Vector U64` remains `Vector U64`, while an element is projected when it
participates in mathematical arithmetic or is passed to an `Int` parameter.

## Abort behavior

The value of a spec function on an aborting execution is undefined. Therefore
its logical arithmetic does not reproduce overflow, underflow, division, or
shift aborts and does not wrap into the source width.

This is intentional. Specifications state mathematical values; executable
verification separately proves checked-operation conditions and declared
abort behavior. If a contract equates an executable result with a
mathematical spec function, the non-aborting range must be available from the
function's checked semantics or stated as a precondition.

## Elaboration boundary

The clean syntax is interpreted only in specification positions. A
spec-function body is wholly mathematical. In contracts and invariants the
clause rewriter propagates that context from `Int` binders and integer
spec-function arguments/results, and recognizes conversion-free length,
size, shift, and overflow-bound expressions. It handles:

- `+`, `-`, `*`, `/`, `%`, unary negation, and shifts;
- heterogeneous `=`, `≠`, `<`, `>`, `≤`, and `≥` involving bounded
  values, `Int`, vector lengths, and arithmetic expressions;
- registered `spec fun` and `spec opaque` calls;
- vector indexing and membership without exposing `.toList`.

Data invariants are assembled earlier by the module macro, so that path runs
the same mathematical operator rewrite before declaring the invariant.

Existing specifications which already spell `.toNat`, `.toInt`, or `.toList`
are treated as explicit Lean representation expressions. Their surrounding
homogeneous arithmetic and relations are preserved. This compatibility rule
lets old proofs keep their definitional shape while new and transpiled specs
omit the representation plumbing.

This is not implemented as a global coercion. A `Move.Spec`-scoped coercion
handles bounded leaves only when a composite specification expression already
has an expected `Int` type; it does not drive operator selection. Move's
executable operator instances are unchanged, and ordinary source functions
continue to be type-checked and lowered with bounded Move semantics.

## Proof surface

Proofs still eventually need to relate the logical `Int` value to the bounded
value produced by executable semantics. The difference is where that work
lives:

- specification authors do not insert conversions;
- `move_norm` unfolds the logical arithmetic/relation helpers to canonical
  `Int` expressions;
- unsigned proof passes rewrite `toInt` to the `toNat` view when natural-number
  arithmetic or vector lengths require it;
- vector bridge lemmas relate logical membership/indexing to executable
  vector operations;
- `uint_bounds`, `omega`, and existing checked-operation lemmas discharge the
  resulting range facts.

Thus normalized internal goals may contain `MoveInt.toInt`, `UInt.toNat`, or
`Vector.toList`, but hand-written specifications and routine proofs do not
repeat those conversions. They are centralized proof infrastructure.

An established proof model may intentionally use `Nat` and `List`. In that
case a small logical-view adapter is the boundary: contracts pass `Vector`
and `Int` values to the adapter without conversions, while the adapter and
the model's internal lemmas retain their existing representations. The
Quicksort verification exercises this pattern end to end.

## Transpiler contract

In specification output the transpiler:

- prints Move integer parameters/results of spec functions as `Int`;
- emits bare integer leaves and mathematical expressions;
- emits `.length`, `values[index]!`, and `x ∈ values` rather than list-view
  plumbing;
- emits `spec opaque` for uninterpreted specification functions;
- leaves executable function signatures and bodies unchanged.

The elaborator, rather than the textual producer, owns insertion of internal
projections. This keeps generated specs readable and gives hand-written specs
the identical language.

## Non-goals

This change does not:

- replace `MoveInt` with `Int` in executable code;
- remove bounded arithmetic/comparison instances;
- change checked semantics, gas behavior, compiler lowering, or runtime
  representation;
- define abort results for spec functions;
- recursively turn every integer nested inside a container or structure into
  `Int`;
- eliminate representation views from the implementation of proof tactics.

Those broader changes had substantial compatibility and soundness costs and
are unnecessary for the actual objective: conversion-free Leaner specs.
