-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Properties.Entry
import Spec.Properties.Natives

/-!
# Non-vacuity (N2, N3)

TRUSTED. The safety theorems and the certificates' agreement statements
quantify over admissible initial states (`AdmissibleInitial`). A function with
no admissible initial state would be covered by none of them. N2 states that
every function they are meant to cover can be started (plan §7, group N).
N1 (`Progress`) is in `Spec.Properties.Safety`; N4 joins with the model's
acceptance of the framework.

Status: plan Phase 2a. With struct types, a parameter type can be uninhabited:
a struct that contains itself, an enum without variants, a native struct, a
struct with a field type outside the subset, or a type that does not resolve.
No program can call a function with such a parameter, so no theorem needs to
cover it, and N2 as first stated ("every function") would be false. N2 is
therefore premised on the parameter types being `Constructible`: having a
value built bottom-up from the declarations. Every primitive, signer and
vector type is constructible, so N2 still covers every function whose
parameters do not name structs, as in Phase 1.

The premise is independent of conformance. `Constructible` follows the
declarations (`Program.fieldTypes`, `Program.variantFieldTypes`), and N2
concludes that conforming arguments exist. A conformance relation that wrongly
rejected some well-built values would make N2 false, not vacuous. Conversely,
every value is a finite tree, so a value type with a conforming value is
constructible: among value types, the premise excludes exactly the
uninhabited ones.

Generic functions (Phase 4, stage A) are covered at every instance whose type
arguments satisfy their constraints (`Program.typeArgsOk`), which is every
instance a call can create; N2 is premised on it. An instance whose type
arguments violate a constraint has no initial state, and no program reaches
it: `CallGeneric` goes wrong first.

Reference types (Phase 3) are not `Constructible`: a reference argument
needs a value the transaction holds for it to designate (`initialState`), so
N2 covers functions with value parameters, and a function with a reference
parameter is covered when another function calls it, or, for `&signer`, by
N2 for transactions. N2 starts from empty global storage, which is well
typed.

Function types (Phase 4, stage B) are not `Constructible` either: a closure is
built by `PackClosure` only, from a function of the program, so N2 covers a
function with a function-typed parameter when another function calls it. A
type that holds a function type in a field is constructible through another
variant only.

D2 and D3 quantify over transactions' initial states (`EntryInitial`, E1 and
E2). N2 for transactions (`TransactionEntriesAdmissible`) states that a
transaction can call every entry function that returns nothing and whose
parameters are leading `signer` or `&signer` parameters followed by
constructible argument types, given an authenticated address when it takes a
signer.

N3 (plan §7): the native contracts are satisfiable, so the theorems that
assume them are not vacuous. A native relation and flows that meet every
contract exist, and they do not only fail: a native whose parameters and
results are plain data returns, at every instance where they are, generic
natives included. With natives with references (2026-11-03), N3 keeps every
clause, states the old ones over the new result form (no writes, no places),
and adds one: a read-shape native whose value parameters and results are plain
data returns on conforming arguments, whatever its reference arguments
designate (`bcs::to_bytes` of a struct of another module, say). The witness is
the old one extended: natives that fail, except those, which return a new
plain value of each result type (`false`, `0`, the empty vector) and consume
their plain value arguments; what their reference arguments designate is
neither consumed nor passed on. Every native call still has a result, so the
modeled natives too, on conforming arguments or not. The update and borrow
shapes are not asked to return: their natives (`mem::swap`, the vector
updates and borrows, `signer::borrow_address`) are exercised by the tests
with contract-respecting relations (`Tests/Semantics.lean`).

The table natives (2026-11-04) change no clause. `contains_box` and
`destroy_empty_box` are read-shape natives: at a plain key type the witness
returns `false` and nothing, whatever the table holds. `add_box`, `remove_box`,
`borrow_box` and `borrow_box_mut` are update and borrow shapes, and the tests
exercise them with contract-respecting relations. A table type is
constructible when its declared fields are: its value is the table with those
fields and no entries.
-/

namespace MoveBytecode

