-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Properties.Safety

/-!
# Transaction-entry premises E1, E2

TRUSTED. The initial states of transactions (plan §7, group E). These are not
facts the verifier establishes: the transaction layer validates entry
arguments, and authentication supplies signers.

D1 is stated for every `AdmissibleInitial` state, a call of *any* function
with *any* conforming arguments, which is stronger than for transactions
only: type safety does not depend on where a value came from. D2 and D3 do. A
struct value of another module's type that no code of that module created, or
a signer for an address nobody authenticated, would falsify them at the first
state. They are therefore stated for `EntryInitial` states, which are
admissible (`EntryInitial.admissible`) but come from a transaction:

- **E1.** The function is an entry function that returns nothing, and its
  arguments are values the transaction layer constructs from the
  transaction's payload. "Entry functions must not have any return values"
  (book, `functions.md`): a transaction calls only entry functions that
  return nothing, and the transaction layer refuses any other before it runs
  (language-owner ruling 2026-11-03). "Entry
  functions can accept parameters of the following types: primitive types,
  references to a `signer`, vectors (where the element type is itself
  acceptable), and certain standard library types such as `String`, `Object`,
  and `Option`" (book, `functions.md`), and "`public` structs and enums can
  be passed as entry and view function arguments if the type has the `copy`
  ability and does not have the `key` ability. All field types must
  themselves be valid argument types, recursively" (book,
  `structs-and-enums.md`, "Transaction Arguments"). The `copy` requirement
  "ensures that no resource type can be constructed from transaction
  arguments" (book, `functions.md`).
- **E2.** Signers come only from authentication: signer values "cannot be
  created via literals or instructions — only by the Move VM", which creates
  them before running the code, and "all of the `signer` arguments must come
  first" (book, `primitive-types.md`, "Signer"). Each is for an address the
  transaction authenticated: its sender or a secondary signer. A leading
  parameter is `signer`, passed by value, or `&signer` ("references to a
  `signer`", book, `functions.md`): the transaction then holds the signer for
  the whole execution and passes a reference to it (`Root.held`,
  `State.held`).

Global storage at the start is any well-typed storage
(`GlobalStorage.wellTyped`): earlier transactions wrote it. Its values were
created before the transaction (`Creator.storage`).

Which struct types the transaction layer accepts is a parameter, `A`. Struct
visibility (`public`) has no field in the binary format, and the standard
library types the layer accepts are named by the layer, so the list cannot be
read off a module. E1 requires only what the book requires of every accepted
type. The authenticated addresses are a parameter too, `auth`.

**Generic entry functions** (plan Phase 4, stage A). The transaction names
the instance it calls: its type arguments are any closed types of the program
satisfying the entry function's constraints (`initialState`,
`Program.typeArgsOk`), as at any call site ("constraints are checked at call
sites", book, `generics-and-abilities.md`, "Verifying Constraints"). Type
arguments create no value, so they need no further restriction for D2 or D3.
The accepted struct types `A` are instances: a generic type the layer accepts
(`Object<T>`, `Option<T>`, book, `functions.md`) is listed at each instance it
accepts, and E1's rule applies to the instance's field types.

**Function values** are no transaction arguments: the book's list of entry
parameter types ("primitive types, references to a `signer`, vectors (where
the element type is itself acceptable), and certain standard library types",
book, `functions.md`) has no function type, so a transaction creates no
closure (D3c).

Status: plan Phase 4, stage B. Scripts (`CompiledScript`) join the programs
later, with the same premises.
-/

namespace MoveBytecode

/-- A type of the arguments a transaction supplies after the leading signers:
a primitive type, a vector of such a type, or an accepted struct or enum type
(`A`). A signer is not one, nor a reference: signers and references to them
come only first, from authentication (E2). A function type is not one. -/
def RuntimeType.IsTransactionArgument (A : List StructId) : RuntimeType → Bool
  | .bool => true
  | .int _ => true
  | .address => true
  | .signer => false
  | .vector elem => elem.IsTransactionArgument A
  | .struct s => A.contains s
  | .function .. => false
  | .ref _ _ => false

namespace Program

/-- Struct or enum type `s` may be accepted as a transaction argument (book,
"Transaction Arguments"): it has `copy` and not `key`, it declares fields or
variants (a native struct has no fields a transaction could supply), and
every field type of the struct, or of every variant of the enum, is itself an
argument type. -/
def ArgumentStruct (P : Program) (A : List StructId) (s : StructId) : Prop :=
  ((RuntimeType.struct s).abilities P).copy = true ∧
  ((RuntimeType.struct s).abilities P).key = false ∧
  ∃ info, P.structType? s = some info ∧
    match info.definition.fieldInformation with
    | .declared _ =>
      ∃ tys, P.fieldTypes s = some tys ∧ ∀ t ∈ tys, t.IsTransactionArgument A = true
    | .declaredVariants variants =>
      ∀ tag, tag < variants.length →
        ∃ tys, P.variantFieldTypes s tag = some tys ∧ ∀ t ∈ tys, t.IsTransactionArgument A = true
    | .native => False

