# Lean on Move: AI-assisted smart-contract verification

Smart-contract verification has an unusual cost profile. The properties we
want are often easy to state—funds are conserved, an index stays in bounds, a
map remains ordered—but turning those properties into machine-checked proofs
requires specialist time. At the same time, AI coding systems have become
increasingly effective at Lean: they can search libraries, propose invariants,
respond to proof-state feedback, and iterate until the Lean kernel accepts a
proof.

Lean on Move explores a direct way to use that capability. A contract is
authored as a Move-like program in Lean, its functional specification is
written beside it, and an AI assistant can develop the proof in the same
source environment. The AI is not part of the trusted computing base. It may
suggest an incorrect proof, but it cannot make Lean accept one: the final
theorem is checked by Lean's small kernel.

This is different from translating Move into an unrelated verification
language and asking an AI to reason about the translation. The developer, the
AI assistant, and Lean all see the same source declarations and specifications.
That shortens the feedback loop and makes existing Lean libraries, tactics,
and theorem-proving tools directly available.

## Why Move fits naturally in Lean

Lean is not only a theorem prover; it also has a typed metaprogramming and
notation system. We use it to represent Move idioms directly rather than
embedding Move as strings or exposing a verbose syntax tree. For example:

- `fun` declares a Move function, while `def` remains available for Lean-only
  definitions and proof helpers;
- `Action T` represents an effectful Move computation;
- `&T`, `&mut T`, `*reference`, and `reference := value` retain familiar Move
  reference notation;
- `&mut Balance[addr].value` represents a global resource and nested field
  borrow;
- structures and enums derive Move abilities such as `Key`, `Copy`, `Drop`,
  and `Store`;
- `spec` associates a declarative contract with a function, and `verify`
  produces a named Lean theorem.

These forms elaborate into ordinary typed Lean declarations while retaining
the syntax needed by the compiler and source-semantics generator. Unsupported
constructs are rejected explicitly instead of being silently reinterpreted.

## A small verified Move module

The following is accepted by the current prototype. The property is stronger
than simply restating the function body: regardless of its argument,
`finalize` cannot produce the intermediate `pending` state.

```lean
import Move

open Move
open scoped Move Move.Spec

move_module Approval where

  @[move_enum]
  inductive Decision where
    | pending
    | approved
    | rejected
    deriving Copy, Drop, Store

  fun finalize (accepted : Bool) : Decision :=
    if accepted then .approved else .rejected

  spec finalize (accepted : Bool) where
    ensures result ≠ .pending

  -- AI-generated proof, checked by the Lean kernel.
  verify finalize by
    intro accepted
    cases accepted <;> simp [finalize]
```

An AI assistant generated the proof by splitting the Boolean input and asking
Lean's simplifier to evaluate each branch. Lean accepts the proof only because
both resulting cases establish the contract. For larger examples the same
interaction scales to helper lemmas, induction hypotheses, data-structure
invariants, and arithmetic side conditions. The checked-in
[ordered-map benchmark](../Tests/Move/OrderedMap.lean), for example, proves the
correctness of its recursive binary search against the authored function body.

Proof declarations are not compiled into the Move module. They establish
properties in Lean, while the Move functions and types remain the executable
program.

## From Lean source to Move bytecode

The implementation has two deliberately separate paths:

```text
                             +-> generated source semantics
                             |   -> `spec` / `verify`
Lean-authored Move module ---+   -> kernel-checked Lean theorem
                             |
                             +-> typed base LCNF
                                 -> named LIR
                                 -> MoveModel.IR
                                 -> versioned XIR
                                 -> compiler-v2 stackless model
                                 -> compiler-v2 transformations
                                 -> Move bytecode + production verifier
```

The detailed architecture and developer reference are in the
[Move README](README.md).

Lean's compiler exposes typed base LCNF, which gives us a stable, explicit
representation of the selected declarations. We normalize the supported
first-order fragment into a named low-level IR (LIR), lower it into the
existing `MoveModel.IR`, and serialize a versioned XIR exchange format. The
Move compiler reads XIR into its model with stackless bytecode, then runs the
normal compiler-v2 transformation, optimization, bytecode generation, and
bytecode-verification pipeline. This preserves important compiler features
instead of bypassing them with a custom bytecode assembler. The
[ordered-map transactional test](../../move-compiler-v2/transactional-tests/tests/leaner/ordered_map.lean)
exercises this complete path on the MoveVM.

Verification does not go through XIR. It operates directly on relational
semantics generated from the retained source body, including Move aborts,
checked arithmetic, resource access, vectors, calls, and references. This
separation makes the claims precise: Lean currently checks source-level
functional correctness, and the production bytecode verifier checks bytecode
safety. A semantic-preservation theorem connecting the source theorem to the
emitted bytecode is still required for a complete end-to-end certificate.

## Current scope and restrictions

This is a working prototype, not yet a replacement for the full Move language.
Its current boundary is intentional:

- there is no general `while`, `for`, or arbitrary loop construct;
- the stack-safe loop mechanism is explicit tail recursion, written
  `continue f args...`; the compiler checks that the call is in tail position
  and lowers it to parallel parameter assignments and a back edge;
- ordinary recursive function calls are supported, but non-tail recursion is
  not converted into a loop;
- source-level proofs cannot yet import modules authored in Move;
- higher-order functions, closures, recursive data types, and several complex
  nested-loan shapes remain outside the accepted compiler fragment;
- only `U64` currently has complete checked source arithmetic;
- the compiler-preservation proof from verified Lean source through LIR,
  Move IR, XIR, compiler v2, and final bytecode remains future work.

Even with these restrictions, the prototype is sufficient to test the core
hypothesis: Move can retain a concise, recognizable programming experience in
Lean, while developers gain access to AI-assisted, kernel-checked verification
in the language where the contract is authored.
