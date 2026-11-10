-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Semantics.Trace
import Spec.Properties.Safety

/-!
# Who may create, consume and take apart values; native contracts for D2, D3

TRUSTED. The permissions that D2 and D3 check on every ghost event, and the
contract natives must meet for D2 and D3 (plan §5.2, "Natives"; §16,
question 2).

**Permissions.** The language gives the privileges over a struct or enum type
to its defining module: values "can only be created ("packed"), destroyed
("unpacked") inside the module that defines the struct" (book,
`structs-and-enums.md`, "Privileged Struct Operations"), and "you are only
able to deconstruct a resource within the module in which it is defined"
(book, "Ownership"). A native function declared in a module is code of that
module (book, `functions.md`, "Native Functions": its "body [is] provided by
the VM"), so it has the module's privileges. Vectors are built-in and
anyone may build or take them apart. Signers "cannot be created via literals
or instructions — only by the Move VM" (book, `primitive-types.md`).

**Exceptions** (`NativePolicy`). Some natives deliberately create values of
other modules' types (a deserializer, a formatter producing a `String`),
create signers (an account module's `create_signer`), or destroy values they
have no privileges over (`table::drop_unchecked_box`, below). Each is an
explicit exception, and D2 and D3 are stated modulo them (plan §16, question
2). The default policy grants none.

**Native contract** (`NativesRespectOwnership`). A native is a relation on
values (`NativeRel`); its flow (`NativeFlows`) says which output nodes are
nodes of its arguments it passes on. For each call that returns, the contract
requires, besides conformance (`NativesRespect`):

- the flow is honest: a node passed on is unchanged (the same value);
- no argument node whose type lacks `copy` is passed on twice (D2a);
- every node the native creates is one it may create: a struct or enum value
  of its module's type or a type the policy grants it, a signer only as the
  policy allows (D3a, D3b);
- every argument node it does not pass on is consumed by it: legitimately if
  the node's type lacks `drop` (a vector, or a struct or enum value of a type
  the native has the privileges of, or a value it may destroy; D2b), and a
  struct or enum value is taken apart (some node inside it passed on) only
  with those privileges (D3a).
  What an immutable reference argument designates is not the native's: it is
  neither consumed nor moved (below).

**Generic natives.** A native is called with a function instance, its type
arguments included (`FunctionRef.typeArgs`), and creates struct values only of
types whose privileges it has, at any instance: privileges are the type's
module's (`NativePolicy.Privileged` compares modules). A native that creates
values of a type argument's type, as a deserializer does, needs an explicit
exception for each type it may create.

**Closures.** A native creates no closure (`Creator.MayCreateClosure`): a
closure over a function lets its holder run it, and only code from which the
function is visible may create one (D3c). A native may pass on a closure it
received, and may not consume one that lacks `drop`: taking a closure apart is
calling it, which a native's contract does not describe.

**Natives with references** (language-owner rulings, 2026-11-03;
`Spec.Semantics.ModeledNatives`). The contract describes the three shapes, not
each native. A native sees what its reference arguments designate
(`State.nativeView`), and its flow covers its outputs, its value results and
the values it writes back (`NativeFlows`):

- what an immutable reference argument designates stays where it is, as does
  what a mutable one designates unless the native writes it back
  (`Program.nativeKeeps`): the native may copy from it only what has `copy`,
  and consumes none of it;
- what a mutable reference argument of an update-shape native designates is
  replaced by the value written back: every node of it is passed on, into a
  result or a value written back, at most once unless it has `copy`, or
  consumed as the native may. `mem::swap` exchanges the two values with their
  identities, and
  `vector::move_range` moves elements, keeping their identities, from one
  vector into the other;
- a read-shape native writes nothing and returns values; it creates, like
  every native, only plain data, values of its own module's types (`Ordering`
  for `cmp::compare`), and what the policy grants.