/-- **E1** for the accepted struct types: every one satisfies the book's
rule. -/
def TransactionArgumentStructs (P : Program) (A : List StructId) : Prop :=
  ∀ s ∈ A, P.ArgumentStruct A s

/-- `f` is an entry function: "intended to be used as an entry point to
execution" (`file_format.rs`, `FunctionDefinition::ENTRY`). Its visibility
does not matter: "entry functions do not have to be public" (book,
`functions.md`). -/
def EntryFunction (P : Program) (f : FunctionRef) : Prop :=
  ∃ info, P.function? f = some info ∧ info.definition.isEntry = true

end Program

/-- The type of a leading signer parameter: `&signer` when `byRef`, `signer`
otherwise. -/
def signerParam (byRef : Bool) : RuntimeType :=
  if byRef then .ref false .signer else .signer

/-- The signers the transaction holds and lends by reference: those of the
`&signer` parameters, in order. -/
def heldSigners (signers : List (Address × Bool)) : List Value :=
  (signers.filter (·.2)).map fun (a, _) => .signer a

/-- The arguments of the leading signer parameters: a signer by value, or a
reference to the next held signer, counting from `k`. -/
def signerArgs : List (Address × Bool) → Nat → List Value
  | [], _ => []
  | (a, false) :: rest, k => .signer a :: signerArgs rest k
  | (_, true) :: rest, k => .ref ⟨false, .signer, ⟨.held k, []⟩, 0⟩ :: signerArgs rest (k + 1)

/-- **E1, E2.** The initial state of a transaction that calls entry function
`f`, over well-typed global storage: `f` has no return types (E1); its
parameters are some leading signer parameters (`signer` or `&signer`)
followed by argument types; the signers are for authenticated addresses
(`auth`, E2), and those passed by reference are held by the transaction; the
other arguments conform to their types (E1). -/
def EntryInitial (P : Program) (auth : List Address) (A : List StructId) (s₀ : State) : Prop :=
  P.TransactionArgumentStructs A ∧
  ∃ (f : FunctionRef) (signers : List (Address × Bool)) (tys : List RuntimeType)
    (args : List Value) (storage : GlobalStorage),
    P.EntryFunction f ∧
    P.returnTypes f = some [] ∧
    P.paramTypes f = some (signers.map (fun x => signerParam x.2) ++ tys) ∧
    (∀ x ∈ signers, x.1 ∈ auth) ∧
    (∀ t ∈ tys, t.IsTransactionArgument A = true) ∧
    conformsAll P args tys = true ∧
    storage.wellTyped P = true ∧
    initialState P f (signerArgs signers 0 ++ args) storage (heldSigners signers) = some s₀

theorem conformsAll_signerArgs (P : Program) (signers : List (Address × Bool)) (k : Nat)
    (args : List Value) (tys : List RuntimeType) (h : conformsAll P args tys = true) :
    conformsAll P (signerArgs signers k ++ args) (signers.map (fun x => signerParam x.2) ++ tys) =
      true := by
  induction signers generalizing k with
  | nil => simpa [signerArgs] using h
  | cons x rest ih =>
    obtain ⟨a, byRef⟩ := x
    cases byRef
    · have := ih k
      simp [signerArgs, signerParam, conformsAll, Value.type, Value.wellFormed,
        RuntimeType.assignable] at this ⊢
      exact this
    · have := ih (k + 1)
      simp [signerArgs, signerParam, conformsAll, Value.type, Value.wellFormed,
        RuntimeType.isValue, RuntimeType.assignable] at this ⊢
      exact this

/-- A transaction's initial state is admissible: E1 and E2 narrow D1's
premise, they do not change it. -/
theorem EntryInitial.admissible {P : Program} {auth : List Address} {A : List StructId}
    {s₀ : State} (h : EntryInitial P auth A s₀) : AdmissibleInitial P s₀ := by
  obtain ⟨_, f, signers, tys, args, storage, _, _, hparams, _, _, hargs, hstorage, hinit⟩ := h
  exact ⟨f, _, _, storage, _, hparams, conformsAll_signerArgs P signers 0 args tys hargs, hstorage,
    hinit⟩

end MoveBytecode
