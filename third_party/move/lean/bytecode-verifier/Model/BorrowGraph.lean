-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Model.Result

/-!
# The borrow graph

UNTRUSTED. Transcribes `move-borrow-graph` (`graph.rs`, `references.rs`,
`paths.rs`) as the reference-safety pass instantiates it: no locations, and
labels for locals, globals and fields.

A reference is borrowed by others along edges carrying a path of labels; a
strong edge borrows exactly that path, a weak one that path or anything
below it. Each parent keeps at most ten edges per child; inserting into a
full set collapses it to one weak edge with the empty path, which then
absorbs every later insertion.

That collapse makes the result depend on the order of insertions, so the
model keeps every map and set sorted as Rust's `BTreeMap` and `BTreeSet`
are, and iterates them in the same order. Rust's `unwrap`s and `assert!`s on
the graph are a `panic` result; `debug_assert!`s are not checked, as in a
release build.
-/

namespace MoveBytecode.Model

namespace BorrowGraph

/-- A label of a path: `Label` in `reference_safety/abstract_state.rs`, with
the derived order: locals, then globals, then fields. -/
inductive Label where
  | local_ (i : Nat)
  | global (s : Nat)
  | field (f : Nat)
  deriving DecidableEq, Repr, Inhabited

def Label.cmp : Label → Label → Ordering
  | .local_ a, .local_ b | .global a, .global b | .field a, .field b => compare a b
  | .local_ _, .global _ | .local_ _, .field _ | .global _, .field _ => .lt
  | .global _, .local_ _ | .field _, .local_ _ | .field _, .global _ => .gt

/-- The lexicographic order of `Vec<Label>`. -/
def pathCmp : List Label → List Label → Ordering
  | [], [] => .eq
  | [], _ :: _ => .lt
  | _ :: _, [] => .gt
  | a :: as, b :: bs => (a.cmp b).then (pathCmp as bs)

/-- `paths::leq`: `p` is a prefix of `q`. -/
def isPrefix : List Label → List Label → Bool
  | [], _ => true
  | _ :: _, [] => false
  | a :: as, b :: bs => a == b && isPrefix as bs

/-- `BorrowEdge`, without its location. -/
structure Edge where
  strong : Bool
  path : List Label
  deriving DecidableEq, Repr, Inhabited

/-- The derived order of `BorrowEdgeNoLoc`: weak before strong, then by path. -/
def Edge.cmp (a b : Edge) : Ordering :=
  (compare a.strong b.strong).then (pathCmp a.path b.path)

/-- `BorrowEdge::leq`: `a` covers `b`. -/
def Edge.leq (a b : Edge) : Bool :=
  a == b || (!a.strong && isPrefix a.path b.path)

def insertEdge (e : Edge) : List Edge → List Edge
  | [] => [e]
  | x :: xs =>
    match e.cmp x with
    | .lt => e :: x :: xs
    | .eq => x :: xs
    | .gt => x :: insertEdge e xs

/-- `MAX_EDGE_SET_SIZE`. -/
def maxEdgeSetSize : Nat := 10

/-- `BorrowEdgeSet`: sorted edges and whether the set has collapsed. -/
structure EdgeSet where
  edges : List Edge := []
  overflown : Bool := false
  deriving DecidableEq, Repr, Inhabited

/-- `BorrowEdgeSet::insert`. The size test does not ask whether the edge is
already present. -/
def EdgeSet.insert (s : EdgeSet) (e : Edge) : EdgeSet :=
  if s.overflown then s
  else if s.edges.length + 1 > maxEdgeSetSize then ⟨[⟨false, []⟩], true⟩
  else { s with edges := insertEdge e s.edges }

def EdgeSet.remove (s : EdgeSet) (e : Edge) : EdgeSet :=
  { s with edges := s.edges.erase e }

/-- A map with `Nat` keys in ascending order, as a `BTreeMap`. -/
abbrev NatMap (α : Type) := List (Nat × α)

namespace NatMap

def find? {α : Type} (m : NatMap α) (k : Nat) : Option α := List.lookup k m

/-- Insert or replace. -/
def insert {α : Type} : NatMap α → Nat → α → NatMap α
  | [], k, v => [(k, v)]
  | (k', v') :: rest, k, v =>
    if k < k' then (k, v) :: (k', v') :: rest
    else if k == k' then (k, v) :: rest
    else (k', v') :: insert rest k v

