# Tactic-Enhanced Move Specification Language

## Context

The Move Prover relies entirely on Z3/CVC5 SMT automation. While this works for
most verification goals, a significant fraction hit timeouts or inconclusiveness
due to non-linear arithmetic, deep quantifier reasoning, or complex induction.
Currently the only "knobs" available are:

- **Pragmas**: `opaque`, `timeout`, `seed`, `unroll`, `inference`
- **Quantifier triggers**: `forall x: T {trigger}: body` (per-quantifier only)
- **Inline assertions**: `spec { assert ...; }` at code points

There is no mechanism for structured proof hints that guide the solver through
hard proofs. This plan adds `proof { ... }` blocks to MSL with tactic commands
that compile to Boogie/SMT constructs, enabling both human and AI-driven proof
guidance.

## Syntax Design

Proof blocks are allowed in function-level spec blocks only (not struct
invariants, module invariants, or schemas).

```move
spec transfer {
    ensures balance(sender) == old(balance(sender)) - amount;
    ensures balance(receiver) == old(balance(receiver)) + amount;

    proof {
        unfold balance;                                    // expand opaque spec function
        assert sum_all() == old(sum_all());                // auxiliary lemma
        use conservation_lemma(sender, receiver, amount);  // instantiate lemma
        assume [trusted] some_hard_property();             // guarded assume (warning emitted)
    }
}
```

### Phase 1 Tactics

| Tactic | Semantics | Boogie Encoding |
|--------|-----------|-----------------|
| `unfold <fun>;` | Expand spec function body in this VC | `assume (forall args :: f(args) == body);` |
| `assert <expr>;` | Auxiliary lemma (proved, then available) | `assert expr;` before postconditions |
| `use <fun>(args);` | Instantiate quantified axiom/lemma at specific values | `assert lemma_fun(args);` |
| `assume [trusted] <expr>;` | Guarded assumption (requires `[trusted]` annotation) | `assume expr;` with warning diagnostic |

The `assume [trusted]` tactic is intentionally gated behind the `[trusted]`
annotation. When used, the prover emits a warning: `"proof uses trusted
assumption at <loc>"`. This is useful for exploration (testing whether an
assumption would make the proof go through) but should not be used in production
specs.

### Phase 2 Tactics (future)

| Tactic | Semantics | Boogie Encoding |
|--------|-----------|-----------------|
| `trigger <quant> with {exprs};` | Add E-matching triggers | `{:trigger exprs}` on quantifier |
| `split on <expr>;` | Case-split the VC | Two verification procedures |
| `induct on <var>;` | Natural/structural induction | Base case + inductive step VCs |

## Implementation Plan

### Step 1: Parser — Add `proof` block to spec syntax

**File:** `third_party/move/move-compiler-v2/legacy-move-compiler/src/parser/ast.rs`

- Add new variant to `SpecBlockMember_`:
  ```rust
  Proof {
      hints: Vec<ProofHint>,
  }
  ```
- Define `ProofHint` and `ProofHint_` types:
  ```rust
  pub type ProofHint = Spanned<ProofHint_>;
  pub enum ProofHint_ {
      Unfold(NameAccessChain),                       // unfold <spec_fun>;
      Assert(Exp),                                    // assert <expr>;
      Use(NameAccessChain, Vec<Exp>),                 // use <fun>(args);
      Assume(Vec<PragmaProperty>, Exp),               // assume [trusted] <expr>;
  }
  ```

**File:** `third_party/move/move-compiler-v2/legacy-move-compiler/src/parser/syntax.rs`

- In `parse_spec_block_member()` (~line 4183): add `"proof"` to the identifier match
- Implement `parse_proof_block()`:
  - Consume `proof` keyword
  - Consume `{`
  - Loop parsing individual hint commands until `}`
  - Each hint: match on `"unfold"`, `"assert"`, `"assume"`, `"use"` keywords
  - `unfold`: parse `NameAccessChain`, consume `;`
  - `assert`: parse `Exp`, consume `;`
  - `assume`: parse `[trusted]` properties (reuse `parse_condition_properties`),
    parse `Exp`, consume `;`. Error if `[trusted]` annotation missing
  - `use`: parse `NameAccessChain`, parse parenthesized arg list, consume `;`