Only an update-shape native writes, and only through its mutable arguments,
and the places a borrow-shape native returns lie inside its reference
arguments: the semantics allows nothing else (`Program.nativeResultFits`), and
`NativesRespect` asks it of every result.

**Inspection** (plan §16, question 2; language-owner ruling, 2026-11-04).
`bcs::to_bytes`, `bcs::serialized_size` and `cmp::compare` read values of any
type, other modules' structs included. Neither D3 nor the policy says anything
about it, and no exception is needed:

- D3 is about integrity, not confidentiality (ruling of 2026-11-04): who may
  create a value of a type, take one apart, and run the privileged operations
  on it (field borrows, `TestVariant`, global storage), not who may read it. A
  read-shape native does none of these to another module's type: it creates
  only what every native may, consumes nothing it saw through a reference, and
  hands out no reference into the value; its reads are `accessed` events, not
  `privileged` ones;
- the language makes every value observable this way: "every Move value must
  be serializable" (book, `references.md`), BCS "is the binary encoding for
  Move resources and other non-module values published on-chain"
  (`bcs.move`), and both natives are public with an unconstrained type
  parameter. Already `Eq` compares other modules' values, which D3 allows
  (plan §7, D3: "D3 does not restrict ... comparing values of `T` in other
  modules"), and the invariants of W2b are "insensitive to ... equality
  comparisons" (plan §7, W2b (v)). Reading changes nothing such an invariant
  depends on.

A native that *creates* or *takes apart* values of other modules' types
(deserialization, reflection) still needs an explicit exception, which the
policy grants (`NativePolicy.privileged`).

**Tables** (language-owner rulings, 2026-11-04). A table's entries are inside
it (`Value.table`), and a table native's flow covers them as any nodes of what
it sees: `add_box` passes the table and its entries on into the table it
writes back, with the new entry, and `remove_box` passes the removed entry on
into its result. The keys are no nodes: a key argument the native puts into
the table is consumed by it, which its `copy` and `drop` allow. The natives
have the privileges of `0x1::table`, and so of `Table` and `Box`.
`drop_unchecked_box`, a native on values, "consumes the table together with
its entries": D2 counts every node of them as consumed by the native, and for
a value of another module's type without `drop` inside an entry (a coin in a
`Table<address, Coin>`), the native policy must allow it
(`NativePolicy.destroys`). The spec does not require the table to be empty,
and `destroy_empty_box`, a read-shape native, promises nothing about
emptiness.

These are satisfiable (N3, `NativeContractsSatisfiable`): a native that only
fails meets them, and so do one that returns new plain data from plain data and
a read-shape native that returns new plain data.
-/

namespace MoveBytecode

/-- Explicit exceptions for natives (plan §16, question 2). -/
structure NativePolicy where
  /-- Native `f` has the privileges of struct or enum type `s`'s module,
  although declared elsewhere: it may create values of `s`, consume them and
  take them apart. -/
  privileged : FunctionRef → StructId → Bool := fun _ _ => false
  /-- Native `f`, called with `args`, may create a signer for address `a`. -/
  mintsSigner : FunctionRef → List Value → Address → Bool := fun _ _ _ => false
  /-- Native `f` may destroy a value of type `t`: consume it whole, passing on
  nothing inside it, although `t` lacks `drop` and `f` lacks the privileges of
  its module. `table::drop_unchecked_box` "consumes the table together with its
  entries" (language-owner ruling 3, 2026-11-04), whatever they hold. -/
  destroys : FunctionRef → RuntimeType → Bool := fun _ _ => false

namespace NativePolicy

/-- Native `f` of module `m` has the privileges of type `s`: `s` is defined
in `m`, or the policy grants them. -/
def Privileged (π : NativePolicy) (f : FunctionRef) (m : ModuleId) (s : StructId) : Bool :=
  s.module == m || π.privileged f s

end NativePolicy

namespace Creator