def erase {α : Type} (m : NatMap α) (k : Nat) : NatMap α := m.filter (·.1 != k)

/-- Collect pairs, a later key replacing an earlier one, as `collect` into a
`BTreeMap` does. -/
def ofList {α : Type} (l : List (Nat × α)) : NatMap α :=
  l.foldl (fun m (k, v) => insert m k v) []

end NatMap

/-- Insert into a sorted set of naturals, as a `BTreeSet`. -/
def insertNat (x : Nat) : List Nat → List Nat
  | [] => [x]
  | y :: ys => if x < y then x :: y :: ys else if x == y then y :: ys else y :: insertNat x ys

/-- `Ref`: the references borrowing this one, those it borrows from, and its
mutability. -/
structure RefInfo where
  borrowedBy : NatMap EdgeSet := []
  borrowsFrom : List Nat := []
  mutable : Bool
  deriving DecidableEq, Repr, Inhabited

end BorrowGraph

open BorrowGraph in
/-- `BorrowGraph`: every live reference by its `RefID`. -/
abbrev Graph := NatMap RefInfo

namespace Graph

open BorrowGraph

def get (g : Graph) (id : Nat) : VResult RefInfo :=
  match g.find? id with
  | some r => pure r
  | none => panicAt "borrow graph: unknown reference"

def isMutable (g : Graph) (id : Nat) : VResult Bool := do
  pure (← g.get id).mutable

/-- `new_ref`. -/
def newRef (g : Graph) (id : Nat) (mutable : Bool) : VResult Graph :=
  if (g.find? id).isSome then panicAt "borrow graph: ref exists"
  else pure (g.insert id { mutable })

/-- `add_edge`. -/
def addEdge (g : Graph) (parent : Nat) (e : Edge) (child : Nat) : VResult Graph := do
  if parent == child then panicAt "borrow graph: self borrow"
  let p ← g.get parent
  let set := ((p.borrowedBy.find? child).getD {}).insert e
  let g : Graph := g.insert parent { p with borrowedBy := p.borrowedBy.insert child set }
  let c ← g.get child
  pure (g.insert child { c with borrowsFrom := insertNat parent c.borrowsFrom })

/-- `factor`: `intermediate` borrows `path` from `parent` strongly, and takes
over the edges of `parent` below that path. -/
def factor (g : Graph) (parent : Nat) (path : List Label) (intermediate : Nat) :
    VResult Graph := do
  let p ← g.get parent
  let needs := p.borrowedBy.flatMap fun (child, set) =>
    (set.edges.filter fun e => isPrefix path e.path).map fun e => (child, e)
  let (borrowedBy, cleanup) ← needs.foldlM (fun (borrowedBy, cleanup) (child, e) =>
    match borrowedBy.find? child with
    | none => panicAt "factor: missing edge set"
    | some set =>
      if !set.edges.contains e then panicAt "factor: missing edge"
      else
        let set := set.remove e
        if set.edges.isEmpty then pure (borrowedBy.erase child, insertNat child cleanup)
        else pure (borrowedBy.insert child set, cleanup)) (p.borrowedBy, [])
  let g : Graph := g.insert parent { p with borrowedBy }
  let g ← cleanup.foldlM (fun (g : Graph) child => do
    let c ← g.get child
    if !c.borrowsFrom.contains parent then panicAt "factor: missing back edge"
    pure (g.insert child { c with borrowsFrom := c.borrowsFrom.erase parent })) g
  let g ← needs.foldlM (fun (g : Graph) (child, e) =>
    g.addEdge intermediate ⟨e.strong, e.path.drop path.length⟩ child) g
  g.addEdge parent ⟨true, path⟩ intermediate

/-- `splice_out_intermediate`. -/
def splice (g : Graph) (parent : Nat) (pe : Edge) (child : Nat) (ce : Edge) : VResult Graph :=
  if parent == child then pure g
  else
    let path := if pe.strong then pe.path ++ ce.path else pe.path
    g.addEdge parent ⟨pe.strong && ce.strong, path⟩ child

