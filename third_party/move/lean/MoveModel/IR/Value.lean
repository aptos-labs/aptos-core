-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

/-!
# IR Values

This module defines runtime values for the supported Move fragment. After
reference elimination, translated programs compute on the
plain-value portion of this domain (TACAS'22 §3.1).

The model currently has one integer width, `u64`. Struct values are field
lists because Move erases their type information from values; global storage
keys retain a resource declaration and its structural type-argument tags.
Vector values are element lists.

The module also defines the specification domain `SVal`.  Move specifications
use the unbounded integer type `num`, so `SVal` replaces runtime `u64` values
with `Int`.  `Value.toSVal` embeds runtime values into this domain.
-/

namespace MoveModel.IR

/-- Account addresses (Move's 256-bit addresses, abstracted to `Nat`). -/
abbrev Address := Nat

/-- A resource declaration identifier, e.g. the declaration shared by all
instantiations of `Coin<T>`. -/
abbrev ResourceId := Nat

/-- One token in the prefix encoding of a closed Move type.  Constructors
which contain other types record their arity, making the encoding structural
and unambiguous.  It is deliberately separate from `Ty` to avoid an import
cycle between values and declaration typing. -/
inductive TypeTagToken where
  | bool | u64 | address | signer
  | typeParam (index : Nat)
  | struct (resource : ResourceId) (arity : Nat)
  | enum (resource : ResourceId) (arity : Nat)
  | vector | ref | mutRef
  deriving BEq, DecidableEq, ReflBEq, LawfulBEq, Repr

/-- Collision-free prefix encoding of one Move type. -/
abbrev TypeTag := List TypeTagToken

/-- Global storage identity.  Move treats `R<A>` and `R<B>` as distinct
resource types even when they share the same declaration id. -/
structure ResourceKey where
  resource : ResourceId
  typeArgs : List TypeTag := []
  deriving BEq, DecidableEq, ReflBEq, LawfulBEq, Repr

/-- A nongeneric resource id denotes the empty instantiation. -/
instance : Coe ResourceId ResourceKey where
  coe resource := ⟨resource, []⟩

instance (n : Nat) : OfNat ResourceKey n where
  ofNat := ⟨n, []⟩

/-- Index of a function local (`TempIndex` in the Rust stackless
bytecode). -/
abbrev LocalIndex := Nat

/-- Identity of a live call frame.  The operational semantics uses the call
depth as the identity.  Borrow analysis guarantees that references into a
callee frame do not survive its return, so a depth may safely be reused by
the next call at that depth. -/
abbrev FrameId := Nat

/-- The root location of a reference: a frame-qualified local or a global
resource.  Instruction operands remain frame-relative `LocalIndex` values;
only references carry the frame identity needed to remain meaningful across
function calls. -/
inductive RefRoot where
  | loc (frame : FrameId) (x : LocalIndex)
  | global (r : ResourceKey) (a : Address)
  deriving BEq, DecidableEq, ReflBEq, LawfulBEq, Repr

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
  deriving BEq, DecidableEq, ReflBEq, LawfulBEq, Repr

/-- Does a path residue match an edge pattern?  `some i` requires the
exact offset, `none` is the dynamic-index wildcard (MVP's `-1` edge
pattern); the lengths must agree. -/
def pathMatches : List (Option Nat) → List Nat → Bool
  | [], [] => true
  | some i :: pat, j :: p => i == j && pathMatches pat p
  | none :: pat, _ :: p => pathMatches pat p
  | _, _ => false

/-- Only the empty concrete path matches the empty pattern. -/
theorem pathMatches_nil {path : List Nat}
    (h : pathMatches [] path = true) : path = [] := by
  cases path with
  | nil => rfl
  | cons _ _ => simp [pathMatches] at h

/-- `isParentTarget pat tp tc`: location `tc` was derived from `tp` along
an edge matching `pat` — same root, and `tc`'s path extends `tp`'s by a
residue matching the pattern (Boogie's `$IsParentMutation`/
`$IsParentMutationHyper`; the empty pattern is `$IsSameMutation`). -/
def isParentTarget (pat : List (Option Nat)) (tp tc : RefTarget) : Bool :=
  tp.root == tc.root &&
  tc.path.take tp.path.length == tp.path &&
  pathMatches pat (tc.path.drop tp.path.length)

/-- Decompose a successful parent-target test into root equality, an exact
path extension, and matching of the residual path. -/
theorem isParentTarget_parts {pat : List (Option Nat)} {tp tc : RefTarget}
    (h : isParentTarget pat tp tc = true) :
    tp.root = tc.root ∧
      tc.path = tp.path ++ tc.path.drop tp.path.length ∧
      pathMatches pat (tc.path.drop tp.path.length) = true := by
  simp only [isParentTarget, Bool.and_eq_true, beq_iff_eq] at h
  have hroot : tp.root = tc.root := by
    cases hrp : tp.root <;> cases hrc : tc.root <;> simp_all
  refine ⟨hroot, ?_, h.2⟩
  calc
    tc.path = tc.path.take tp.path.length ++
        tc.path.drop tp.path.length :=
      (List.take_append_drop tp.path.length tc.path).symm
    _ = tp.path ++ tc.path.drop tp.path.length := by rw [h.1.2]

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
  | variant (tag : Nat) (fields : List Value)
  | vector (elems : List Value)
  | ref (t : RefTarget)
  | mut (t : RefTarget) (v : Value)
  deriving BEq, Repr

namespace Value

mutual
  /-- Whether two reference-free values have compatible erased runtime type
  shapes.  This rejects observable heterogeneous equality (for example `u64`
  versus `bool`) in ill-typed configurations.  Empty vectors are compatible
  with any vector because their element type is erased from runtime values;
  well-typed IR supplies the stronger static guarantee. -/
  def sameTypeShape : Value → Value → Bool
    | .u64 _, .u64 _ | .bool _, .bool _ | .address _, .address _ => true
    | .struct xs, .struct ys => sameTypeShapeList xs ys
    | .variant tx xs, .variant ty ys => tx == ty && sameTypeShapeList xs ys
    | .vector [], .vector _ | .vector _, .vector [] => true
    | .vector (x :: _), .vector (y :: _) => sameTypeShape x y
    | _, _ => false

  def sameTypeShapeList : List Value → List Value → Bool
    | [], [] => true
    | x :: xs, y :: ys => sameTypeShape x y && sameTypeShapeList xs ys
    | _, _ => false
end

mutual

/-- No reference (or mutation) occurs in the value.  The Move type system
keeps references out of structs, vectors, global memory, and constants;
operations that store or build values are stuck on offending payloads. -/
@[simp] def refFree : Value → Bool
  | .u64 _ | .bool _ | .address _ => true
  | .struct fs => refFreeList fs
  | .variant _ fs => refFreeList fs
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
  | .variant _ fs, i :: p => (fs[i]?).bind (getPath · p)
  | .vector es, i :: p => (es[i]?).bind (getPath · p)
  | _, _ :: _ => none

/-- Functionally update the value at a path (`none` if the path does not
exist). -/
def setPath : Value → List Nat → Value → Option Value
  | _, [], v => some v
  | .struct fs, i :: p, v =>
      (fs[i]?).bind fun f =>
        (setPath f p v).map fun f' => Value.struct (fs.set i f')
  | .variant tag fs, i :: p, v =>
      (fs[i]?).bind fun f =>
        (setPath f p v).map fun f' => Value.variant tag (fs.set i f')
  | .vector es, i :: p, v =>
      (es[i]?).bind fun e =>
        (setPath e p v).map fun e' => Value.vector (es.set i e')
  | _, _ :: _, _ => none