/-- Who may create a struct or enum value of type `s` (D3a): code of `s`'s
module (`Pack`, `PackVariant`), a native with the privileges of `s`, the
transaction for an accepted argument type (`A`, E1), or, for a value already
in global storage when the transaction starts, the earlier transactions that
stored it: D3 is judged relative to the initial storage, which earlier
executions of verified code produced. -/
def MayCreateStruct (π : NativePolicy) (A : List StructId) : Creator → StructId → Bool
  | .transaction, s => A.contains s
  | .storage, _ => true
  | .instruction m _, s => s.module == m
  | .native f m _, s => π.Privileged f m s

/-- Who may create a signer for address `a` (D3b): the transaction for an
authenticated address (E2), or a native the policy allows. No instruction,
and nothing in global storage: a signer "cannot be put into global storage"
(book, `generics-and-abilities.md`). -/
def MayCreateSigner (π : NativePolicy) (auth : List Address) : Creator → Address → Bool
  | .transaction, a => auth.contains a
  | .storage, _ => false
  | .instruction _ _, _ => false
  | .native f _ args, a => π.mintsSigner f args a

/-- Who may create a closure: code running `PackClosure`, which checks that
its target is visible from the code's module (D3c,
`Encapsulation.closuresRespected`), and global storage, for a closure stored
before the transaction. A transaction supplies no closure (E1: a function type
is no transaction argument type), and a native creates none: a closure over a
function lets its holder run the function, and only code that may call the
function may create one (plan §5.2, "Calls and visibility"). A native may
pass a closure it received on. -/
def MayCreateClosure : Creator → Bool
  | .instruction _ _ => true
  | .storage => true
  | .transaction => false
  | .native _ _ _ => false

/-- Whether `c` may create value `v` (its root node): D3a for struct and
enum values, D3b for signers, D3c for closures. Anyone may create plain data
and vectors. A reference creates no struct value or signer; who may create
references, and to what, is D4's (Phase 3, stage B). -/
def Permits (π : NativePolicy) (A : List StructId) (auth : List Address) (c : Creator) :
    Value → Bool
  | .signer a => c.MayCreateSigner π auth a
  | .struct s _ => c.MayCreateStruct π A s
  | .variant s _ _ => c.MayCreateStruct π A s
  | .bool _ => true
  | .int _ _ => true
  | .address _ => true
  | .vector _ _ => true
  | .ref _ => true
  | .closure .. => c.MayCreateClosure
  | .table s .. => c.MayCreateStruct π A s

end Creator

namespace Consumer

/-- Who may take a struct or enum value of type `s` apart (D3a): `Unpack` or
`UnpackVariant` in `s`'s module, or a native with the privileges of `s`. A
native that drops the whole value, passing nothing inside it on, does not
take it apart. `VecUnpack` never consumes a struct. -/
def MayTakeApart (π : NativePolicy) : Consumer → StructId → Bool
  | .unpack m, s => s.module == m
  | .vecUnpack, _ => false
  | .callClosure _, _ => false
  | .native f m whole, s => whole || π.Privileged f m s

/-- **Legitimate consumption** (plan §7, D2) of value `v`, which matters for
values without `drop`: `Unpack`/`UnpackVariant` of a struct or enum value in
its defining module, which transfers the fields; `VecUnpack` of a vector,
which transfers the elements; `CallClosure` of a closure, which transfers its
captured values to the target's parameters ("`CallClosure` on a moved
closure, which consumes it and transfers the captured values to the callee's
parameters", plan §7); or a native consuming a vector, or a struct or enum
value or a table of a type it has the privileges of, or destroying, whole, a
value of a type the policy lets it destroy (`NativePolicy.destroys`). -/
def Legitimate (π : NativePolicy) : Consumer → Value → Bool
  | .unpack m, .struct s _ => s.module == m
  | .unpack m, .variant s _ _ => s.module == m
  | .unpack m, .table s .. => s.module == m
  | .unpack _, _ => false
  | .vecUnpack, .vector _ _ => true
  | .vecUnpack, _ => false
  | .callClosure _, .closure .. => true
  | .callClosure _, _ => false
  | .native f m whole, v => (whole && π.destroys f v.type) || nativeLegitimate π f m v