/-- `release`: remove a reference, connecting each of its parents to each of
its children. -/
def release (g : Graph) (id : Nat) : VResult Graph := do
  let r ← g.get id
  let g : Graph := g.erase id
  let g ← r.borrowsFrom.foldlM (fun (g : Graph) parent => do
    let p ← g.get parent
    let parentEdges ← match p.borrowedBy.find? id with
      | some s => pure s
      | none => panicAt "release: missing parent edge"
    let g : Graph := g.insert parent { p with borrowedBy := p.borrowedBy.erase id }
    parentEdges.edges.foldlM (fun (g : Graph) pe =>
      r.borrowedBy.foldlM (fun (g : Graph) (child, childEdges) =>
        childEdges.edges.foldlM (fun (g : Graph) ce => g.splice parent pe child ce) g) g) g) g
  r.borrowedBy.foldlM (fun (g : Graph) (child, _) => do
    let c ← g.get child
    pure (g.insert child { c with borrowsFrom := c.borrowsFrom.erase id })) g

/-- Whether an edge counts as a borrow at `label?`: a full borrow always, and
a field borrow when no label is given or its path starts with the label. -/
def atLabel (label? : Option Label) (e : Edge) : Bool :=
  match e.path, label? with
  | [], _ => true
  | _ :: _, none => true
  | l :: _, some l' => l == l'

/-- `has_full_borrows`. -/
def hasFullBorrows (g : Graph) (id : Nat) : VResult Bool := do
  pure ((← g.get id).borrowedBy.any fun (_, s) => s.edges.any (·.path.isEmpty))

/-- `has_consistent_borrows`. -/
def hasConsistentBorrows (g : Graph) (id : Nat) (label? : Option Label) : VResult Bool := do
  pure ((← g.get id).borrowedBy.any fun (_, s) => s.edges.any (atLabel label?))

/-- `has_consistent_mutable_borrows`: a borrower at the label is mutable. -/
def hasConsistentMutableBorrows (g : Graph) (id : Nat) (label? : Option Label) :
    VResult Bool := do
  (← g.get id).borrowedBy.anyM fun (child, s) =>
    if s.edges.any (atLabel label?) then g.isMutable child else pure false

/-- `is_writable`, which asserts a mutable reference. -/
def isWritable (g : Graph) (id : Nat) : VResult Bool := do
  if !(← g.isMutable id) then panicAt "is_writable: immutable reference"
  pure !(← g.hasConsistentBorrows id none)

/-- `is_freezable`, which asserts a mutable reference. -/
def isFreezable (g : Graph) (id : Nat) (label? : Option Label) : VResult Bool := do
  if !(← g.isMutable id) then panicAt "is_freezable: immutable reference"
  pure !(← g.hasConsistentMutableBorrows id label?)

/-- `is_readable`. -/
def isReadable (g : Graph) (id : Nat) (label? : Option Label) : VResult Bool := do
  if !(← g.isMutable id) then pure true else g.isFreezable id label?

/-- `unmatched_edges`: the edges of `other` no edge of `self` covers, by
parent, child and edge in order. -/
def unmatched (self other : Graph) : VResult (List (Nat × Nat × Edge)) :=
  other.foldlM (fun acc (parent, otherRef) => do
    let selfRef ← match self.find? parent with
      | some r => pure r
      | none => panicAt "unmatched_edges: missing reference"
    otherRef.borrowedBy.foldlM (fun acc (child, otherEdges) =>
      otherEdges.edges.foldlM (fun acc oe =>
        let found := match selfRef.borrowedBy.find? child with
          | some s => s.edges.any (·.leq oe)
          | none => false
        if found then pure acc
        else if parent == child then panicAt "unmatched_edges: self borrow"
        else pure (acc ++ [(parent, child, oe)])) acc) acc) []

/-- `leq`. -/
def leq (self other : Graph) : VResult Bool := do
  pure (← self.unmatched other).isEmpty

/-- `join`: add the unmatched edges of `other`. -/
def join (self other : Graph) : VResult Graph := do
  (← self.unmatched other).foldlM
    (fun (g : Graph) (parent, child, e) => g.addEdge parent e child) self

/-- `remap_refs`, a later key replacing an earlier one on a collision. -/
def remap (g : Graph) (idMap : NatMap Nat) : Graph :=
  let f (id : Nat) := (idMap.find? id).getD id
  NatMap.ofList (g.map fun (id, r) =>
    (f id, { r with
      borrowedBy := NatMap.ofList (r.borrowedBy.map fun (c, s) => (f c, s))
      borrowsFrom := r.borrowsFrom.foldl (fun s p => insertNat (f p) s) [] }))

end Graph

end MoveBytecode.Model