### Step 2: Model — Represent proof hints in the semantic model

**File:** `third_party/move/move-model/src/ast.rs`

- Add proof hint representation:
  ```rust
  pub enum ProofHint {
      /// Expand the body of the named spec function in this verification context.
      Unfold(Loc, QualifiedSymbol),
      /// An auxiliary assertion (lemma) to prove before the main postconditions.
      Assert(Loc, Exp),
      /// Instantiate a spec function/axiom at specific arguments.
      Use(Loc, QualifiedId<FunId>, Vec<Exp>),
      /// Trusted assumption (requires [trusted] annotation; emits warning).
      Assume(Loc, Exp),
  }
  ```
- Add field to `Spec`:
  ```rust
  pub struct Spec {
      // ... existing fields ...
      pub proof_hints: Vec<ProofHint>,
  }
  ```
- Update `Spec::structural_eq()`, `Spec::is_empty()`, `Spec::visit_*` methods
  to account for proof hints
- Update `Spec::used_funs_with_uses()` and `Spec::called_funs_with_callsites()`
  to include functions referenced in proof hints

**File:** `third_party/move/move-model/src/pragmas.rs`

- Not strictly needed for Phase 1 (proof hints are not pragma-based), but add
  a doc comment noting the new `proof` block feature

### Step 3: Model Builder — Translate parsed proof hints to model

**File:** `third_party/move/move-model/src/builder/module_builder.rs`

- In `def_ana_spec()` and `def_ana_code_spec_block()` (~line 1176): add a match
  arm for `EA::SpecBlockMember_::Proof { hints }`:
  - For each `ProofHint_::Unfold(name)`: resolve `name` to a `QualifiedSymbol`
    referencing a spec function; error if not found or not a spec function
  - For each `ProofHint_::Assert(exp)`: type-check `exp` as a boolean expression
    in the current spec context (reuse `def_ana_condition` machinery)
  - For each `ProofHint_::Use(name, args)`: resolve `name` as a spec function,
    type-check args against its signature
  - For each `ProofHint_::Assume(props, exp)`: verify `[trusted]` annotation
    present in `props`; type-check `exp` as boolean; emit warning diagnostic
  - Store results in `spec.proof_hints`

- Validate that proof hints are only allowed in function-level spec blocks (not
  struct invariants, module invariants, or schemas); emit error otherwise

### Step 4: Spec Checker — Ensure proof hint expressions are pure

**File:** `third_party/move/move-compiler-v2/src/env_pipeline/spec_checker.rs`

- In `check_spec()`: iterate over `spec.proof_hints` and check pureness of
  `Assert` and `Use` expressions (same as existing condition checking)

### Step 5: Pipeline — Process proof hints before Boogie generation

**File (new):** `third_party/move/move-prover/bytecode-pipeline/src/proof_hint_processor.rs`

- Implement `FunctionTargetProcessor` trait
- `process()` method:
  - Read `proof_hints` from `func_env.get_spec()`
  - For `Unfold(sym)`: record the spec function symbol in an annotation on the
    `FunctionData` (e.g., `UnfoldAnnotation(BTreeSet<QualifiedSymbol>)`)
  - For `Assert(exp)`: inject `Bytecode::Prop(Assert, exp)` at the function
    exit point, BEFORE the existing postcondition assertions. The VC info
    message should be `"proof hint assertion does not hold"`
  - For `Use(fun_id, args)`: inject `Bytecode::Prop(Assert, fun_call_exp)`
    that asserts the spec function applied to the given args
  - For `Assume(exp)`: inject `Bytecode::Prop(Assume, exp)` after the assert
    hints. Emit a warning diagnostic: `"proof uses trusted assumption"`