/-- Type `t` has a value built bottom-up from `P`'s declarations: a primitive
or signer type; any vector type, which has the empty vector; a struct type
whose declared field types are all constructible; or an enum type with a
variant whose field types are all constructible. No reference type and no
function type is constructible. -/
inductive Program.Constructible (P : Program) : RuntimeType → Prop where
  | bool : P.Constructible .bool
  | int (t : IntType) : P.Constructible (.int t)
  | address : P.Constructible .address
  | signer : P.Constructible .signer
  | vector (elem : RuntimeType) : P.Constructible (.vector elem)
  | struct (s : StructId) (tys : List RuntimeType) :
      P.fieldTypes s = some tys → (∀ t ∈ tys, P.Constructible t) → P.Constructible (.struct s)
  | variant (s : StructId) (tag : VariantIndex) (tys : List RuntimeType) :
      P.variantFieldTypes s tag = some tys → (∀ t ∈ tys, P.Constructible t) →
      P.Constructible (.struct s)

/-- **N2.** Every function instance `f` of the program that has code, whose
type arguments satisfy its constraints, whose parameter and local types are
in the subset (`P.localTypes f = some _`), and whose parameter types are
constructible, has an admissible initial state whose only frame runs `f` from
its first instruction. -/
def Program.EntriesAdmissible (P : Program) : Prop :=
  ∀ f localTys paramTys, P.typeArgsOk f = true →
    P.localTypes f = some localTys → P.paramTypes f = some paramTys →
    (∀ t ∈ paramTys, P.Constructible t) →
    ∃ s₀ fr, AdmissibleInitial P s₀ ∧ s₀.frames = [fr] ∧ fr.function = f ∧ fr.pc = 0

/-- **N2 for transactions.** Every instance `f` of an entry function with
code, with no return types, whose type arguments satisfy its constraints, and
whose parameters are leading signer parameters (`signer`, or `&signer` where
`byRef` says so) followed by argument types that are constructible, has a
transaction initial state (E1, E2) whose only frame runs `f` from its first
instruction, for any authenticated addresses `auth` (nonempty if `f` takes a
signer) and accepted struct types `A` that satisfy E1. -/
def Program.TransactionEntriesAdmissible (P : Program) : Prop :=
  ∀ (auth : List Address) (A : List StructId) (f : FunctionRef) (localTys : List RuntimeType)
    (byRef : List Bool) (tys : List RuntimeType),
    P.TransactionArgumentStructs A → P.EntryFunction f → P.returnTypes f = some [] →
    P.typeArgsOk f = true →
    P.localTypes f = some localTys →
    P.paramTypes f = some (byRef.map signerParam ++ tys) →
    (∀ t ∈ tys, t.IsTransactionArgument A = true ∧ P.Constructible t) → (byRef = [] ∨ auth ≠ []) →
    ∃ s₀ fr, EntryInitial P auth A s₀ ∧ s₀.frames = [fr] ∧ fr.function = f ∧ fr.pc = 0

/-- Plain data: booleans, integers, addresses, and vectors of plain data.
Every native may create and drop it. -/
def RuntimeType.IsPlain : RuntimeType → Bool
  | .bool => true
  | .int _ => true
  | .address => true
  | .vector elem => elem.IsPlain
  | .signer => false
  | .struct _ => false
  | .function .. => false
  | .ref _ _ => false

/-- The type of what a native sees of an argument of type `t`
(`State.nativeView`): the target of a reference type, otherwise `t` itself. -/
def RuntimeType.seen : RuntimeType → RuntimeType
  | .ref _ target => target
  | t => t

/-- **N3.** For every program and native policy, some natives and flows meet
every native contract (`NativesRespect`, `NativesRespectOwnership`), every
native call has a result, a native whose parameter and return types are plain
data returns on conforming arguments, and a native of the read shape
(`Spec.Semantics.ModeledNatives`) whose value parameters and results are plain
data returns on conforming arguments, as it sees them: whatever its reference
arguments designate. A native that only fails, and natives that return new
plain data, witness it. -/
def NativeContractsSatisfiable : Prop :=
  ∀ (P : Program) (π : NativePolicy), ∃ (N : NativeRel) (ν : NativeFlows),
    NativesRespect P N ∧ NativesRespectOwnership P N ν π ∧
    (∀ f args, ∃ r, N f args r) ∧
    (∀ f params returns args, P.paramTypes f = some params → P.returnTypes f = some returns →
      (∀ t ∈ params ++ returns, t.IsPlain = true) → conformsAll P args params = true →
      ∃ vals, N f args (.returned vals [] [])) ∧
    ∀ f params returns views, P.nativeShape? f = some .read → P.paramTypes f = some params →
      P.returnTypes f = some returns → (∀ t ∈ params, t.isValue = true → t.IsPlain = true) →
      (∀ t ∈ returns, t.IsPlain = true) → conformsAll P views (params.map RuntimeType.seen) = true →
      ∃ vals, N f views (.returned vals [] [])

end MoveBytecode