/-- Reading a concatenated path is equivalent to reading each part in sequence. -/
theorem getPath_append : ∀ {p q : List Nat} {v : Value},
    v.getPath (p ++ q) = (v.getPath p).bind (·.getPath q)
  | [], q, v => by simp [getPath]
  | i :: p, q, v => by
      cases v <;> simp [getPath, getPath_append, Option.bind_assoc]

/-- Replacing one list element with a reference-free value preserves
pointwise reference-freedom. -/
theorem refFreeList_set {xs : List Value} {i : Nat} {v : Value}
    (hxs : refFreeList xs) (hv : v.refFree) :
    refFreeList (xs.set i v) := by
  induction xs generalizing i with
  | nil => simp
  | cons x xs ih =>
      cases i <;> simp_all [List.set, refFreeList]

/-- An element selected from a reference-free value list is reference-free. -/
theorem refFree_of_getElem? {vs : List Value} {i : Nat} {v : Value}
    (hfree : refFreeList vs) (hv : vs[i]? = some v) : v.refFree := by
  induction vs generalizing i with
  | nil => simp at hv
  | cons w ws ih =>
      simp only [refFreeList, Bool.and_eq_true] at hfree
      cases i with
      | zero =>
          simp at hv
          subst v
          exact hfree.1
      | succ i => exact ih hfree.2 (by simpa using hv)

/-- A list is reference-free exactly when each of its members is
reference-free. -/
theorem refFreeList_iff_forall {vs : List Value} :
    refFreeList vs ↔ ∀ v ∈ vs, v.refFree := by
  induction vs with
  | nil => simp
  | cons v vs ih => simp [ih]

/-- A reference-free value cannot be a runtime reference. -/
theorem refFree_ne_ref {v : Value} (h : v.refFree) (rt : RefTarget) :
    v ≠ .ref rt := by
  intro heq
  subst v
  simp at h

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
  | variant (tag : Nat) (fields : List SVal)
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
  | .variant tag fs => .variant tag (fs.map Value.toSVal)
  | .vector es => .vector (es.map Value.toSVal)
  | .ref t => .ref t
  | .mut t v => .mut t v.toSVal

end MoveModel.IR