**File:** `third_party/move/move-prover/bytecode-pipeline/src/pipeline_factory.rs`

- Add `ProofHintProcessor::new()` after `SpecInstrumentationProcessor` and
  before `GlobalInvariantAnalysisProcessor` (around line 53-54)

**File:** `third_party/move/move-prover/bytecode-pipeline/src/lib.rs`

- Add `pub mod proof_hint_processor;`

### Step 6: Boogie Backend — Handle unfold annotations

**File:** `third_party/move/move-prover/boogie-backend/src/bytecode_translator.rs`

- In `generate_function_body()` (~line 2407), after entry assumptions:
  - Check for `UnfoldAnnotation` on the function target
  - For each unfolded spec function: emit an `assume` of the function's
    definition axiom (i.e., `assume forall args :: f(args) == body(args)`)
  - This makes the function's body available to the solver even if it is
    marked `opaque`

**File:** `third_party/move/move-prover/boogie-backend/src/spec_translator.rs`

- No changes needed for Phase 1 — the `Assert` and `Use` hints are already
  bytecode `Prop` instructions and translate normally

### Step 7: Sourcifier — Round-trip proof hints back to source

**File:** `third_party/move/move-model/src/sourcifier.rs`

- In the spec printing code: if `spec.proof_hints` is non-empty, emit a
  `proof { ... }` block with each hint rendered in Move syntax

### Step 8: Tests

**File (new):** `third_party/move/move-prover/tests/sources/functional/proof_hints.move`

Test cases:
1. `unfold` on an opaque spec function enables verification that would
   otherwise timeout/fail
2. `assert` lemma is proved and helps prove the main postcondition
3. `use` instantiates a spec function at specific values
4. `assume [trusted]` produces a warning but allows verification to succeed
5. Error: `assume` without `[trusted]` annotation
6. Error: `unfold` on a non-existent function
7. Error: `assert` with non-boolean expression
8. Error: proof block in struct invariant (not allowed)
9. Existing tests continue to pass (proof hints are optional)

## Key Files Summary

| File | Change |
|------|--------|
| `.../legacy-move-compiler/src/parser/ast.rs` | Add `ProofHint_` enum, `Proof` variant |
| `.../legacy-move-compiler/src/parser/syntax.rs` | Parse `proof { ... }` blocks |
| `.../move-model/src/ast.rs` | Add `ProofHint` enum, `proof_hints` to `Spec` |
| `.../move-model/src/builder/module_builder.rs` | Translate parsed hints to model |
| `.../move-compiler-v2/src/env_pipeline/spec_checker.rs` | Check hint pureness |
| `.../bytecode-pipeline/src/proof_hint_processor.rs` | **NEW** — process hints |
| `.../bytecode-pipeline/src/pipeline_factory.rs` | Register processor |
| `.../bytecode-pipeline/src/lib.rs` | Add module |
| `.../boogie-backend/src/bytecode_translator.rs` | Emit unfold assumptions |
| `.../move-model/src/sourcifier.rs` | Render proof hints to source |
| `.../tests/sources/functional/proof_hints.move` | **NEW** — test cases |

## Verification

1. `cargo check -p move-compiler-v2 -p move-model -p move-prover-bytecode-pipeline -p move-prover-boogie-backend -p move-prover` — compilation
2. `cargo test -p move-prover` — all existing tests pass
3. New test with `--keep` flag — inspect generated Boogie for correct unfold
   assumptions and lemma assertions
4. Manual test: write a spec that fails without proof hints, succeeds with them

## Soundness Notes

- `unfold`: Adds the true definition of a function — sound by construction
- `assert`: Must be proved by the solver — sound (adds a proof obligation)
- `use`: Instantiates an existing axiom — sound (universal instantiation)
- `assume [trusted]`: **NOT sound** — deliberately so. Gated behind `[trusted]`
  annotation and always emits a warning. Intended for exploration (testing
  whether an assumption makes a proof go through). Production code should
  replace `assume` with `assert` once the lemma is identified