where
  /-- A native's consumption with no exception: a vector, or a struct or enum
  value or a table of a type it has the privileges of. -/
  nativeLegitimate (π : NativePolicy) (f : FunctionRef) (m : ModuleId) : Value → Bool
    | .vector _ _ => true
    | .struct s _ => π.Privileged f m s
    | .variant s _ _ => π.Privileged f m s
    | .table s .. => π.Privileged f m s
    | _ => false

/-- Whether `c` may consume value `v` (its root node): legitimately if `v`'s
type lacks `drop` (D2b), and, for a struct or enum value, only if `c` may
take it apart (D3a). -/
def Permits (P : Program) (π : NativePolicy) (c : Consumer) (v : Value) : Bool :=
  ((v.type.abilities P).drop || c.Legitimate π v) &&
    match v.type with
    | .struct s => c.MayTakeApart π s
    | _ => true

end Consumer

/-- One native call respects ownership and encapsulation: native `f` of
module `m`, which saw its arguments as `args` (`State.nativeView`), produces
the outputs `vals` (its value results, then the values it writes back) with
flow `φ`, and

- every output node is either the node of what it saw that its flow names,
  unchanged, or created by the native as it may (`Creator.Permits`; the
  accepted argument types and authenticated addresses concern only the
  transaction and are empty here);
- of a value argument, or of what a mutable reference argument designates, no
  node whose type lacks `copy` is passed on more than once, and every node not
  passed on is consumed as the native may (`Consumer.Permits`): the value
  written back replaces what the mutable argument designated;
- of what any other reference argument designates, which stays where it is
  (an immutable one, or a mutable one the native does not write back,
  `Program.nativeKeeps`), a node whose type lacks `copy` is not passed on at
  all (that would copy it), and nothing is consumed.

Which arguments are references is read from `f`'s parameter types, which the
arguments conform to. For a native that takes values only, this is what it was
before. -/
def NativeCallRespects (P : Program) (π : NativePolicy) (f : FunctionRef) (m : ModuleId)
    (args vals : List Value) (φ : Nat → Path → Origin) : Bool :=
  let origins := passedOn φ vals
  let creator := Creator.native f m args
  let shared (k : Nat) : Bool := ((P.paramTypes f).bind (·[k]?)).any (P.nativeKeeps f)
  (nodesOf vals).all (fun (j, p, v) =>
    match φ j p with
    | .arg k q => (args[k]?.bind (·.at? q)) == some v
    | .fresh => creator.Permits π [] [] v) &&
  (nodesOf args).all (fun (k, q, w) =>
    if shared k then (w.type.abilities P).copy || !origins.contains (k, q)
    else
      ((w.type.abilities P).copy || origins.count (k, q) ≤ 1) &&
      (origins.contains (k, q) ||
        (Consumer.native f m (keepsNothingInside origins k q)).Permits P π w))

/-- **Native contract for D2 and D3.** Every call of a native that returns is
explained by its flow `ν` over its outputs, its value results then the values
it writes back, and respects ownership and encapsulation under policy `π`
(`NativeCallRespects`). The places a borrow-shape native returns create
references, not values: they are `NativesRespect`'s. -/
def NativesRespectOwnership (P : Program) (N : NativeRel) (ν : NativeFlows) (π : NativePolicy) :
    Prop :=
  ∀ f args vals writes places, N f args (.returned vals writes places) →
    ∃ info, P.function? f = some info ∧
      NativeCallRespects P π f info.moduleId args (vals ++ writes) (ν f args (vals ++ writes)) = true

end MoveBytecode
