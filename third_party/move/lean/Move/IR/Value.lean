-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

/-!
# IR Values

Runtime values of the bytecode language: the reference-free, monomorphic
fragment of Move that is the source of the translation
(see TACAS'22 §3.1 — after reference elimination, programs compute on plain
values only).

Current restrictions: a single integer width (`u64`), and structs as plain
field lists (struct types are erased after monomorphization; resource types
appear only as `ResourceId` indices of global memory).  Vectors are element
lists; the other integer widths are later refinements.

This file also defines the *specification-level* value domain `SVal`:
specification arithmetic in the Move Prover is over the unbounded integer
type `num`, so spec values replace `u64` by `Int`, with the embedding
`Value.toSVal`.
-/

namespace Move.IR

/-- Account addresses (Move's 256-bit addresses, abstracted to `Nat`). -/
abbrev Address := Nat

/-- A monomorphized resource type, e.g. `Account` or `Coin<USD>`.  After
monomorphization these are plain identifiers indexing global memory. -/
abbrev ResourceId := Nat

/-- Index of a function local (`TempIndex` in the Rust stackless
bytecode). -/
abbrev LocalIndex := Nat

/-- The root location of a reference: a local of the current frame or a
global resource. -/
inductive RefRoot where
  | loc (x : LocalIndex)
  | global (r : ResourceId) (a : Address)
  deriving BEq, Repr

/-- A runtime reference: a root location plus a path of offsets — the borrow
chain `borrow_loc`/`borrow_global` followed by `borrow_field`s and
`borrow_vec_elem`s (a path element is a field offset into a struct or an
element index into a vector, disambiguated by the value it traverses).
References exist only in the locals of a frame during execution; they are
never stored in global memory, passed at verified function boundaries, or
seen by the specification language. -/
structure RefTarget where
  root : RefRoot
  path : List Nat
  deriving BEq, Repr

/-- Does a path residue match an edge pattern?  `some i` requires the
exact offset, `none` is the dynamic-index wildcard (MVP's `-1` edge
pattern); the lengths must agree. -/
def pathMatches : List (Option Nat) → List Nat → Bool
  | [], [] => true
  | some i :: pat, j :: p => i == j && pathMatches pat p
  | none :: pat, _ :: p => pathMatches pat p
  | _, _ => false

/-- `isParentTarget pat tp tc`: location `tc` was derived from `tp` along
an edge matching `pat` — same root, and `tc`'s path extends `tp`'s by a
residue matching the pattern (Boogie's `$IsParentMutation`/
`$IsParentMutationHyper`; the empty pattern is `$IsSameMutation`). -/
def isParentTarget (pat : List (Option Nat)) (tp tc : RefTarget) : Bool :=
  tp.root == tc.root &&
  tc.path.take tp.path.length == tp.path &&
  pathMatches pat (tc.path.drop tp.path.length)

/-- The number of `u64` values; arithmetic producing a result `≥ U64_SIZE`
aborts (Move aborts on arithmetic overflow). -/
def U64_SIZE : Nat := 2 ^ 64

/-- IR runtime values.  `ref` values arise only from the borrow
instructions (see `RefTarget`).  `mut` values are the *mutation* datum of
the Move Prover's reference elimination (Boogie's
`$Mutation(l: $Location, p: Vec int, v: T)`, TACAS'22 §3.1's `Mut<T>`): a
location — reusing `RefTarget` — together with the checked-out value it
carries during a read-update-write cycle.  They arise only from the
mutation operations of eliminated code, never from source programs. -/
inductive Value where
  | u64 (n : Nat)
  | bool (b : Bool)
  | address (a : Address)
  | struct (fields : List Value)
  | vector (elems : List Value)
  | ref (t : RefTarget)
  | mut (t : RefTarget) (v : Value)
  deriving BEq, Repr

namespace Value

mutual

/-- No reference (or mutation) occurs in the value.  The Move type system
keeps references out of structs, vectors, global memory, and constants;
operations that store or build values are stuck on offending payloads. -/
@[simp] def refFree : Value → Bool
  | .u64 _ | .bool _ | .address _ => true
  | .struct fs => refFreeList fs
  | .vector es => refFreeList es
  | .ref _ => false
  | .mut _ _ => false

/-- No reference occurs in any of the values. -/
@[simp] def refFreeList : List Value → Bool
  | [] => true
  | v :: vs => refFree v && refFreeList vs

end

/-- Resolve a top-level reference through `deref`; a mutation is its
carried value; any other value is itself.  Move's `==` compares the values
references refer to. -/
def derefWith (deref : RefTarget → Option Value) : Value → Option Value
  | .ref t => deref t
  | .mut _ v => some v
  | v => some v

/-- Follow a path through a value (field offsets into structs, element
indices into vectors). -/
def getPath : Value → List Nat → Option Value
  | v, [] => some v
  | .struct fs, i :: p => (fs[i]?).bind (getPath · p)
  | .vector es, i :: p => (es[i]?).bind (getPath · p)
  | _, _ :: _ => none

/-- Functionally update the value at a path (`none` if the path does not
exist). -/
def setPath : Value → List Nat → Value → Option Value
  | _, [], v => some v
  | .struct fs, i :: p, v =>
      (fs[i]?).bind fun f =>
        (setPath f p v).map fun f' => Value.struct (fs.set i f')
  | .vector es, i :: p, v =>
      (es[i]?).bind fun e =>
        (setPath e p v).map fun e' => Value.vector (es.set i e')
  | _, _ :: _, _ => none

end Value

/-- Specification-level values.  Spec arithmetic in the Move Prover is over
the unbounded `num` type, so integers are `Int` here; `bool`, `address`,
`struct` and `vector` mirror the runtime shapes.  Runtime values embed via
`Value.toSVal`. -/
inductive SVal where
  | int (i : Int)
  | bool (b : Bool)
  | address (a : Address)
  | struct (fields : List SVal)
  | vector (elems : List SVal)
  | ref (t : RefTarget)
  | mut (t : RefTarget) (sv : SVal)

/-- Embed a runtime value into the specification domain (`u64 n` ↦ the
unbounded integer `n`).  References and mutations embed *opaquely* (their
own constructors), so a specification-level integer or boolean pins the
underlying runtime shape — the loop typed havoc and the invariant assumes
rely on this to exclude mutation values from value-typed locals. -/
@[simp] def Value.toSVal : Value → SVal
  | .u64 n => .int n
  | .bool b => .bool b
  | .address a => .address a
  | .struct fs => .struct (fs.map Value.toSVal)
  | .vector es => .vector (es.map Value.toSVal)
  | .ref t => .ref t
  | .mut t v => .mut t v.toSVal

end Move.IR
