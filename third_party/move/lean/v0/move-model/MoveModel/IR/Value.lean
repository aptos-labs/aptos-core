-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

/-!
# IR Values

This module defines runtime values for the supported Move fragment. After
reference elimination, translated programs compute on the
plain-value portion of this domain (TACAS'22 §3.1).

Integer values are signed unbounded integers, as in the Move Prover's Boogie
model: one `int` carrier for every integer width, constrained by the validity
predicates of `ValueTyping.lean` (`IsValid` at `.uint w` bounds the carrier
to `[0, w.size)`, the `$IsValid'u64'` discipline). Widths appear in types and
in the operations whose semantics needs a bound; `u64` abbreviations keep the
dominant width convenient. Struct values are field lists because Move erases
their type information from values; global storage keys retain a resource
declaration and its structural type-argument tags. Vector values are element
lists.

The module also defines the specification domain `SVal`.  Move specifications
use the unbounded integer type `num`, so `SVal` replaces runtime `u64` values
with `Int`.  `Value.toSVal` embeds runtime values into this domain.
-/

namespace MoveModel.IR

/-- Account addresses (Move's 256-bit addresses, abstracted to `Nat`). -/
abbrev Address := Nat

/-- The widths of Move's unsigned integer types. -/
inductive IntWidth where
  | w8 | w16 | w32 | w64 | w128 | w256
  deriving BEq, DecidableEq, ReflBEq, LawfulBEq, Ord, Repr

/-- The number of bits of an integer width. -/
def IntWidth.bits : IntWidth → Nat
  | .w8 => 8
  | .w16 => 16
  | .w32 => 32
  | .w64 => 64
  | .w128 => 128
  | .w256 => 256

/-- The number of values of an integer width; checked arithmetic producing a
result `≥ w.size` aborts (Move aborts on arithmetic overflow). -/
def IntWidth.size (w : IntWidth) : Nat := 2 ^ w.bits

/-- Every width admits values. -/
theorem IntWidth.size_pos (w : IntWidth) : 0 < w.size :=
  Nat.two_pow_pos w.bits

/-- Every width admits at least the values zero and one. -/
theorem IntWidth.one_lt_size (w : IntWidth) : 1 < w.size :=
  Nat.one_lt_two_pow_iff.mpr (by cases w <;> simp [IntWidth.bits])

/-- Half the value count, `2^(bits-1)`.  The signed range of the width is the
two's-complement interval `-halfSize ≤ i < halfSize` (i.e. `-2^(bits-1)` up to
`2^(bits-1) - 1`). -/
def IntWidth.halfSize (w : IntWidth) : Nat := 2 ^ (w.bits - 1)

/-- Every width admits signed values. -/
theorem IntWidth.halfSize_pos (w : IntWidth) : 0 < w.halfSize :=
  Nat.two_pow_pos _

/-- Every width's signed range spans more than one value. -/
theorem IntWidth.one_lt_halfSize (w : IntWidth) : 1 < w.halfSize := by
  cases w <;> decide

/-- Two halves make the whole: `2 * halfSize = size`. -/
theorem IntWidth.two_mul_halfSize (w : IntWidth) : 2 * w.halfSize = w.size := by
  cases w <;> decide

/-- The signed upper bound is below the unsigned cardinality. -/
theorem IntWidth.halfSize_lt_size (w : IntWidth) : w.halfSize < w.size := by
  have := w.two_mul_halfSize; have := w.halfSize_pos; omega

/-- The two's-complement bit pattern of a signed value `i ∈ [-halfSize, halfSize)`,
returned as an unsigned magnitude in `[0, size)`. -/
def IntWidth.toBits (w : IntWidth) (i : Int) : Int :=
  if i < 0 then i + w.size else i

/-- Interpret an unsigned bit pattern `u ∈ [0, size)` as the two's-complement
signed value it denotes, in `[-halfSize, halfSize)` (`ofBits ∘ toBits = id`). -/
def IntWidth.ofBits (w : IntWidth) (u : Int) : Int :=
  if u < (w.halfSize : Int) then u else u - w.size

/-- The cardinality is twice the half, over `Int`. -/
theorem IntWidth.size_int (w : IntWidth) : (w.size : Int) = 2 * (w.halfSize : Int) := by
  have := w.two_mul_halfSize; omega

/-- The two's-complement bit pattern of an in-range signed value is a valid
unsigned magnitude. -/
theorem IntWidth.toBits_mem (w : IntWidth) {i : Int}
    (hlo : -(w.halfSize : Int) ≤ i) (hhi : i < (w.halfSize : Int)) :
    0 ≤ w.toBits i ∧ w.toBits i < (w.size : Int) := by
  have hs := w.size_int; unfold IntWidth.toBits; split <;> omega

/-- Reinterpreting a valid unsigned bit pattern lands in the signed range. -/
theorem IntWidth.ofBits_mem (w : IntWidth) {u : Int}
    (h0 : 0 ≤ u) (hlt : u < (w.size : Int)) :
    -(w.halfSize : Int) ≤ w.ofBits u ∧ w.ofBits u < (w.halfSize : Int) := by
  have hs := w.size_int; unfold IntWidth.ofBits; split <;> omega

/-- Truncated remainder by an in-range nonzero divisor stays in the signed
range (`|i.tmod j| < |j| ≤ halfSize`). -/
theorem IntWidth.tmod_mem (w : IntWidth) {i j : Int}
    (hjlo : -(w.halfSize : Int) ≤ j) (hjhi : j < (w.halfSize : Int)) (hj : j ≠ 0) :
    -(w.halfSize : Int) ≤ i.tmod j ∧ i.tmod j < (w.halfSize : Int) := by
  have hb : (i.tmod j).natAbs < j.natAbs := by
    rw [Int.natAbs_tmod]; exact Nat.mod_lt _ (Int.natAbs_pos.mpr hj)
  have hjn : j.natAbs ≤ w.halfSize := by omega
  have hbound : (i.tmod j).natAbs < w.halfSize := Nat.lt_of_lt_of_le hb hjn
  omega

/-- Wrap an arbitrary integer into the width's two's-complement signed range —
the signed analogue of `n % size` for unsigned literals and wrapping
arithmetic. -/
def IntWidth.wrapSigned (w : IntWidth) (n : Int) : Int :=
  w.ofBits (n % (w.size : Int))

/-- A wrapped value lies in the signed range. -/
theorem IntWidth.wrapSigned_mem (w : IntWidth) (n : Int) :
    -(w.halfSize : Int) ≤ w.wrapSigned n ∧ w.wrapSigned n < (w.halfSize : Int) := by
  have hpos : (0 : Int) < (w.size : Int) := by exact_mod_cast w.size_pos
  exact w.ofBits_mem (Int.emod_nonneg _ (by omega)) (Int.emod_lt_of_pos _ hpos)

/-- Wrapping is the identity on values already in the signed range. -/
theorem IntWidth.wrapSigned_of_mem (w : IntWidth) {n : Int}
    (hlo : -(w.halfSize : Int) ≤ n) (hhi : n < (w.halfSize : Int)) :
    w.wrapSigned n = n := by
  have hs := w.size_int
  have hpos : (0 : Int) < (w.size : Int) := by exact_mod_cast w.size_pos
  unfold IntWidth.wrapSigned IntWidth.ofBits
  by_cases h : 0 ≤ n
  · have hmod : n % (w.size : Int) = n := Int.emod_eq_of_lt h (by omega)
    rw [hmod]; simp [hhi]
  · have hmod : n % (w.size : Int) = n + w.size := by
      have hr := Int.add_emod_right n (w.size : Int)
      rw [Int.emod_eq_of_lt (by omega) (by omega)] at hr
      omega
    rw [hmod]; simp only [if_neg (show ¬ n + (w.size : Int) < (w.halfSize : Int) by omega)]
    omega

/-- Arithmetic (floor) division of an in-range signed value by a positive
divisor stays in the signed range. -/
theorem IntWidth.fdiv_mem (w : IntWidth) {i : Int}
    (hlo : -(w.halfSize : Int) ≤ i) (hhi : i < (w.halfSize : Int))
    {d : Int} (hd : 1 ≤ d) :
    -(w.halfSize : Int) ≤ i.fdiv d ∧ i.fdiv d < (w.halfSize : Int) := by
  have hd0 : (0 : Int) < d := by omega
  have hpos : (0 : Int) < (w.halfSize : Int) := by exact_mod_cast w.halfSize_pos
  have hmul : (w.halfSize : Int) ≤ (w.halfSize : Int) * d := by
    have hnn : 0 ≤ (w.halfSize : Int) * (d - 1) := Int.mul_nonneg (Int.le_of_lt hpos) (by omega)
    rw [Int.mul_sub, Int.mul_one] at hnn; omega
  have hfe : i.fdiv d = i / d := by rw [Int.fdiv_eq_ediv]; simp [Int.le_of_lt hd0]
  rw [hfe]
  refine ⟨?_, ?_⟩
  · rw [Int.le_ediv_iff_mul_le hd0, Int.neg_mul]; omega
  · rw [Int.ediv_lt_iff_lt_mul hd0]; omega

/-- A Move integer type: a width together with a signedness.  A value of type
`nt` is a mathematical integer confined to the range `[nt.lo, nt.hi)`; the two
signedness cases differ only in those bounds. -/
structure NumType where
  width : IntWidth
  signed : Bool
  deriving BEq, DecidableEq, ReflBEq, LawfulBEq, Ord, Repr

namespace NumType

/-- Unsigned / signed numeric types at each width, as named constants. -/
abbrev u8 : NumType := ⟨.w8, false⟩
abbrev u16 : NumType := ⟨.w16, false⟩
abbrev u32 : NumType := ⟨.w32, false⟩
abbrev u64 : NumType := ⟨.w64, false⟩
abbrev u128 : NumType := ⟨.w128, false⟩
abbrev u256 : NumType := ⟨.w256, false⟩
abbrev i8 : NumType := ⟨.w8, true⟩
abbrev i16 : NumType := ⟨.w16, true⟩
abbrev i32 : NumType := ⟨.w32, true⟩
abbrev i64 : NumType := ⟨.w64, true⟩
abbrev i128 : NumType := ⟨.w128, true⟩
abbrev i256 : NumType := ⟨.w256, true⟩

/-- Number of representable values (the modulus), `2 ^ bits`. -/
abbrev size (nt : NumType) : Nat := nt.width.size

/-- Inclusive lower bound: `0` when unsigned, `-2^(bits-1)` when signed. -/
def lo (nt : NumType) : Int := if nt.signed then -(nt.width.halfSize : Int) else 0

/-- Exclusive upper bound: `2^bits` when unsigned, `2^(bits-1)` when signed. -/
def hi (nt : NumType) : Int :=
  if nt.signed then (nt.width.halfSize : Int) else (nt.width.size : Int)

@[simp] theorem lo_unsigned {nt : NumType} (h : nt.signed = false) : nt.lo = 0 := by
  simp [lo, h]
@[simp] theorem hi_unsigned {nt : NumType} (h : nt.signed = false) :
    nt.hi = (nt.size : Int) := by simp [hi, h, size]
@[simp] theorem lo_signed {nt : NumType} (h : nt.signed = true) :
    nt.lo = -(nt.width.halfSize : Int) := by simp [lo, h]
@[simp] theorem hi_signed {nt : NumType} (h : nt.signed = true) :
    nt.hi = (nt.width.halfSize : Int) := by simp [hi, h]

theorem size_pos (nt : NumType) : 0 < nt.size := nt.width.size_pos

/-- The range has exactly `size` elements. -/
theorem hi_eq_lo_add_size (nt : NumType) : nt.hi = nt.lo + (nt.size : Int) := by
  have hs := nt.width.size_int; unfold lo hi; split <;> simp [size] <;> omega

theorem lo_nonpos (nt : NumType) : nt.lo ≤ 0 := by
  unfold lo; split
  · have := nt.width.halfSize_pos; omega
  · omega

theorem pos_lt_hi (nt : NumType) : 0 < nt.hi := by
  unfold hi; split
  · have := nt.width.halfSize_pos; exact_mod_cast this
  · have := nt.width.size_pos; exact_mod_cast this

/-- Wrap an arbitrary integer into the range (two's-complement / modular). -/
def wrap (nt : NumType) (n : Int) : Int := nt.lo + ((n - nt.lo) % (nt.size : Int))

theorem wrap_mem (nt : NumType) (n : Int) : nt.lo ≤ nt.wrap n ∧ nt.wrap n < nt.hi := by
  have hpos : (0 : Int) < (nt.size : Int) := by exact_mod_cast nt.size_pos
  have h0 : 0 ≤ (n - nt.lo) % (nt.size : Int) := Int.emod_nonneg _ (by omega)
  have h1 : (n - nt.lo) % (nt.size : Int) < (nt.size : Int) := Int.emod_lt_of_pos _ hpos
  have := nt.hi_eq_lo_add_size; unfold wrap; omega

theorem wrap_of_mem (nt : NumType) {n : Int} (hlo : nt.lo ≤ n) (hhi : n < nt.hi) :
    nt.wrap n = n := by
  have := nt.hi_eq_lo_add_size
  have hmod : (n - nt.lo) % (nt.size : Int) = n - nt.lo :=
    Int.emod_eq_of_lt (by omega) (by omega)
  unfold wrap; omega

/-- The two's-complement bit pattern, an unsigned magnitude in `[0, size)`. -/
def toBits (nt : NumType) (v : Int) : Int := v % (nt.size : Int)

/-- Interpret an unsigned bit pattern as the value in range it denotes. -/
def fromBits (nt : NumType) (u : Int) : Int := if u < nt.hi then u else u - (nt.size : Int)

theorem toBits_mem (nt : NumType) (v : Int) : 0 ≤ nt.toBits v ∧ nt.toBits v < (nt.size : Int) := by
  have hpos : (0 : Int) < (nt.size : Int) := by exact_mod_cast nt.size_pos
  exact ⟨Int.emod_nonneg _ (by omega), Int.emod_lt_of_pos _ hpos⟩

theorem fromBits_mem (nt : NumType) {u : Int} (h0 : 0 ≤ u) (hlt : u < (nt.size : Int)) :
    nt.lo ≤ nt.fromBits u ∧ nt.fromBits u < nt.hi := by
  have := nt.hi_eq_lo_add_size; have := nt.lo_nonpos; have := nt.pos_lt_hi
  unfold fromBits; split <;> omega

/-- Truncated remainder by an in-range nonzero divisor stays in range. -/
theorem tmod_mem (nt : NumType) {i j : Int}
    (hilo : nt.lo ≤ i) (hjlo : nt.lo ≤ j) (hjhi : j < nt.hi) (hj : j ≠ 0) :
    nt.lo ≤ i.tmod j ∧ i.tmod j < nt.hi := by
  have habs : (i.tmod j).natAbs < j.natAbs := by
    rw [Int.natAbs_tmod]; exact Nat.mod_lt _ (Int.natAbs_pos.mpr hj)
  have := nt.hi_eq_lo_add_size; have := nt.lo_nonpos; have := nt.pos_lt_hi
  by_cases hs : nt.signed
  · -- signed: lo = -hi; |tmod| < |j| ≤ hi
    simp only [lo_signed hs, hi_signed hs] at *; omega
  · -- unsigned: lo = 0; tmod has the sign of i (≥ 0), and < j < hi
    simp only [Bool.not_eq_true] at hs
    have hnn : 0 ≤ i.tmod j := by
      rw [lo_unsigned hs] at hilo; exact Int.tmod_nonneg _ hilo
    simp only [lo_unsigned hs] at *; omega

/-- Arithmetic (floor) division by a positive power of two stays in range. -/
theorem fdiv_mem (nt : NumType) {i : Int} (hlo : nt.lo ≤ i) (hhi : i < nt.hi)
    {d : Int} (hd : 1 ≤ d) : nt.lo ≤ i.fdiv d ∧ i.fdiv d < nt.hi := by
  have hd0 : (0 : Int) < d := by omega
  have hfe : i.fdiv d = i / d := by rw [Int.fdiv_eq_ediv]; simp [Int.le_of_lt hd0]
  have := nt.lo_nonpos; have := nt.pos_lt_hi
  have hlonp := nt.lo_nonpos
  have hhipos := nt.pos_lt_hi
  rw [hfe]
  refine ⟨?_, ?_⟩
  · rw [Int.le_ediv_iff_mul_le hd0]
    -- `lo * d ≤ lo ≤ i` since `lo ≤ 0` and `d ≥ 1`
    have hnn : 0 ≤ (-nt.lo) * (d - 1) := Int.mul_nonneg (by omega) (by omega)
    rw [Int.neg_mul, Int.mul_sub, Int.mul_one] at hnn
    omega
  · rw [Int.ediv_lt_iff_lt_mul hd0]
    -- `i < hi ≤ hi * d` since `hi > 0` and `d ≥ 1`
    have hnn : 0 ≤ nt.hi * (d - 1) := Int.mul_nonneg (by omega) (by omega)
    rw [Int.mul_sub, Int.mul_one] at hnn
    omega

end NumType

/-- A resource declaration identifier, e.g. the declaration shared by all
instantiations of `Coin<T>`. -/
abbrev ResourceId := Nat

/-- One token in the prefix encoding of a closed Move type.  Constructors
which contain other types record their arity, making the encoding structural
and unambiguous.  It is deliberately separate from `Ty` to avoid an import
cycle between values and declaration typing. -/
inductive TypeTagToken where
  | bool | int (nt : NumType) | address | signer
  | typeParam (index : Nat)
  | struct (resource : ResourceId) (arity : Nat)
  | enum (resource : ResourceId) (arity : Nat)
  | vector | ref | mutRef
  deriving BEq, DecidableEq, ReflBEq, LawfulBEq, Ord, Repr

/-- Unsigned / signed integer tags, abbreviating the unified `int` token. -/
abbrev TypeTagToken.uint (w : IntWidth) : TypeTagToken := .int ⟨w, false⟩
abbrev TypeTagToken.sint (w : IntWidth) : TypeTagToken := .int ⟨w, true⟩

/-- The dominant integer width, abbreviated. -/
abbrev TypeTagToken.u64 : TypeTagToken := .uint .w64

/-- Collision-free prefix encoding of one Move type. -/
abbrev TypeTag := List TypeTagToken

/-- Global storage identity.  Move treats `R<A>` and `R<B>` as distinct
resource types even when they share the same declaration id. -/
structure ResourceKey where
  resource : ResourceId
  typeArgs : List TypeTag := []
  deriving BEq, DecidableEq, ReflBEq, LawfulBEq, Ord, Repr

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
  deriving BEq, DecidableEq, ReflBEq, LawfulBEq, Ord, Repr

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
  deriving BEq, DecidableEq, ReflBEq, LawfulBEq, Ord, Repr

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

/-- The number of `u64` values; vector lengths and cursor arithmetic are
bounded by it. -/
def U64_SIZE : Nat := 2 ^ 64

/-- `U64_SIZE` is the size of the dominant width. -/
theorem u64_size_eq : IntWidth.w64.size = U64_SIZE := rfl

/-- The dominant unsigned type's range, in the `U64_SIZE` spelling proofs use. -/
@[simp] theorem NumType.u64_size : NumType.u64.size = U64_SIZE := rfl
@[simp] theorem NumType.u64_lo : NumType.u64.lo = 0 := rfl
@[simp] theorem NumType.u64_hi : NumType.u64.hi = (U64_SIZE : Int) := rfl

/-- IR runtime values.  `ref` values arise only from the borrow
instructions (see `RefTarget`).  `mut` values are the *mutation* datum of
the Move Prover's reference elimination (Boogie's
`$Mutation(l: $Location, p: Vec int, v: T)`, TACAS'22 §3.1's `Mut<T>`): a
location — reusing `RefTarget` — together with the checked-out value it
carries during a read-update-write cycle.  They arise only from the
mutation operations of eliminated code, never from source programs. -/
inductive Value where
  | int (i : Int)
  | bool (b : Bool)
  | address (a : Address)
  | struct (fields : List Value)
  | variant (tag : Nat) (fields : List Value)
  | vector (elems : List Value)
  | ref (t : RefTarget)
  | mut (t : RefTarget) (v : Value)
  deriving BEq, Ord, Repr

namespace Value

/-- A nonnegative integer value, as written by `u64`-typed code. -/
abbrev u64 (n : Nat) : Value := .int n

mutual
  /-- Whether two reference-free values have compatible erased runtime type
  shapes.  This rejects observable heterogeneous equality (for example `u64`
  versus `bool`) in ill-typed configurations.  Empty vectors are compatible
  with any vector because their element type is erased from runtime values;
  well-typed IR supplies the stronger static guarantee. -/
  @[simp] def sameTypeShape : Value → Value → Bool
    | .int _, .int _ | .bool _, .bool _ | .address _, .address _ => true
    | .struct xs, .struct ys => sameTypeShapeList xs ys
    | .variant tx xs, .variant ty ys =>
        if tx == ty then sameTypeShapeList xs ys else true
    | .vector [], .vector _ | .vector _, .vector [] => true
    | .vector (x :: _), .vector (y :: _) => sameTypeShape x y
    | _, _ => false

  @[simp] def sameTypeShapeList : List Value → List Value → Bool
    | [], [] => true
    | x :: xs, y :: ys => sameTypeShape x y && sameTypeShapeList xs ys
    | _, _ => false
end

mutual

/-- No reference (or mutation) occurs in the value.  The Move type system
keeps references out of structs, vectors, global memory, and constants;
operations that store or build values are stuck on offending payloads. -/
@[simp] def refFree : Value → Bool
  | .int _ | .bool _ | .address _ => true
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
@[simp] def derefWith (deref : RefTarget → Option Value) : Value → Option Value
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

/-- Embed a runtime value into the specification domain.  Runtime integers
are already the unbounded integers of Move's `num`, so they embed as
themselves.  References and mutations embed *opaquely* (their
own constructors), so a specification-level integer or boolean pins the
underlying runtime shape — the loop typed havoc and the invariant assumes
rely on this to exclude mutation values from value-typed locals. -/
@[simp] def Value.toSVal : Value → SVal
  | .int i => .int i
  | .bool b => .bool b
  | .address a => .address a
  | .struct fs => .struct (fs.map Value.toSVal)
  | .variant tag fs => .variant tag (fs.map Value.toSVal)
  | .vector es => .vector (es.map Value.toSVal)
  | .ref t => .ref t
  | .mut t v => .mut t v.toSVal

end MoveModel.IR
