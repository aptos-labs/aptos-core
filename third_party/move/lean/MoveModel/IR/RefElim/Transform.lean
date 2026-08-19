-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Value
import MoveModel.IR.State
import MoveModel.IR.Syntax
import MoveModel.IR.Semantics
import MoveModel.IR.Frame
import MoveModel.IR.Liveness

/-!
# Reference Elimination (TACAS'22 §3.1)

The direct verification translation rejects reference operations.  This
module removes them before specification injection, following the structure
of the production Move Prover pass.

Mutable references become values in the *mutation algebra*.  `Value.mut` and
the operations from `mkMutLoc` through `mutAddr` model Boogie's `$Mutation`.
The resulting bytecode is reference free and can be compiled by
`Prover.Translate.compileFun`.

The pipeline mirrors the real prover's pass structure:

1. **Immutable references.** `eliminate_imm_refs` changes `&T` locals into
   `T` values.  Immutable borrows and reads become copies; `freeze_ref`
   becomes a read.  Only `&mut` references remain.
2. **Liveness.** A backward analysis finds the point where a reference and
   all values derived from it die.  The borrow is released at that point.
3. **Borrow graph.** A forward union analysis records derivations between
   local roots, global roots, and reference locals.  Edges distinguish direct
   copies, fields, and vector indices.  Multiple incoming edges at a join are
   the possible write-back parents.
4. **Rewriting.** A borrow checks out a mutation value.  Field and vector
   borrows derive sub-mutations; reads and writes become `getMut` and `setMut`.
   When the borrow dies, its payload is written back to a parent mutation,
   local root, or global resource.

Dynamic parent tests guard write-back when a join has several candidates.
The pass implements this dispatch with block splitting.  New blocks are
allocated above `body.size`, preserving existing block identifiers, loop
headers, and back edges.  Edge-specific deaths use edge-split blocks.  Loop
targets are extended with every local written by inserted code.

## Rejected programs

The transformation is partial and reports unsupported cases as errors.

* Nested references are unsupported.  The summary-free pass also rejects
  references at function boundaries; `refElimProg` handles the supported
  cross-call cases using summaries.
* A borrowed local root cannot be read or overwritten.  A reference local
  cannot be reused while values derived from its previous borrow remain live.
* Operations cannot observe or replace a resource while a global borrow of
  that resource is live.  Calls are similarly restricted.  `move_to` and
  `exists` remain allowed because they do not inspect the checked-out value.
* `immCheck` rejects changes to an ancestor while a copied immutable borrow is
  live.  Otherwise the copied target could diverge from the source read.
* A parent mutation cannot be used while a child mutation is pending, because
  the parent payload is stale below the child path.  Sibling borrows must have
  disjoint paths, including returned derivations in summaries.
* The current path-insensitive analysis rejects overwriting an `&mut`
  parameter slot, passing one mutation to multiple `&mut` parameters, and
  joins where a live child has branch-dependent intermediate parents.

`Tests/Interp/RefElimAgree.lean` contains counterexamples motivating the
checks required by `refElim_correct`.

## Deferred write-back and aborts

Writes through a reference update the mutation payload.  Global memory is
updated only when the borrow dies.  This read-update-write discipline makes
the encoding alias free.

If execution aborts while a global borrow is live, the abort memory does not
contain the pending payload update.  This difference is unobservable to the
caller because the VM discards effects on abort.  Accordingly, `AgreeOutcome`
requires equal abort codes but compares memory only for normal returns.
Definition-side `aborts_if` checks may inspect the transient exit memory, so
their verification remains a separate obligation after transformation.  The
relation also ignores the target's internal final values for `&mut` parameters.
-/

namespace MoveModel.IR

/-! ## Fresh locals -/

/-- Fresh-local allocation: new locals live above the declared
locals, with their types accumulated in declaration order. -/
structure ElimSt where
  base : Nat
  newTys : List Ty

/-- Allocate a fresh local of the given type. -/
def ElimSt.alloc (st : ElimSt) (ty : Ty) : ElimSt × LocalIndex :=
  ({ st with newTys := st.newTys ++ [ty] }, st.base + st.newTys.length)

/-! ## Type helpers -/

/-- Strip an *immutable* reference layer (the imm pass turns `&T` into
`T`; `&mut` stays for the mutation pass). -/
def Ty.stripImm : Ty → Ty
  | .ref t => t
  | t => t

/-- Strip one reference layer (the imm pass turns `&T` locals into `T`). -/
def Ty.stripRef : Ty → Ty
  | .ref t => t
  | .mutRef t => t
  | t => t

/-- The type of field `i` of a value of type `ty`. -/
def fieldTy (Δ : StructDecls) (ty : Ty) (i : Nat) : Except String Ty := do
  let .struct r := ty | throw "field access on a non-struct type"
  let some d := Δ r | throw s!"undeclared struct {r}"
  let some ft := d.fields[i]? | throw s!"field {i} out of range"
  pure ft

/-- The element type of a vector type. -/
def elemTy : Ty → Except String Ty
  | .vector t => pure t
  | _ => throw "vector element borrow of a non-vector type"

/-- The declared type of a local. -/
def localTy (d : FunDecl) (x : LocalIndex) : Except String Ty := do
  let some ty := d.locals x | throw s!"local {x} has no declared type"
  pure ty

/-- Is the local declared `&mut`? -/
def isMutLocal (d : FunDecl) (x : LocalIndex) : Bool :=
  match d.locals x with
  | some (.mutRef _) => true
  | _ => false

/-- Is the local one of the function's mutable-reference parameter slots? -/
def isMutParam (d : FunDecl) (x : LocalIndex) : Bool :=
  x < d.numParams && isMutLocal d x

/-- A positive mutable-local classification exposes its payload type. -/
theorem isMutLocal_eq_true {d : FunDecl} {x : LocalIndex} :
    isMutLocal d x = true ↔ ∃ ty, d.locals x = some (.mutRef ty) := by
  unfold isMutLocal
  cases h : d.locals x <;> simp
  rename_i ty
  cases ty <;> simp

/-- The payload type of a `&mut`-declared local. -/
def mutPayloadTy (d : FunDecl) (x : LocalIndex) : Except String Ty := do
  let some (.mutRef t) := d.locals x
    | throw s!"local {x} is not a mutable reference"
  pure t

/-! ## The borrow graph (forward dataflow, union join) -/

/-- A borrow-graph node: the root locations and the reference locals.
`anyRoot` marks a call-manufactured borrow — the callee may return a
reference rooted at *some* global unknown to the caller's graph (only the
imm pass's derivation graph uses it; the mutation pass resolves calls
precisely through summaries). -/
inductive BNode where
  | localRoot (x : LocalIndex)
  | globalRoot (r : ResourceId)
  | refNode (t : LocalIndex)
  | anyRoot
  deriving DecidableEq, Repr

/-- One derivation step of a borrow edge: a field offset or a dynamic
vector index. -/
inductive BStep where
  | field (i : Nat)
  | index
  deriving DecidableEq, Repr

/-- Convert a static borrow path to the runtime matching pattern used by
mutation-parent dispatch; vector indices are dynamic wildcards. -/
def bPathPattern (path : List BStep) : List (Option Nat) :=
  path.map fun step =>
    match step with
    | .field i => some i
    | .index => none

/-- A borrow edge: `child` (always a reference local) was derived from
`parent` along the step path — `[]` is a direct copy/alias, one step a
field or element borrow, several a derivation summarized through a call
(MVP's hyper edges). -/
structure BEdge where
  parent : BNode
  child : LocalIndex
  path : List BStep
  deriving DecidableEq, Repr

/-- The borrow graph: a set of edges, accumulated over all paths (the
union join of MVP's dataflow); several in-edges of one reference are the
write-back candidates resolved dynamically by `isParent`. -/
abbrev BGraph := List BEdge

/-- Insert a borrow edge unless an equal edge is already present. -/
def gInsert (e : BEdge) (g : BGraph) : BGraph :=
  if g.contains e then g else e :: g

/-- Remove every derivation whose child is overwritten.  Borrow graphs are
may-graphs at joins, but assigning a fresh reference value to a local is a
strong update: derivations of the local's previous value must not remain as
write-back candidates later in the same block. -/
def gRemoveChild (child : LocalIndex) (g : BGraph) : BGraph :=
  g.filter (fun edge => edge.child != child)

/-- Strongly replace a reference local's derivation by one new edge. -/
def gReplaceChild (e : BEdge) (g : BGraph) : BGraph :=
  e :: gRemoveChild e.child g

/-- Form the deduplicated union of two borrow graphs. -/
def gUnion (a b : BGraph) : BGraph :=
  a.foldl (fun acc e => gInsert e acc) b

/-- Decide whether every edge of one borrow graph occurs in another. -/
def gSub (a b : BGraph) : Bool :=
  a.all (b.contains ·)

/-- Decide extensional equality of two borrow graphs. -/
def gEq (a b : BGraph) : Bool :=
  a.all b.contains && b.all a.contains

/-- The in-edges (write-back candidates) of a reference local. -/
def inEdges (g : BGraph) (t : LocalIndex) : List BEdge :=
  g.filter (·.child == t)

/-! ## Function summaries (MVP's inter-procedural borrow analysis) -/


/-- Insert a derivation edge, transitively closed through reference
parents: the child inherits its parent's ancestors, so a reference's
roots and chain references are all among its *direct* in-edge parents
(the imm pass's coverage invariant — total, no fuel; the mutation pass
keeps one-hop edges, since its write-backs are per-hop). -/
def gInsertClosed (e : BEdge) (g : BGraph) : BGraph :=
  let inherited : List BEdge := match e.parent with
    | .refNode p => (inEdges g p).map fun pe => ⟨pe.parent, e.child, []⟩
    | _ => []
  inherited.foldl (fun g e => gInsert e g) (gInsert e g)

/-- The borrow summary of a function: which parameter positions are
mutable references (they are returned as *finals*, MVP's
`call r := f(r)`), and — per original return position — from which
argument position, along which derivation path, a returned reference may
have been derived (MVP's `Reference(param) —Hyper→ ReturnPlaceholder`
edges).  Value returns have no derivations. -/
structure FunSummary where
  mutParams : List LocalIndex
  retDerivs : List (List (Nat × List BStep))
  deriving BEq, Repr

/-- The summary table of the program under elimination. -/
abbrev Summaries := FunId → Option FunSummary

/-- The empty summary table (used by the single-function entry point:
reference-crossing calls are then rejected). -/
def noSummaries : Summaries := fun _ => none

/-- The mutable-reference parameter positions of a declaration. -/
def mutParamsOf (d : FunDecl) : List LocalIndex :=
  (List.range d.numParams).filter (isMutLocal d)

/-- The graph transfer of one instruction (edge collection only; the
fragment checks happen during rewriting).  A call with a summarized
callee contributes the instantiated summary edges: the `j`-th result
derives from the argument at the summarized position along the
summarized (hyper) path. -/
def graphStep (sums : Summaries) (d : FunDecl) (g : BGraph) :
    Instr → BGraph
  | .call [dst] .borrowLoc [x] => gReplaceChild ⟨.localRoot x, dst, []⟩ g
  | .call [dst] (.borrowGlobal r) _ =>
      gReplaceChild ⟨.globalRoot r, dst, []⟩ g
  | .call [dst] (.borrowField i) [t] =>
      gReplaceChild ⟨.refNode t, dst, [.field i]⟩ g
  | .call [dst] .borrowVecElem [t, _] =>
      gReplaceChild ⟨.refNode t, dst, [.index]⟩ g
  | .assign dst src =>
      if isMutLocal d src && isMutLocal d dst then
        gReplaceChild ⟨.refNode src, dst, []⟩ g
      else g
  | .call dsts (.function f) srcs =>
      match sums f with
      | none => g
      | some sum =>
          (dsts.zip sum.retDerivs).foldl
            (fun g (dst, derivs) =>
              derivs.foldl
                (fun g (argPos, path) =>
                  match srcs[argPos]? with
                  | some src => gInsert ⟨.refNode src, dst, path⟩ g
                  | none => g)
                (gRemoveChild dst g))
            g
  | _ => g

/-- Transfer a borrow graph forward through every instruction of a block. -/
def graphThroughBlock (sums : Summaries) (d : FunDecl) (blk : Block)
    (g : BGraph) : BGraph :=
  blk.instrs.foldl (graphStep sums d) g

/-- Entry borrow graphs per block, by forward fixpoint with union join. -/
def borrowAnalysis (sums : Summaries) (d : FunDecl) :
    Array BGraph := Id.run do
  let n := d.body.size
  let total := (List.range n).foldl
    (fun acc b => acc + ((d.body.blocks b).map (·.instrs.length)).getD 0) 0
  let mut entry : Array BGraph := Array.replicate n []
  let rounds := n * (total + 2) + 2
  for _ in [0:rounds] do
    let mut changed := false
    for b in [0:n] do
      match d.body.blocks b with
      | none => pure ()
      | some blk =>
        let out := graphThroughBlock sums d blk (entry.getD b [])
        for s in termSuccs blk.term do
          let merged := gUnion out (entry.getD s [])
          unless gEq merged (entry.getD s []) do
            entry := entry.set! s merged
            changed := true
    unless changed do
      return entry
  return entry

/-! ## The immutable-reference pre-pass (`eliminate_imm_refs`) -/

/-- Is the local declared `&` (immutable)? -/
def isImmLocal (d : FunDecl) (x : LocalIndex) : Bool :=
  match d.locals x with
  | some (.ref _) => true
  | _ => false

/-- Rewrite one instruction of the imm pass.  `origTy` classifies locals
by their *original* declared types. -/
def elimImmInstr (d : FunDecl) (st : ElimSt) :
    Instr → Except String (ElimSt × List Instr)
  | .call [dst] .borrowLoc [x] => do
      match ← localTy d dst with
      | .ref _ => do
          if (← localTy d x).isRef then
            throw "nested references are not supported"
          pure (st, [.assign dst x])
      | _ => pure (st, [.call [dst] .borrowLoc [x]])
  | .call [dst] (.borrowField i) [t] => do
      if !isImmLocal d dst && isImmLocal d t then
        throw "mutable borrow through an immutable reference"
      match ← localTy d dst with
      | .ref _ =>
          match ← localTy d t with
          | .ref _ => pure (st, [.call [dst] (.getField i) [t]])
          | .mutRef ty => do
              let (st, tmp) := st.alloc ty
              pure (st, [.call [tmp] .readRef [t],
                         .call [dst] (.getField i) [tmp]])
          | _ => throw "field borrow of a non-reference"
      | _ => pure (st, [.call [dst] (.borrowField i) [t]])
  | .call [dst] .borrowVecElem [t, i] => do
      if !isImmLocal d dst && isImmLocal d t then
        throw "mutable borrow through an immutable reference"
      match ← localTy d dst with
      | .ref _ =>
          match ← localTy d t with
          | .ref _ => pure (st, [.call [dst] .vecGet [t, i]])
          | .mutRef ty => do
              let (st, tmp) := st.alloc ty
              pure (st, [.call [tmp] .readRef [t],
                         .call [dst] .vecGet [tmp, i]])
          | _ => throw "vector element borrow of a non-reference"
      | _ => pure (st, [.call [dst] .borrowVecElem [t, i]])
  | .call [dst] (.borrowGlobal r) [aT] => do
      match ← localTy d dst with
      | .ref _ => pure (st, [.call [dst] (.getGlobal r) [aT]])
      | _ => pure (st, [.call [dst] (.borrowGlobal r) [aT]])
  | .call [dst] .freezeRef [t] =>
      if !isImmLocal d dst then
        throw "freeze into a non-immutable slot"
      else do
        match ← localTy d t with
        | .ref _ => pure (st, [.assign dst t])
        | .mutRef _ => pure (st, [.call [dst] .readRef [t]])
        | _ => throw "freeze of a non-reference"
  | .call [dst] .readRef [t] => do
      match ← localTy d t with
      | .ref _ => pure (st, [.assign dst t])
      | _ => pure (st, [.call [dst] .readRef [t]])
  | .call [] .writeRef [t, vt] => do
      match ← localTy d t with
      | .ref _ => throw "write through an immutable reference"
      | _ => pure (st, [.call [] .writeRef [t, vt]])
  | .call _ (.borrowFieldInst _ _) _
  | .call _ (.borrowGlobalInst _ _) _ =>
      throw "generic reference operations are not yet supported by reference elimination"
  | .load dst v =>
      if v.refFree then pure (st, [.load dst v])
      else throw "a reference literal is not source-level"
  | .assign dst src =>
      -- assignments must respect the imm/non-imm split: the imm side
      -- becomes a value copy, so a reference flowing across the split
      -- would change meaning (compilers coerce with `freeze_ref`)
      if isImmLocal d dst && !isImmLocal d src then
        throw "coerce into an immutable slot with freeze_ref"
      else if !isImmLocal d dst && isImmLocal d src then
        throw "an immutable reference assigned to a non-reference slot"
      else pure (st, [.assign dst src])
  | .call _ (.mkMutLoc _) _ | .call _ (.mkMutGlobal _) _
  | .call _ (.childMutField _) _ | .call _ .childMutIndex _
  | .call _ .getMut _ | .call _ .setMut _ | .call _ (.isParent _) _
  | .call _ (.mutPathIndex _) _ | .call _ (.isMutLoc _) _
  | .call _ (.isMutGlobal _) _ | .call _ .mutAddr _ =>
      throw "mutation operations are not source-level"
  | i => pure (st, [i])

/-- All ancestors of a reference in the derivation graph — its direct
in-edge parents (the graph is kept transitively closed at insertion,
`gInsertClosed`). -/
def immAncestors (g : BGraph) (t : LocalIndex) : List BNode :=
  ((inEdges g t).map (·.parent)).eraseDups

/-- The transfer of the pre-elimination derivation graph: every borrow
and reference copy records an edge, so a live immutable reference's
ancestors — the roots and every reference on the chain — are visible. -/
def immStep (d : FunDecl) (g : BGraph) : Instr → BGraph
  | .call [dst] .borrowLoc [x] => gInsertClosed ⟨.localRoot x, dst, []⟩ g
  | .call [dst] (.borrowGlobal r) _ =>
      gInsertClosed ⟨.globalRoot r, dst, []⟩ g
  | .call [dst] (.borrowField _) [t] => gInsertClosed ⟨.refNode t, dst, []⟩ g
  | .call [dst] .borrowVecElem [t, _] =>
      gInsertClosed ⟨.refNode t, dst, []⟩ g
  | .call [dst] .freezeRef [t] => gInsertClosed ⟨.refNode t, dst, []⟩ g
  | .assign dst src =>
      -- a copy inherits derivations only if the source can hold a
      -- reference: declared as one, or itself derived (leaks through
      -- value slots keep their chains; plain value copies stay clean)
      if (d.locals src).any (·.isRef) || !(immAncestors g src).isEmpty
      then gInsertClosed ⟨.refNode src, dst, []⟩ g
      else g
  | .call dsts (.function _) srcs =>
      -- a returned reference derives from an argument target or is a
      -- callee-manufactured global borrow (`anyRoot`)
      dsts.foldl (init := g) fun g dst =>
        if (d.locals dst).any (·.isRef) then
          srcs.foldl (init := gInsertClosed ⟨.anyRoot, dst, []⟩ g) fun g s =>
            if (d.locals s).any (·.isRef) then
              gInsertClosed ⟨.refNode s, dst, []⟩ g
            else g
        else g
  | _ => g

/-- Transfer the immutable-reference graph through a block. -/
def immThroughBlock (d : FunDecl) (blk : Block) (g : BGraph) :
    BGraph :=
  blk.instrs.foldl (immStep d) g

/-- The seed edges of the entry block: a reference-typed parameter is a
borrow of *some unknown location*.  Its runtime value retains the original
frame-qualified root; `anyRoot` is the static abstraction used by the
exclusivity checks (`anyRoot` meets every global). -/
def paramSeeds (d : FunDecl) : BGraph :=
  (List.range d.numParams).filterMap fun i =>
    if (d.locals i).any Ty.isRef then some ⟨.anyRoot, i, []⟩ else none

/-- Entry derivation graphs per block (forward fixpoint, union join;
the entry block starts from the parameter seeds). -/
def immAnalysis (d : FunDecl) : Array BGraph := Id.run do
  let n := d.body.size
  let total := (List.range n).foldl
    (fun acc b => acc + ((d.body.blocks b).map (·.instrs.length)).getD 0) 0
  let mut entry : Array BGraph :=
    (Array.replicate n []).set! d.body.entry (paramSeeds d)
  let rounds := n * (total + 2) + 2
  for _ in [0:rounds] do
    let mut changed := false
    for b in [0:n] do
      match d.body.blocks b with
      | none => pure ()
      | some blk =>
        let out := immThroughBlock d blk (entry.getD b [])
        for s in termSuccs blk.term do
          let merged := gUnion out (entry.getD s [])
          unless gEq merged (entry.getD s []) do
            entry := entry.set! s merged
            changed := true
    unless changed do
      return entry
  return entry


/-- Do two ancestor nodes possibly name the same location?  `anyRoot`
(a call-manufactured borrow — some unknown global) meets every global. -/
def nodeMeets : BNode → BNode → Bool
  | .anyRoot, .globalRoot _ | .globalRoot _, .anyRoot
  | .anyRoot, .anyRoot => true
  | a, b => a == b

/-- The union of the ancestor sets of the given locals. -/
def collectAnc (g : BGraph) (ts : List LocalIndex) (acc : List BNode) :
    List BNode :=
  ts.foldl (init := acc) fun acc t =>
    (immAncestors g t).foldl
      (fun a n => if a.contains n then a else n :: a) acc

/-- The set of nodes the exclusivity check protects at an instruction:
the ancestors of the live immutable copies — and, at a call, of the
call's own immutable arguments. -/
def protectedAnc (d : FunDecl) (g : BGraph) (liveAfter : LiveSet)
    (i : Instr) : List BNode :=
  let anc := collectAnc g (liveAfter.toList.filter (isImmLocal d)) []
  match i with
  | .call _ (.function _) srcs =>
      collectAnc g (srcs.filter (isImmLocal d)) anc
  | _ => anc

/-- Reject operations whose effect a live immutable copy would observe:
the source reads through the reference at *use* time, the eliminated code
copied at *borrow* time — any value change of an ancestor in the window
diverges (`move_from` makes the source stuck, which forward simulation
tolerates, so removal is fine; `move_to` never aliases a present
  borrow).  A call additionally protects its own immutable arguments while
  the callee accesses their frame-qualified roots directly. -/
def immCheck (d : FunDecl) (g : BGraph) (liveAfter : LiveSet)
    (i : Instr) : Except String Unit :=
  let anc := protectedAnc d g liveAfter i
  -- writes go through arbitrary references: a write through `t` is
  -- observable iff `t`'s chain and a live copy's chain share any node
  -- (the same root suffices — the reference need not be on the copy's
  -- own derivation chain)
  let touches : LocalIndex → Bool := fun t =>
    anc.contains (.refNode t) ||
      (immAncestors g t).any fun n => anc.any (nodeMeets n)
  -- an `&`-typed slot that may hold a copied reference must not itself
  -- be borrowed: reads through the borrow see the reference in the
  -- source but the copy in the target
  if match i with
     | .call dsts .borrowLoc [x] =>
         isImmLocal d x ||
           (dsts.any (isImmLocal d) && !(immAncestors g x).isEmpty)
     | _ => false then
    throw "borrow of an immutable-reference slot"
  -- a written local must not be a live copy's root, nor an ancestor
  -- root of an immutable reference the instruction itself reads (the
  -- fresh copy would capture its own slot)
  else if (instrDefs i).any (fun x =>
      anc.contains (.localRoot x) ||
      (collectAnc g ((instrUses i).filter (fun u =>
        isImmLocal d u || isMutLocal d u)) []).contains
        (.localRoot x)) then
    throw "local written while immutably borrowed"
  -- a non-reference argument slot with derivation ancestors may hold a
  -- leaked reference the callee's frame cannot cover
  else if match i with
     | .call _ (.function _) srcs =>
         srcs.any (fun t => !(d.locals t).any Ty.isRef &&
           !(immAncestors g t).isEmpty)
     | _ => false then
    throw "a possibly-reference-holding value crosses a call boundary"
  else if anc.isEmpty then
    pure ()
  else
    match i with
    | .call _ .writeRef (t :: _) =>
        if touches t then
          throw "reference written while immutably borrowed"
        else pure ()
    | .call [dst] .borrowLoc [x] =>
        if isMutLocal d dst && anc.contains (.localRoot x) then
          throw "local mutably borrowed while immutably borrowed"
        else pure ()
    | .call [dst] (.borrowGlobal r) _ =>
        if isMutLocal d dst && anc.any (nodeMeets (.globalRoot r)) then
          throw "resource mutably borrowed while immutably borrowed"
        else pure ()
    | .call [dst] (.borrowField _) (t :: _) =>
        if isMutLocal d dst && anc.contains (.refNode t) then
          throw "reference re-borrowed while immutably borrowed"
        else pure ()
    | .call [dst] .borrowVecElem (t :: _) =>
        if isMutLocal d dst && anc.contains (.refNode t) then
          throw "reference re-borrowed while immutably borrowed"
        else pure ()
    -- `move_to` included: it can re-create a resource `move_from`
    -- removed while a copy is live — the source read comes back with the
    -- fresh value, the copy stays stale (`move_from` alone only makes
    -- the source stuck)
    | .call _ (.writeGlobal r) _ | .call _ (.moveTo r) _ =>
        if anc.any (nodeMeets (.globalRoot r)) then
          throw "resource written while immutably borrowed"
        else pure ()
    | .call _ (.function _) srcs =>
        if srcs.any (fun t => isMutLocal d t && touches t) then
          throw "mutable reference passed to a call while immutably \
            borrowed"
        else if anc.any (fun nn => match nn with
            | .globalRoot _ | .anyRoot => true
            | _ => false) then
          throw "call while a global is immutably borrowed"
        else pure ()
    | _ => pure ()

/-! ## A-posteriori stability of the dataflow results

The fixpoint loops are fuel-bounded; on exhaustion they return an
*under*-approximation, which the exclusivity checks must not trust (a
missed live local or edge is a missed rejection).  Consumers therefore
validate stability — each block's result contains its transfer — and
reject otherwise; the correctness proofs invert these checks instead of
reasoning about the fixpoint iterations. -/

/-- Is a forward graph result a post-fixpoint (each successor's entry
graph contains the block's out-graph)? -/
def graphStable (d : FunDecl) (through : Block → BGraph → BGraph)
    (entry : Array BGraph) : Bool :=
  (List.range d.body.size).all fun b =>
    match d.body.blocks b with
    | none => true
    | some blk =>
        let out := through blk (entry.getD b [])
        (termSuccs blk.term).all fun s => gSub out (entry.getD s [])

/-- Reference kinds must agree across a call boundary: the imm pass
turns an `&`-typed slot into a value slot on *both* sides of the call,
so an immutable-slot argument must land in an immutable parameter and
vice versa, a `&mut` return must land in a `&mut` destination, and an
immutable destination must not receive one (a per-function pass cannot
coerce either mismatch). -/
def immBoundaryInstr (sigs : FunId → Option FunDecl) (d : FunDecl) :
    Instr → Bool
  | .call dsts (.function f) srcs =>
      match sigs f with
      | none => false
      | some d' =>
          (List.range srcs.length).all (fun i =>
            match srcs[i]? with
            | some src =>
                isImmLocal d src == isImmLocal d' i &&
                (d.locals src).any Ty.isRef == (d'.locals i).any Ty.isRef
            | none => true) &&
          (List.range dsts.length).all (fun j =>
            match dsts[j]?, d'.returns[j]? with
            | some dst, some (.mutRef _) => isMutLocal d dst
            | _, _ => true)
  | _ => true

/-- Rewrite one block's instruction list: check, rewrite, and step the
derivation graph per instruction (structurally recursive — the
correctness proof inverts it by plain induction). -/
def elimImmBlock (sigs : FunId → Option FunDecl) (d : FunDecl)
    (lat : LiveSet) :
    List Instr → BGraph → ElimSt →
    Except String (List Instr × ElimSt × BGraph)
  | [], g, st => pure ([], st, g)
  | i :: is, g, st => do
      unless (instrDefs i ++ instrUses i).all (· < d.numLocals) do
        throw "an instruction references an undeclared local"
      unless immBoundaryInstr sigs d i do
        throw "reference kinds disagree at a call boundary"
      immCheck d g (liveBeforeSuffix lat is) i
      let (st', tgt) ← elimImmInstr d st i
      let (rest, stEnd, gEnd) ← elimImmBlock sigs d lat is
        (immStep d g i) st'
      pure (tgt ++ rest, stEnd, gEnd)

/-- An immutable *reference* must not escape through `ret`: the pass
returns the copy, the raw semantics the reference — a per-function pass
cannot rewrite the callers' view (MVP's whole-program pass rewrites the
signature).  An `&`-typed ret source must be a provably plain value —
not a parameter, no derivation ancestors. -/
def checkRetEscape (d : FunDecl) (gEnd : BGraph) :
    Term → Except String Unit
  | .ret srcs =>
      if srcs.all (fun src =>
          !isImmLocal d src ||
            (!decide (src < d.numParams) &&
              (immAncestors gEnd src).isEmpty)) then
        -- a non-reference slot with derivation ancestors may hold a
        -- leaked reference: the caller cannot cover its root
        if srcs.all (fun src =>
            (d.locals src).any Ty.isRef ||
              (immAncestors gEnd src).isEmpty) then
          -- a `&mut`-typed source must be declared `&mut` at its
          -- position: the caller's boundary check keys on the declared
          -- return kinds
          if (List.range srcs.length).all (fun j =>
              match srcs[j]? with
              | some src =>
                  !isMutLocal d src ||
                    (d.returns[j]?).any (fun t => match t with
                      | .mutRef _ => true
                      | _ => false)
              | none => true) then pure ()
          else throw "a mutable reference is returned at a \
            value-declared position"
        else throw "a possibly-reference-holding value escapes through \
          ret"
      else throw "an immutable borrow escapes through ret"
  | _ => pure ()

/-- All instruction and terminator operands must be declared locals. -/
def checkTermRange (d : FunDecl) (term : Term) : Except String Unit :=
  if (termReads term).all (· < d.numLocals) then pure ()
  else throw "a terminator references an undeclared local"

/-- Rewrite the given blocks in order, threading the fresh-local
state. -/
def elimImmBlocks (sigs : FunId → Option FunDecl) (d : FunDecl)
    (liveIn : Array LiveSet) (graphs : Array BGraph) :
    List BlockId → ElimSt →
    Except String (List (Option Block) × ElimSt)
  | [], st => pure ([], st)
  | b :: bs, st =>
      match d.body.blocks b with
      | none => do
          let (rest, stEnd) ← elimImmBlocks sigs d liveIn graphs bs st
          pure (none :: rest, stEnd)
      | some blk => do
          let (instrs, st', gEnd) ← elimImmBlock sigs d
            (liveAtTermIn liveIn blk) blk.instrs (graphs.getD b []) st
          let _ ← checkTermRange d blk.term
          let _ ← checkRetEscape d gEnd blk.term
          let (rest, stEnd) ← elimImmBlocks sigs d liveIn graphs bs st'
          pure (some ⟨instrs, blk.term⟩ :: rest, stEnd)

/-- Eliminate immutable references: `&T` locals become `T` values.  Only
`&mut` references remain for the mutation pass.  The pass enforces the
copy-at-borrow discipline (`immCheck`): nothing a live immutable copy
derives from may change value while it is live. -/
def elimImmRefs (sigs : FunId → Option FunDecl) (d : FunDecl) :
    Except String FunDecl := do
  unless d.numParams ≤ d.numLocals do
    throw "function parameters exceed the declared local range"
  let liveIn := liveAnalysis d
  let graphs := immAnalysis d
  unless liveStable d liveIn do
    throw "the liveness analysis does not converge"
  unless graphStable d (immThroughBlock d) graphs do
    throw "the derivation-graph analysis does not converge"
  unless gSub (paramSeeds d) (graphs.getD d.body.entry []) do
    throw "the derivation-graph analysis lost the parameter seeds"
  let (blocks, st) ← elimImmBlocks sigs d liveIn graphs
    (List.range d.body.size) ⟨d.numLocals, []⟩
  pure { d with
    numLocals := d.numLocals + st.newTys.length
    locals := fun t =>
      if t < d.numLocals then (d.locals t).map Ty.stripImm
      else st.newTys[t - d.numLocals]?
    returns := d.returns.map Ty.stripImm
    body := { d.body with blocks := fun b => (blocks[b]?).join } }

/-- Statically disjoint derivation paths: distinct field offsets at some
common position, before either path ends or takes a dynamic index. -/
def disjointPaths : List BStep → List BStep → Bool
  | .field i :: p, .field j :: q =>
      if i == j then disjointPaths p q else true
  | _, _ => false

/-- Summarize one (post-imm) declaration under the current table: the
derivations of every returned reference, by walking ancestor chains from
the ret-point graphs up to the parameter nodes.  Chains reaching a root
or an underived non-parameter reference are verifier-illegal escapes. -/
def summarize (sums : Summaries) (d : FunDecl) :
    Except String FunSummary := do
  let graphs := borrowAnalysis sums d
  unless graphStable d (graphThroughBlock sums d) graphs do
    throw "the borrow-graph analysis does not converge"
  let numRets := d.returns.length
  let mut derivs : Array (List (Nat × List BStep)) :=
    Array.replicate numRets []
  for b in List.range d.body.size do
    match d.body.blocks b with
    | none => pure ()
    | some blk =>
      match blk.term with
      | .ret srcs =>
          let g := graphThroughBlock sums d blk (graphs.getD b [])
          for (src, j) in srcs.zipIdx do
            if isMutLocal d src then
              let chains ← walk g (d.numLocals + 1) src []
              for c in chains do
                if !(derivs.getD j []).contains c then
                  derivs := derivs.set! j (c :: derivs.getD j [])
      | _ => pure ()
  -- two *returned* references sharing an argument origin must be
  -- statically disjoint (distinct fields before any dynamic index):
  -- their caller-side write-backs would otherwise race
  for j in List.range numRets do
    for k in List.range numRets do
      if j < k then
        for (a, p) in derivs.getD j [] do
          for (a', q) in derivs.getD k [] do
            if a == a' && !disjointPaths p q then
              throw "overlapping returned reference derivations"
  pure ⟨mutParamsOf d, derivs.toList⟩
where
  /-- All (argument position, relative path) derivations of `t`,
  accumulating the path suffix below `t`. -/
  walk (g : BGraph) : Nat → LocalIndex → List BStep →
      Except String (List (Nat × List BStep))
    | 0, _, _ => throw "reference derivations do not converge"
    | fuel + 1, t, suffix => do
      if t < d.numParams then
        if isMutLocal d t then pure [(t, suffix)]
        else throw "reference derived from a non-reference parameter"
      else
        match inEdges g t with
        | [] => throw "a returned reference escapes its function \
            (derived from a local, a global, or uninitialized)"
        | es =>
            es.foldlM (init := ([] : List (Nat × List BStep)))
              fun acc e =>
                match e.parent with
                | .refNode p => do
                    let more ← walk g fuel p (e.path ++ suffix)
                    pure (acc ++ more)
                | .localRoot _ | .globalRoot _ =>
                    throw "a returned reference escapes its function \
                      (rooted at a local or a global)"
                | .anyRoot =>
                    throw "internal: anyRoot in a borrow graph"

/-! ## The rewriting emitter -/

structure EmitSt where
  elim : ElimSt
  nextId : BlockId
  /-- Finished blocks (original and split-off). -/
  done : List (BlockId × Block)
  /-- Split-off block ↦ the source block it derives from (loop
  membership). -/
  blockSrc : List (BlockId × BlockId)
  curId : BlockId
  cur : List Instr
  /-- Mutations whose payload may differ from its checkout (written
  through, or receiving write-backs/finals) — block-local, seeded
  conservatively at block entry.  A *blocked* intermediate at `ret` (its
  write-back withheld by a returned descendant) is sound only if
  unwritten. -/
  written : List LocalIndex

/-- The index allocated by the next fresh-local request. -/
def EmitSt.nextLocal (st : EmitSt) : LocalIndex :=
  st.elim.base + st.elim.newTys.length

/-- State-and-error monad used while emitting core-elimination blocks. -/
private abbrev EM := StateT EmitSt (Except String)

/-- Allocate a fresh temporary local of the given type. -/
def alloc (ty : Ty) : EM LocalIndex := do
  let s ← get
  let (e, x) := s.elim.alloc ty
  set { s with elim := e }
  pure x

/-- Exact state and index produced by one fresh-local allocation. -/
theorem alloc_inv {ty : Ty} {st st' : EmitSt} {x : LocalIndex}
    (h : alloc ty st = .ok (x, st')) :
    x = st.elim.base + st.elim.newTys.length ∧
      st' = { st with elim :=
        { st.elim with newTys := st.elim.newTys ++ [ty] } } := by
  unfold alloc at h
  change Except.ok
      (st.elim.base + st.elim.newTys.length,
        { st with elim :=
          { st.elim with newTys := st.elim.newTys ++ [ty] } }) =
    Except.ok (x, st') at h
  cases h
  exact ⟨rfl, rfl⟩

/-- Allocation returns the current frontier and advances it by one. -/
theorem alloc_nextLocal {ty : Ty} {st st' : EmitSt} {x : LocalIndex}
    (h : alloc ty st = .ok (x, st')) :
    x = st.nextLocal ∧ st'.nextLocal = st.nextLocal + 1 := by
  obtain ⟨rfl, rfl⟩ := alloc_inv h
  simp [EmitSt.nextLocal, Nat.add_assoc]

/-- Allocation monotonically advances the fresh-local frontier. -/
theorem alloc_nextLocal_le {ty : Ty} {st st' : EmitSt} {x : LocalIndex}
    (h : alloc ty st = .ok (x, st')) : st.nextLocal ≤ st'.nextLocal := by
  rw [(alloc_nextLocal h).2]
  exact Nat.le_succ _

/-- Append one instruction to the current emitted block. -/
def emit (i : Instr) : EM Unit :=
  modify fun s => { s with cur := s.cur ++ [i] }

/-- Record a mutation local as having a pending write-back. -/
def markWritten (t : LocalIndex) : EM Unit :=
  modify fun s =>
    if s.written.contains t then s else { s with written := t :: s.written }

/-- Clear the pending-write marker for a mutation local. -/
def clearWritten (t : LocalIndex) : EM Unit :=
  modify fun s => { s with written := s.written.filter (· ≠ t) }

/-- Append a list of instructions to the current emitted block. -/
def emitAll (is : List Instr) : EM Unit :=
  modify fun s => { s with cur := s.cur ++ is }

/-- Allocate a fresh block and remember its originating source block. -/
def newBlockId (src : BlockId) : EM BlockId := do
  let s ← get
  set { s with
    nextId := s.nextId + 1
    blockSrc := (s.nextId, src) :: s.blockSrc }
  pure s.nextId

/-- Close the current block with `term`; continue accumulating into
`contId`. -/
def closeBlock (term : Term) (contId : BlockId) : EM Unit :=
  modify fun s =>
    { s with
      done := (s.curId, ⟨s.cur, term⟩) :: s.done
      curId := contId
      cur := [] }

/-- Emit `body` under the guard local `g` (a diamond: branch to a fresh
body block or directly to the continuation). -/
def emitGuarded (src : BlockId) (g : LocalIndex)
    (body : List Instr) : EM Unit := do
  let doId ← newBlockId src
  let contId ← newBlockId src
  closeBlock (.branch g doId contId) contId
  modify fun s =>
    { s with done := (doId, ⟨body, .jump contId⟩) :: s.done }

/-! ## Write-back generation -/

/-- Lift an `Except` computation into the emitter. -/
def EM.lift {α : Type} (x : Except String α) : EM α :=
  fun s => x.map (·, s)

/-- The functional update of `cur : curTy` (the parent payload during a
write-back of `t` into `p`) along the remaining derivation path: descend
by field selection resp. element read (indices recovered from the child's
dynamic path with `mutPathIndex` at the absolute step position `k`),
replace the leaf with `t`'s payload, and rebuild with
`update_field`/`vec_set` on the way up.  Returns the local holding the
updated value. -/
def buildUpdate (Δ : StructDecls) (p t : LocalIndex) :
    Nat → LocalIndex → Ty → List BStep → EM (List Instr × LocalIndex)
  | _, _, tc, [] => do
      let c ← alloc tc
      pure ([.call [c] .getMut [t]], c)
  | k, cur, curTy, .field i :: rest => do
      let ft ← EM.lift (fieldTy Δ curTy i)
      let sub ← alloc ft
      let (instrs, out') ← buildUpdate Δ p t (k + 1) sub ft rest
      let out ← alloc curTy
      pure (.call [sub] (.getField i) [cur] :: instrs ++
        [.call [out] (.updateField i) [cur, out']], out)
  | k, cur, curTy, .index :: rest => do
      let et ← EM.lift (elemTy curTy)
      let idx ← alloc .u64
      let sub ← alloc et
      let (instrs, out') ← buildUpdate Δ p t (k + 1) sub et rest
      let out ← alloc curTy
      pure (.call [idx] (.mutPathIndex k) [p, t] ::
        .call [sub] .vecGet [cur, idx] :: instrs ++
        [.call [out] .vecSet [cur, idx, out']], out)

/-- The write-back of the dying reference `t` along the in-edge `e`
(one step: into the parent mutation — a functional update of its payload
along the edge path — the root local, or global memory). -/
def wbBody (Δ : StructDecls) (d : FunDecl) (t : LocalIndex)
    (e : BEdge) : EM (List Instr) := do
  match e.parent with
  | .refNode p =>
      let tp ← EM.lift (mutPayloadTy d p)
      match e.path with
      | [] =>
          let tc ← EM.lift (mutPayloadTy d t)
          let c ← alloc tc
          pure [.call [c] .getMut [t], .call [p] .setMut [p, c]]
      | path => do
          let a ← alloc tp
          let (instrs, out) ← buildUpdate Δ p t 0 a tp path
          pure (.call [a] .getMut [p] :: instrs ++
            [.call [p] .setMut [p, out]])
  | .localRoot x =>
      if !e.path.isEmpty then
        throw "internal: a root borrow edge carries a path"
      else pure [.call [x] .getMut [t]]
  | .globalRoot r =>
      if !e.path.isEmpty then
        throw "internal: a root borrow edge carries a path"
      else do
        let tc ← EM.lift (mutPayloadTy d t)
        let ad ← alloc .address
        let c ← alloc tc
        pure [.call [ad] .mutAddr [t], .call [c] .getMut [t],
              .call [] (.writeGlobal r) [ad, c]]
  | .anyRoot => throw "internal: anyRoot in a borrow graph"

/-- The dispatch guard testing whether `t`'s dynamic location was derived
along `e` (`is_parent` with the edge path as pattern — dynamic indices as
wildcards; `is_mut_loc`/`is_mut_global` for roots). -/
def wbGuard (t : LocalIndex) (e : BEdge) :
    EM (List Instr × LocalIndex) := do
  let g ← alloc .bool
  let pat := bPathPattern e.path
  let test : Instr ←
    match e.parent with
    | .refNode p => pure (.call [g] (.isParent pat) [p, t])
    | .localRoot x => pure (.call [g] (.isMutLoc x) [t])
    | .globalRoot r => pure (.call [g] (.isMutGlobal r) [t])
    | .anyRoot => throw "internal: anyRoot in a borrow graph"
  pure ([test], g)

/-- Emit a guarded write-back for every candidate origin. -/
def emitGuardedWriteBacks (Δ : StructDecls) (d : FunDecl)
    (src : BlockId) (t : LocalIndex) : List BEdge → EM Unit
  | [] => pure ()
  | e :: rest => do
      let (gis, gl) ← wbGuard t e
      emitAll gis
      emitGuarded src gl (← wbBody Δ d t e)
      if let .refNode p := e.parent then markWritten p
      emitGuardedWriteBacks Δ d src t rest

/-- Emit the write-backs of the dying reference `t`: unguarded along a
unique in-edge, otherwise one guarded step per candidate. -/
def emitWriteBacks (Δ : StructDecls) (d : FunDecl) (src : BlockId)
    (g : BGraph) (t : LocalIndex) : EM Unit := do
  match inEdges g t with
  | [] => pure ()
  | [e] => do
      emitAll (← wbBody Δ d t e)
      if let .refNode p := e.parent then markWritten p
  | es => emitGuardedWriteBacks Δ d src t es

/-- Does `t` have a pending derived reference (a checked-out child)?
While it does, `t`'s payload is *stale* below the child's path — using or
copying `t` then would diverge from the source's read-through (the
exclusivity Move's borrow checker enforces). -/
def hasPendingChild (g : BGraph) (pending : List LocalIndex)
    (t : LocalIndex) : Bool :=
  pending.any fun c => c ≠ t && (inEdges g c).any (·.parent == .refNode t)

/-- A negative pending-child test rules out every concrete pending child edge
to that parent. -/
theorem noPendingChild_of_false {g : BGraph} {pending : List LocalIndex}
    {p c : LocalIndex} (h : hasPendingChild g pending p = false)
    (hc : c ∈ pending) (hcp : c ≠ p) {e : BEdge}
    (he : e ∈ inEdges g c) (hparent : e.parent = .refNode p) : False := by
  have hedge : (inEdges g c).any (·.parent == .refNode p) = true :=
    List.any_eq_true.mpr ⟨e, he, by simp [hparent]⟩
  have hchild : pending.any (fun c =>
      c ≠ p && (inEdges g c).any (·.parent == .refNode p)) = true :=
    List.any_eq_true.mpr ⟨c, hc, by simp [hcp, hedge]⟩
  change (pending.any (fun c =>
    c ≠ p && (inEdges g c).any (·.parent == .refNode p))) = false at h
  rw [h] at hchild
  contradiction

/-- Reference-local parents required to write back the mutations in `pending`.
Root parents are locations rather than mutation locals and are therefore not
included. -/
def pendingRefParents (g : BGraph)
    (pending : List LocalIndex) : List LocalIndex :=
  pending.flatMap fun child =>
    (inEdges g child).filterMap fun edge =>
      match edge.parent with
      | .refNode parent => some parent
      | _ => none

/-- Close a pending set over reference-local parents.  At most one new local
can be discovered per useful round, so `numLocals` rounds suffice for the
finite local graph, including cyclic may-graphs. -/
def closePendingParents (g : BGraph) : Nat → List LocalIndex → List LocalIndex
  | 0, pending => pending
  | fuel + 1, pending =>
      closePendingParents g fuel
        ((pendingRefParents g pending ++ pending).eraseDups)

/-- A live reference has an unsafe joined parent when its may-graph names
more than one possible immediate parent and at least one is a reference
local.  Root dispatch is self-contained in the child mutation, but testing a
reference-local parent reads that parent slot; at a join, a parent belonging
to the other branch need not be initialized. -/
def hasJoinedRefParent (g : BGraph) (t : LocalIndex) : Bool :=
  let parents := ((inEdges g t).map (·.parent)).eraseDups
  parents.length > 1 && parents.any fun p =>
    match p with
    | .refNode _ => true
    | _ => false

/-- Reject path-insensitive join graphs for which guarded write-back would
need to inspect a reference-parent local not initialized on every path. -/
def checkJoinedRefParents (d : FunDecl) (live : LiveSet)
    (g : BGraph) : Except String Unit := do
  for t in List.range d.numLocals do
    if isMutLocal d t && live.contains t && hasJoinedRefParent g t then
      throw s!"reference local {t} has branch-dependent intermediate parents"

/-- Process the deaths among `pending` at a point where `liveNow` is the
live set: write back every pending reference that is dead and has no
pending derived reference, children first (the cascade of MVP's
`dying_nodes` ancestor chains). -/
def processDeaths (Δ : StructDecls) (d : FunDecl) (src : BlockId)
    (g : BGraph) (liveNow : LiveSet) :
    List LocalIndex → EM (List LocalIndex)
  | pending => go pending.length pending
where
  go : Nat → List LocalIndex → EM (List LocalIndex)
    | 0, pending => pure pending
    | fuel + 1, pending => do
      match pending.find? fun t =>
          !liveNow.contains t && !hasPendingChild g pending t with
      | none => pure pending
      | some t => do
          emitWriteBacks Δ d src g t
          go fuel (pending.filter (· ≠ t))

/-! ## The rewriting walk -/

/-- Is the local root `x` borrowed (a pending reference derives from
it)? -/
def rootBorrowed (g : BGraph) (pending : List LocalIndex)
    (x : LocalIndex) : Bool :=
  pending.any fun c => (inEdges g c).any (·.parent == .localRoot x)

/-- Is the resource type `r` borrowed? -/
def globalBorrowed (g : BGraph) (pending : List LocalIndex)
    (r : ResourceId) : Bool :=
  pending.any fun c => (inEdges g c).any (·.parent == .globalRoot r)

/-- Is any resource type borrowed? -/
def anyGlobalBorrowed (g : BGraph) (pending : List LocalIndex) :
    Bool :=
  pending.any fun c => (inEdges g c).any fun e =>
    match e.parent with
    | .globalRoot _ => true
    | _ => false

/-- Reject direct uses of borrowed local roots (Move's exclusivity: while
`x` is mutably borrowed, `x` itself is untouchable). -/
def checkRoots (g : BGraph) (pending : List LocalIndex)
    : List LocalIndex → EM Unit
  | [] => pure ()
  | x :: xs => do
      if rootBorrowed g pending x then
        throw s!"local {x} is used while mutably borrowed"
      checkRoots g pending xs

/-- The exclusivity checks of a value operation touching global memory. -/
def checkGlobalOp (g : BGraph) (pending : List LocalIndex) :
    Oper → EM Unit
  | .getGlobal r | .moveFrom r | .writeGlobal r | .mkMutGlobal r => do
      if globalBorrowed g pending r then
        throw s!"resource {r} is accessed while mutably borrowed"
  | _ => pure ()

/-- Raw rewrite of one instruction; returns the updated graph and pending
set.  The public wrapper below enforces that instruction rewriting cannot
close blocks; block splitting belongs exclusively to death processing. -/
def rewriteInstrCore (sums : Summaries) (d : FunDecl) (g : BGraph)
    (pending : List LocalIndex) :
    Instr → EM (BGraph × List LocalIndex)
  | .call [dst] .borrowLoc [x] => do
      if isMutLocal d x then throw "nested references are not supported"
      if !(isMutLocal d dst) then throw "borrow into a non-reference local"
      if isMutParam d dst then
        throw "overwriting a mutable-reference parameter slot is unsupported"
      if rootBorrowed g pending x then
        throw s!"local {x} is borrowed while already mutably borrowed"
      if pending.contains dst then
        throw "re-borrow while derived references are live"
      emit (.call [dst] (.mkMutLoc x) [x])
      clearWritten dst
      pure (gReplaceChild ⟨.localRoot x, dst, []⟩ g, dst :: pending)
  | .call [dst] (.borrowGlobal r) [aT] => do
      if !(isMutLocal d dst) then throw "borrow into a non-reference local"
      if isMutParam d dst then
        throw "overwriting a mutable-reference parameter slot is unsupported"
      if globalBorrowed g pending r then
        throw s!"resource {r} is borrowed while already mutably borrowed"
      if pending.contains dst then
        throw "re-borrow while derived references are live"
      checkRoots g pending [aT]
      emit (.call [dst] (.mkMutGlobal r) [aT])
      clearWritten dst
      pure (gReplaceChild ⟨.globalRoot r, dst, []⟩ g, dst :: pending)
  | .call [dst] (.borrowField i) [t] => do
      if !(isMutLocal d t && isMutLocal d dst) then
        throw "field borrow outside the reference discipline"
      if isMutParam d dst then
        throw "overwriting a mutable-reference parameter slot is unsupported"
      if pending.contains dst then
        throw "re-borrow while derived references are live"
      -- sibling borrows are fine only along statically disjoint fields
      if pending.any (fun c => c ≠ dst &&
          (inEdges g c).any fun e => e.parent == .refNode t &&
            (match e.path with
             | .field j :: _ => j == i
             | _ => true)) then
        throw "overlapping borrow while a derived reference is live"
      emit (.call [dst] (.childMutField i) [t])
      clearWritten dst
      pure (gReplaceChild ⟨.refNode t, dst, [.field i]⟩ g, dst :: pending)
  | .call [dst] .borrowVecElem [t, iT] => do
      if !(isMutLocal d t && isMutLocal d dst) then
        throw "vector element borrow outside the reference discipline"
      if isMutParam d dst then
        throw "overwriting a mutable-reference parameter slot is unsupported"
      if pending.contains dst then
        throw "re-borrow while derived references are live"
      if hasPendingChild g pending t then
        throw "overlapping borrow while a derived reference is live"
      checkRoots g pending [iT]
      emit (.call [dst] .childMutIndex [t, iT])
      clearWritten dst
      pure (gReplaceChild ⟨.refNode t, dst, [.index]⟩ g, dst :: pending)
  | .call [dst] .readRef [t] => do
      if !(isMutLocal d t) then throw "read through a non-reference"
      if isMutLocal d dst then throw "nested references are not supported"
      if hasPendingChild g pending t then
        throw "reference used while a derived reference is live"
      emit (.call [dst] .getMut [t])
      pure (g, pending)
  | .call [] .writeRef [t, vt] => do
      if !(isMutLocal d t) then throw "write through a non-reference"
      if isMutLocal d vt then
        throw "reference stored through a reference"
      if hasPendingChild g pending t then
        throw "reference used while a derived reference is live"
      checkRoots g pending [vt]
      emit (.call [t] .setMut [t, vt])
      markWritten t
      pure (g, pending)
  | .call _ .freezeRef _ =>
      throw "internal: freeze survived the imm pass"
  | .call _ .borrowLoc _ | .call _ (.borrowGlobal _) _
  | .call _ (.borrowField _) _ | .call _ .borrowVecElem _
  | .call _ .readRef _ | .call _ .writeRef _ =>
      throw "malformed reference operation"
  | .assign dst src' => do
      if isMutLocal d src' then
        if !(isMutLocal d dst) then
          throw "reference assigned to a non-reference local"
        if isMutParam d dst then
          throw "overwriting a mutable-reference parameter slot is unsupported"
        if pending.contains dst then
          throw "re-borrow while derived references are live"
        if hasPendingChild g pending src' then
          throw "reference used while a derived reference is live"
        emit (.assign dst src')
        clearWritten dst
        pure (gReplaceChild ⟨.refNode src', dst, []⟩ g, dst :: pending)
      else do
        if isMutLocal d dst then
          throw "value assigned to a reference local"
        checkRoots g pending [src', dst]
        emit (.assign dst src')
        pure (g, pending)
  | .load dst v => do
      if isMutLocal d dst then throw "constant loaded into a reference local"
      if !v.refFree then throw "reference constant"
      checkRoots g pending [dst]
      emit (.load dst v)
      pure (g, pending)
  | .nop => do
      emit .nop
      pure (g, pending)
  | .call dsts (.function f) srcs => do
      if anyGlobalBorrowed g pending then
        throw "call while a global borrow is live"
      if !(srcs ++ dsts).any (isMutLocal d) then
        -- no references cross this boundary
        checkRoots g pending (srcs ++ dsts)
        emit (.call dsts (.function f) srcs)
        pure (g, pending)
      else do
        -- MVP's `call r := f(r)`: mutation arguments are passed as
        -- values and received back as finals (extra destinations); the
        -- callee's summary instantiates the derivations of returned
        -- references into the caller's graph
        let some sum := sums f
          | throw "callee summary unavailable for a reference-passing call"
        let mutArgs ← sum.mutParams.mapM fun p => do
          let some src := srcs[p]?
            | throw "reference-passing call with too few arguments"
          if !(isMutLocal d src) then
            throw "a non-reference argument for a &mut parameter"
          if hasPendingChild g pending src then
            throw "reference used while a derived reference is live"
          pure src
        if mutArgs.eraseDups.length != mutArgs.length then
          throw "the same mutable reference is passed to multiple &mut parameters"
        checkRoots g pending
          ((srcs.filter (!isMutLocal d ·)) ++ dsts.filter (!isMutLocal d ·))
        let mut g' := g
        let mut pending' := pending
        for (dst, derivs) in dsts.zip sum.retDerivs do
          if isMutLocal d dst then do
            if isMutParam d dst then
              throw "overwriting a mutable-reference parameter slot is unsupported"
            if pending.contains dst then
              throw "re-borrow while derived references are live"
            if derivs.isEmpty then
              throw "a returned reference has no summarized derivation"
            g' := gRemoveChild dst g'
            for (argPos, path) in derivs do
              let some src := srcs[argPos]?
                | throw "reference-passing call with too few arguments"
              g' := gInsert ⟨.refNode src, dst, path⟩ g'
            pending' := dst :: pending'
        emit (.call (dsts ++ mutArgs) (.function f) srcs)
        for a in mutArgs do
          markWritten a
        for dst in dsts do
          if isMutLocal d dst then clearWritten dst
        pure (g', pending')
  | .call dsts op srcs => do
      -- `eq` dereferences mutations (like references); everything else
      -- must not consume them
      if op != .eq && (srcs ++ dsts).any (isMutLocal d) then
        throw "reference consumed by an unsupported operation"
      if op == .eq && dsts.any (isMutLocal d) then
        throw "reference produced by an unsupported operation"
      if op == .eq &&
          srcs.any (fun t => isMutLocal d t &&
            hasPendingChild g pending t) then
        throw "reference used while a derived reference is live"
      checkGlobalOp g pending op
      checkRoots g pending ((srcs.filter (!isMutLocal d ·)) ++ dsts)
      emit (.call dsts op srcs)
      pure (g, pending)

/-- Run an emitter computation while retaining the caller's finished-block
trace.  Used at boundaries which are not permitted to close blocks. -/
def EM.keepDone {α : Type} (x : EM α) : EM α := fun s =>
  match x s with
  | .error e => .error e
  | .ok (a, s') => .ok (a, { s' with done := s.done })

/-- Rewrite one instruction without changing the finished-block trace or
leaving its active block.  The wrapper retains the existing prefix and only
keeps the raw rewrite's newly appended suffix, making the cursor contract
structural rather than a case-by-case proof obligation. -/
def rewriteInstr (sums : Summaries) (d : FunDecl) (g : BGraph)
    (pending : List LocalIndex) (i : Instr) : EM (BGraph × List LocalIndex) :=
  fun st =>
    match rewriteInstrCore sums d g pending i st with
    | .error e => .error e
    | .ok (result, st') => .ok (result, { st' with
        done := st.done
        curId := st.curId
        cur := st.cur ++ st'.cur.drop st.cur.length })

/-- Rewrite instruction/live-after points in order, including every
death-triggered write-back between adjacent source instructions.  Naming this
recursive phase gives correctness proofs a per-instruction certificate
boundary without changing the emitter state or output. -/
def rewriteCoreInstrs (sums : Summaries) (Δ : StructDecls) (d : FunDecl)
    (src : BlockId) : List (Instr × LiveSet) → BGraph →
    List LocalIndex → EM (BGraph × List LocalIndex)
  | [], g, pending => pure (g, pending)
  | (i, liveAfter) :: rest, g, pending => do
      let (g', pending') ← rewriteInstr sums d g pending i
      let pending'' ← processDeaths Δ d src g' liveAfter pending'
      rewriteCoreInstrs sums Δ d src rest g' pending''

/-- Emit a branch-edge block when references die on only that successor.
The original block cursor is restored after the synthetic block is closed. -/
def splitCoreEdge (Δ : StructDecls) (d : FunDecl)
    (liveIn : Array LiveSet) (b : BlockId) (g : BGraph)
    (pending : List LocalIndex) (target : BlockId) : EM BlockId := do
  if pending.all (fun t => (liveIn.getD target ∅).contains t) then
    pure target
  else do
    let w ← newBlockId b
    let saved ← get
    modify fun s => { s with curId := w, cur := [] }
    let pending' ← processDeaths Δ d b g (liveIn.getD target ∅) pending
    let _ := pending'
    closeBlock (.jump target) 0
    modify fun s => { s with curId := saved.curId, cur := saved.cur }
    pure w

/-- Finish an emitted source block after its instruction points have been
rewritten.  This phase contains terminator deaths and any required edge
splitting, and is separate so correctness proofs can certify instructions and
terminators independently. -/
def finishCoreBlock (Δ : StructDecls) (d : FunDecl) (liveIn : Array LiveSet)
    (b : BlockId) (g : BGraph) (pending : List LocalIndex) :
    Term → EM Unit
  | .ret srcs => do
      let keep := srcs.filter (isMutLocal d) ++ mutParamsOf d
      let pending' ← processDeaths Δ d b g (LiveSet.ofList keep) pending
      let written := (← get).written
      if pending'.any (fun t => !keep.contains t && written.contains t) then
        throw "an intermediate borrow escapes through a returned reference"
      closeBlock (.ret (srcs ++ mutParamsOf d)) 0
  | .abort c => do
      closeBlock (.abort c) 0
  | .jump b' => do
      let pending' ← processDeaths Δ d b g (liveIn.getD b' ∅) pending
      let _ := pending'
      closeBlock (.jump b') 0
  | .branch c b₁ b₂ => do
      checkRoots g pending [c]
      let liveBoth := (liveIn.getD b₁ ∅).union (liveIn.getD b₂ ∅)
      let pendingB ← processDeaths Δ d b g liveBoth pending
      let t₁ ← splitCoreEdge Δ d liveIn b g pendingB b₁
      let t₂ ← splitCoreEdge Δ d liveIn b g pendingB b₂
      closeBlock (.branch c t₁ t₂) 0

/-- Mutation locals that may already be checked out on entry to a source
block. -/
def coreEntryPending (d : FunDecl) (liveIn : Array LiveSet)
    (b : BlockId) (g : BGraph) : List LocalIndex :=
  closePendingParents g d.numLocals <|
    (List.range d.numLocals).filter fun t =>
    isMutLocal d t && (liveIn.getD b ∅).contains t &&
      (!(inEdges g t).isEmpty || t < d.numParams)

/-- Reset the block-local emitter fields before rewriting a source block. -/
def prepareCoreBlock (d : FunDecl) (liveIn : Array LiveSet)
    (b : BlockId) (g : BGraph) (st : EmitSt) : EmitSt :=
  { st with curId := b, cur := [], written := coreEntryPending d liveIn b g }

/-- Whether the emitter has finished a block with the given identifier. -/
def blockEmitted (b : BlockId) (st : EmitSt) : Bool :=
  st.done.any fun p => p.1 == b

/-- A positive emitted-block check exposes the exact recorded block. -/
theorem blockEmitted_mem {b : BlockId} {st : EmitSt}
    (h : blockEmitted b st = true) :
    ∃ blk, (b, blk) ∈ st.done := by
  obtain ⟨p, hp, hid⟩ := List.any_eq_true.mp h
  have heq : p.1 = b := by simpa using hid
  obtain ⟨id, blk⟩ := p
  simp only at heq
  subst id
  exact ⟨blk, hp⟩

/-- Validate the phase contract that rewriting a declared block finished its
entry segment, even when later write-backs continued in split blocks. -/
def ensureBlockEmitted (b : BlockId) : EM Unit := fun st =>
  if blockEmitted b st then .ok ((), st)
  else .error "internal: source block was not emitted"

/-- Rewrite one source block: walk the instructions (with per-point death
processing), then handle terminator-edge deaths — uniformly before the
terminator where possible, in edge-split blocks otherwise. -/
def rewriteBlock (sums : Summaries) (Δ : StructDecls) (d : FunDecl)
    (liveIn : Array LiveSet) (b : BlockId) (blk : Block) (g₀ : BGraph) :
    EM Unit := do
  EM.lift (checkJoinedRefParents d (liveIn.getD b ∅) g₀)
  modify (prepareCoreBlock d liveIn b g₀)
  -- Pending references are borrowed incoming values or `&mut` parameters.
  let pending₀ := coreEntryPending d liveIn b g₀
  -- live-after sets, per instruction
  let liveAtTerm := liveAtTermIn liveIn blk
  let (g, pending) ← rewriteCoreInstrs sums Δ d b
    (blk.instrs.zip (liveAfterEach liveAtTerm blk.instrs)) g₀ pending₀
  finishCoreBlock Δ d liveIn b g pending blk.term
  ensureBlockEmitted b

/-! ## Append-only emitter law -/

/-- A stateful emitter computation never removes a previously finished
block.  This is the sole state-monad law needed to connect intermediate
rewrite certificates to the final dense CFG. -/
def PreservesDone {α : Type}
    (x : StateT EmitSt (Except String) α) : Prop :=
  ∀ s a s', x s = .ok (a, s') →
    ∀ p ∈ s.done, p ∈ s'.done

namespace PreservesDone

/-- Pure state computations preserve finished blocks. -/
theorem pure {α : Type} (a : α) :
    PreservesDone (pure a : StateT EmitSt (Except String) α) := by
  intro s _ s' h p hp
  change Except.ok (a, s) = Except.ok (_, s') at h
  cases h
  exact hp

/-- Append-only computations compose through monadic bind. -/
theorem bind {α β : Type} {x : StateT EmitSt (Except String) α}
    {k : α → StateT EmitSt (Except String) β}
    (hx : PreservesDone x) (hk : ∀ a, PreservesDone (k a)) :
    PreservesDone (x >>= k) := by
  intro s b s' h p hp
  change (x s).bind (fun result => k result.1 result.2) =
    Except.ok (b, s') at h
  obtain ⟨result, hxrun, hkrun⟩ := Except.bind_ok_inv h
  obtain ⟨a, s₁⟩ := result
  exact hk a s₁ b s' hkrun p (hx s a s₁ hxrun p hp)

/-- Reading the emitter state preserves finished blocks. -/
theorem get : PreservesDone (get : StateT EmitSt (Except String) EmitSt) := by
  intro s _ s' h p hp
  change Except.ok (s, s) = Except.ok (_, s') at h
  cases h
  exact hp

/-- A direct state modification preserves finished blocks when its projection
does so pointwise. -/
theorem modify (f : EmitSt → EmitSt)
    (hf : ∀ s p, p ∈ s.done → p ∈ (f s).done) :
    PreservesDone (modify f : StateT EmitSt (Except String) Unit) := by
  intro s _ s' h p hp
  change Except.ok ((), f s) = Except.ok (_, s') at h
  cases h
  exact hf s p hp

/-- An immediately failing computation vacuously preserves state. -/
theorem error {α : Type} (e : String) :
    PreservesDone (fun _ => Except.error e :
      StateT EmitSt (Except String) α) := by
  intro _ _ _ h
  cases h

end PreservesDone

/-- Fresh-local allocation does not change finished blocks. -/
theorem alloc_preservesDone (ty : Ty) : PreservesDone (alloc ty) := by
  intro s _ s' h p hp
  simp only [alloc, get, bind, StateT.bind, Except.bind,
    set, StateT.set, pure, StateT.pure, Except.pure] at h
  cases h
  exact hp

/-- Appending an instruction does not change finished blocks. -/
theorem emit_preservesDone (i : Instr) : PreservesDone (emit i) :=
  PreservesDone.modify _ (by simp)

/-- Mutation bookkeeping does not change finished blocks. -/
theorem markWritten_preservesDone (t : LocalIndex) :
    PreservesDone (markWritten t) :=
  PreservesDone.modify _ (by intros; split <;> assumption)

/-- Clearing mutation bookkeeping does not change finished blocks. -/
theorem clearWritten_preservesDone (t : LocalIndex) :
    PreservesDone (clearWritten t) :=
  PreservesDone.modify _ (by simp)

/-- Appending several instructions does not change finished blocks. -/
theorem emitAll_preservesDone (is : List Instr) :
    PreservesDone (emitAll is) :=
  PreservesDone.modify _ (by simp)

/-- Fresh block allocation does not change finished blocks. -/
theorem newBlockId_preservesDone (src : BlockId) :
    PreservesDone (newBlockId src) := by
  intro s _ s' h p hp
  simp only [newBlockId, get, bind, StateT.bind, Except.bind,
    set, StateT.set, pure, StateT.pure, Except.pure] at h
  cases h
  exact hp

/-- Closing a block prepends one entry and preserves all older entries. -/
theorem closeBlock_preservesDone (term : Term) (contId : BlockId) :
    PreservesDone (closeBlock term contId) :=
  PreservesDone.modify _ (by
    intro s p hp
    exact List.mem_cons_of_mem _ hp)

/-- A lifted error computation never changes emitter state. -/
theorem EM.lift_preservesDone {α : Type} (x : Except String α) :
    PreservesDone (EM.lift x) := by
  intro s a s' h p hp
  unfold EM.lift at h
  cases hx : x with
  | error e => rw [hx] at h; cases h
  | ok v =>
      simp [hx] at h
      obtain ⟨rfl, rfl⟩ := h
      exact hp

/-- Guarded emission only prepends its branch and body blocks. -/
theorem emitGuarded_preservesDone (src : BlockId) (g : LocalIndex)
    (body : List Instr) : PreservesDone (emitGuarded src g body) := by
  apply PreservesDone.bind (newBlockId_preservesDone src)
  intro doId
  apply PreservesDone.bind (newBlockId_preservesDone src)
  intro contId
  apply PreservesDone.bind (closeBlock_preservesDone
    (.branch g doId contId) contId)
  intro _
  exact PreservesDone.modify _ (by
    intro s p hp
    exact List.mem_cons_of_mem _ hp)

/-- Functional-update code generation only allocates locals. -/
theorem buildUpdate_preservesDone (Δ : StructDecls)
    (p t : LocalIndex) : ∀ k cur curTy path,
    PreservesDone (buildUpdate Δ p t k cur curTy path)
  | k, cur, curTy, [] => by
      rw [buildUpdate]
      apply PreservesDone.bind (alloc_preservesDone curTy)
      intro _
      exact PreservesDone.pure _
  | k, cur, curTy, .field i :: rest => by
      rw [buildUpdate]
      apply PreservesDone.bind (EM.lift_preservesDone (fieldTy Δ curTy i))
      intro ft
      apply PreservesDone.bind (alloc_preservesDone ft)
      intro sub
      apply PreservesDone.bind
        (buildUpdate_preservesDone Δ p t (k + 1) sub ft rest)
      intro result
      apply PreservesDone.bind (alloc_preservesDone curTy)
      intro _
      exact PreservesDone.pure _
  | k, cur, curTy, .index :: rest => by
      rw [buildUpdate]
      apply PreservesDone.bind (EM.lift_preservesDone (elemTy curTy))
      intro et
      apply PreservesDone.bind (alloc_preservesDone .u64)
      intro _
      apply PreservesDone.bind (alloc_preservesDone et)
      intro sub
      apply PreservesDone.bind
        (buildUpdate_preservesDone Δ p t (k + 1) sub et rest)
      intro result
      apply PreservesDone.bind (alloc_preservesDone curTy)
      intro _
      exact PreservesDone.pure _

/-- Generating one write-back body only allocates locals. -/
theorem wbBody_preservesDone (Δ : StructDecls) (d : FunDecl)
    (t : LocalIndex) (e : BEdge) : PreservesDone (wbBody Δ d t e) := by
  cases e with
  | mk parent child path =>
    cases parent with
    | refNode p =>
      simp only [wbBody]
      apply PreservesDone.bind (EM.lift_preservesDone (mutPayloadTy d p))
      intro tp
      cases path with
      | nil =>
        apply PreservesDone.bind
          (EM.lift_preservesDone (mutPayloadTy d t))
        intro tc
        apply PreservesDone.bind (alloc_preservesDone tc)
        intro _
        exact PreservesDone.pure _
      | cons step rest =>
        apply PreservesDone.bind (alloc_preservesDone tp)
        intro a
        apply PreservesDone.bind
          (buildUpdate_preservesDone Δ p t 0 a tp (step :: rest))
        intro _
        exact PreservesDone.pure _
    | localRoot x =>
      simp only [wbBody]
      split
      · exact PreservesDone.error _
      · exact PreservesDone.pure _
    | globalRoot r =>
      simp only [wbBody]
      split
      · exact PreservesDone.error _
      · apply PreservesDone.bind
          (EM.lift_preservesDone (mutPayloadTy d t))
        intro tc
        apply PreservesDone.bind (alloc_preservesDone .address)
        intro _
        apply PreservesDone.bind (alloc_preservesDone tc)
        intro _
        exact PreservesDone.pure _
    | anyRoot =>
      simp only [wbBody]
      exact PreservesDone.error _

/-- Generating a dynamic write-back guard only allocates its Boolean local. -/
theorem wbGuard_preservesDone (t : LocalIndex) (e : BEdge) :
    PreservesDone (wbGuard t e) := by
  unfold wbGuard
  apply PreservesDone.bind (alloc_preservesDone .bool)
  intro _
  cases e.parent <;>
    first | exact PreservesDone.pure _ | exact PreservesDone.error _

/-- A list of guarded candidate write-backs only extends finished blocks. -/
theorem emitGuardedWriteBacks_preservesDone (Δ : StructDecls)
    (d : FunDecl) (src : BlockId) (t : LocalIndex) : ∀ es,
    PreservesDone (emitGuardedWriteBacks Δ d src t es)
  | [] => PreservesDone.pure ()
  | e :: rest => by
      rw [emitGuardedWriteBacks]
      apply PreservesDone.bind (wbGuard_preservesDone t e)
      intro guard
      apply PreservesDone.bind (emitAll_preservesDone guard.1)
      intro _
      apply PreservesDone.bind (wbBody_preservesDone Δ d t e)
      intro body
      apply PreservesDone.bind
        (emitGuarded_preservesDone src guard.2 body)
      intro _
      cases e.parent with
      | refNode p =>
          apply PreservesDone.bind (markWritten_preservesDone p)
          intro _
          exact emitGuardedWriteBacks_preservesDone Δ d src t rest
      | _ => exact emitGuardedWriteBacks_preservesDone Δ d src t rest

/-- Emitting all candidate write-backs only extends finished blocks. -/
theorem emitWriteBacks_preservesDone (Δ : StructDecls) (d : FunDecl)
    (src : BlockId) (g : BGraph) (t : LocalIndex) :
    PreservesDone (emitWriteBacks Δ d src g t) := by
  unfold emitWriteBacks
  cases hedges : inEdges g t with
  | nil => exact PreservesDone.pure ()
  | cons e rest =>
    cases rest with
    | nil =>
      apply PreservesDone.bind (wbBody_preservesDone Δ d t e)
      intro body
      apply PreservesDone.bind (emitAll_preservesDone body)
      intro _
      cases e.parent with
      | refNode p => exact markWritten_preservesDone p
      | _ => exact PreservesDone.pure ()
    | cons e' rest =>
      exact emitGuardedWriteBacks_preservesDone Δ d src t
        (e :: e' :: rest)

/-- Each fuelled death-processing iteration only extends finished blocks. -/
theorem processDeaths_go_preservesDone (Δ : StructDecls) (d : FunDecl)
    (src : BlockId) (g : BGraph) (liveNow : LiveSet) :
    ∀ fuel pending,
      PreservesDone (processDeaths.go Δ d src g liveNow fuel pending)
  | 0, pending => by
      rw [processDeaths.go]
      exact PreservesDone.pure pending
  | fuel + 1, pending => by
      rw [processDeaths.go]
      cases hfind : pending.find? (fun t =>
          !liveNow.contains t && !hasPendingChild g pending t) with
      | none => exact PreservesDone.pure pending
      | some t =>
        apply PreservesDone.bind
          (emitWriteBacks_preservesDone Δ d src g t)
        intro _
        exact processDeaths_go_preservesDone Δ d src g liveNow fuel
          (pending.filter (· ≠ t))

/-- Death processing is append-only on the finished-block trace. -/
theorem processDeaths_preservesDone (Δ : StructDecls) (d : FunDecl)
    (src : BlockId) (g : BGraph) (liveNow : LiveSet)
    (pending : List LocalIndex) :
    PreservesDone (processDeaths Δ d src g liveNow pending) := by
  unfold processDeaths
  exact processDeaths_go_preservesDone Δ d src g liveNow
    pending.length pending

/-- Root-exclusivity checks never modify emitter state. -/
theorem checkRoots_preservesDone (g : BGraph)
    (pending : List LocalIndex) : ∀ xs,
    PreservesDone (checkRoots g pending xs)
  | [] => PreservesDone.pure ()
  | x :: xs => by
      rw [checkRoots]
      split
      · exact PreservesDone.error _
      · exact checkRoots_preservesDone g pending xs

/-- Global-exclusivity checks never modify emitter state. -/
theorem checkGlobalOp_preservesDone (g : BGraph)
    (pending : List LocalIndex) (op : Oper) :
    PreservesDone (checkGlobalOp g pending op) := by
  cases op <;> simp only [checkGlobalOp] <;>
    first
    | exact PreservesDone.pure ()
    | (split <;>
        first | exact PreservesDone.error _ | exact PreservesDone.pure ())

/-- Rewriting one source instruction only extends finished blocks. -/
theorem rewriteInstr_preservesDone (sums : Summaries) (d : FunDecl)
    (g : BGraph) (pending : List LocalIndex) (i : Instr) :
    PreservesDone (rewriteInstr sums d g pending i) := by
  intro s a s' h p hp
  unfold rewriteInstr at h
  cases hr : rewriteInstrCore sums d g pending i s with
  | error e => simp [hr] at h
  | ok result =>
      obtain ⟨value, next⟩ := result
      simp only [hr, Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, rfl⟩ := h
      exact hp

/-- Rewriting a sequence of instruction points is append-only on finished
blocks. -/
theorem rewriteCoreInstrs_preservesDone (sums : Summaries)
    (Δ : StructDecls) (d : FunDecl) (src : BlockId) :
    ∀ points g pending,
      PreservesDone (rewriteCoreInstrs sums Δ d src points g pending)
  | [], g, pending => PreservesDone.pure (g, pending)
  | (i, liveAfter) :: rest, g, pending => by
      rw [rewriteCoreInstrs]
      apply PreservesDone.bind
        (rewriteInstr_preservesDone sums d g pending i)
      intro result
      obtain ⟨g', pending'⟩ := result
      apply PreservesDone.bind
        (processDeaths_preservesDone Δ d src g' liveAfter pending')
      intro pending''
      exact rewriteCoreInstrs_preservesDone sums Δ d src rest g' pending''

/-- Splitting one branch edge only appends its optional synthetic block. -/
theorem splitCoreEdge_preservesDone (Δ : StructDecls) (d : FunDecl)
    (liveIn : Array LiveSet) (b : BlockId) (g : BGraph)
    (pending : List LocalIndex) (target : BlockId) :
    PreservesDone (splitCoreEdge Δ d liveIn b g pending target) := by
  unfold splitCoreEdge
  split
  · exact PreservesDone.pure target
  · apply PreservesDone.bind (newBlockId_preservesDone b)
    intro _
    apply PreservesDone.bind PreservesDone.get
    intro saved
    apply PreservesDone.bind
      (PreservesDone.modify _ (by simp))
    intro _
    apply PreservesDone.bind
      (processDeaths_preservesDone Δ d b g
        (liveIn.getD target ∅) pending)
    intro _
    apply PreservesDone.bind (closeBlock_preservesDone (.jump target) 0)
    intro _
    apply PreservesDone.bind
      (PreservesDone.modify _ (by simp))
    intro _
    exact PreservesDone.pure _

/-- Terminator rewriting, including successor-specific edge blocks, is
append-only on finished blocks. -/
theorem finishCoreBlock_preservesDone (Δ : StructDecls) (d : FunDecl)
    (liveIn : Array LiveSet) (b : BlockId) (g : BGraph)
    (pending : List LocalIndex) (term : Term) :
    PreservesDone (finishCoreBlock Δ d liveIn b g pending term) := by
  cases term with
  | ret srcs =>
      simp only [finishCoreBlock]
      apply PreservesDone.bind
        (processDeaths_preservesDone Δ d b g
          (LiveSet.ofList (srcs.filter (isMutLocal d) ++ mutParamsOf d))
          pending)
      intro pending'
      apply PreservesDone.bind PreservesDone.get
      intro st
      split
      · exact PreservesDone.error _
      · exact closeBlock_preservesDone
          (.ret (srcs ++ mutParamsOf d)) 0
  | abort c => exact closeBlock_preservesDone (.abort c) 0
  | jump target =>
      simp only [finishCoreBlock]
      apply PreservesDone.bind
        (processDeaths_preservesDone Δ d b g
          (liveIn.getD target ∅) pending)
      intro _
      exact closeBlock_preservesDone (.jump target) 0
  | branch c left right =>
      simp only [finishCoreBlock]
      apply PreservesDone.bind (checkRoots_preservesDone g pending [c])
      intro _
      apply PreservesDone.bind
        (processDeaths_preservesDone Δ d b g
          ((liveIn.getD left ∅).union (liveIn.getD right ∅)) pending)
      intro pendingB
      apply PreservesDone.bind
        (splitCoreEdge_preservesDone Δ d liveIn b g pendingB left)
      intro left'
      apply PreservesDone.bind
        (splitCoreEdge_preservesDone Δ d liveIn b g pendingB right)
      intro right'
      exact closeBlock_preservesDone (.branch c left' right') 0

/-- Checking that a source block was emitted does not alter emitter state. -/
theorem ensureBlockEmitted_preservesDone (b : BlockId) :
    PreservesDone (ensureBlockEmitted b) := by
  intro st a st' h p hp
  unfold ensureBlockEmitted at h
  split at h
  · cases h
    exact hp
  · cases h

/-- Rewriting one declared source block only appends finished blocks. -/
theorem rewriteBlock_preservesDone (sums : Summaries) (Δ : StructDecls)
    (d : FunDecl) (liveIn : Array LiveSet) (b : BlockId) (blk : Block)
    (g₀ : BGraph) :
    PreservesDone (rewriteBlock sums Δ d liveIn b blk g₀) := by
  unfold rewriteBlock
  apply PreservesDone.bind
    (EM.lift_preservesDone
      (checkJoinedRefParents d (liveIn.getD b ∅) g₀))
  intro _
  apply PreservesDone.bind
    (PreservesDone.modify _ (by simp [prepareCoreBlock]))
  intro _
  apply PreservesDone.bind
    (rewriteCoreInstrs_preservesDone sums Δ d b
      (blk.instrs.zip (liveAfterEach (liveAtTermIn liveIn blk) blk.instrs))
      g₀ (coreEntryPending d liveIn b g₀))
  intro result
  obtain ⟨g, pending⟩ := result
  apply PreservesDone.bind
    (finishCoreBlock_preservesDone Δ d liveIn b g pending blk.term)
  intro _
  exact ensureBlockEmitted_preservesDone b

/-! ## The pass -/

/-- The first-order result of the core elimination — everything a
consumer needs to rebuild either a `FunDecl` or the frontend's list-based
declaration forms (`Frontend/Elim.lean`). -/
structure ElimOut where
  numParams : Nat
  /-- Declared types of all locals, original first, fresh appended. -/
  localTys : List Ty
  /-- Original returns plus the `&mut`-parameter finals. -/
  returns : List Ty
  /-- Dense: block `b` of the output is `blocks[b]`. -/
  blocks : List Block
  entry : BlockId
  /-- Split-off block ↦ the source block it derives from (loop
  membership). -/
  blockSrc : List (BlockId × BlockId)
  contract : Contract

/-- Initial state of the core emitter.  Naming this boundary lets correctness
proofs state block-traversal certificates without duplicating the emitter
record literal. -/
def initialEmitState (d : FunDecl) : EmitSt :=
  { elim := ⟨d.numLocals, []⟩
    nextId := d.body.size
    done := []
    blockSrc := []
    curId := 0
    cur := []
    written := [] }

/-- Traverse source block identifiers and emit every declared block.  This is
the proof-visible boundary between analysis and the stateful block rewriter. -/
def emitCoreBlocks (sums : Summaries) (Δ : StructDecls) (d : FunDecl)
    (liveIn : Array LiveSet) (graphs : Array BGraph)
    (bs : List BlockId) : StateT EmitSt (Except String) Unit := do
  match bs with
  | [] => pure ()
  | b :: rest =>
    match d.body.blocks b with
    | none => pure ()
    | some blk => rewriteBlock sums Δ d liveIn b blk (graphs.getD b [])
    emitCoreBlocks sums Δ d liveIn graphs rest

/-- Traversing any list of source block identifiers only appends finished
blocks. -/
theorem emitCoreBlocks_preservesDone (sums : Summaries) (Δ : StructDecls)
    (d : FunDecl) (liveIn : Array LiveSet) (graphs : Array BGraph) :
    ∀ bs, PreservesDone (emitCoreBlocks sums Δ d liveIn graphs bs)
  | [] => PreservesDone.pure ()
  | b :: rest => by
      rw [emitCoreBlocks]
      cases hblk : d.body.blocks b with
      | none =>
          simp only
          exact emitCoreBlocks_preservesDone sums Δ d liveIn graphs rest
      | some blk =>
          simp only
          apply PreservesDone.bind
            (rewriteBlock_preservesDone sums Δ d liveIn b blk
              (graphs.getD b []))
          intro _
          exact emitCoreBlocks_preservesDone sums Δ d liveIn graphs rest

/-- Select one block while densifying emitter output. -/
def denseCoreBlock (d : FunDecl) (done : List (BlockId × Block))
    (b : BlockId) : Except String Block :=
  match done.find? (·.1 == b) with
  | some (_, blk) => pure blk
  | none =>
      if b < d.body.size && (d.body.blocks b).isNone then
        pure ⟨[], .jump b⟩
      else throw "internal: an output block id was not emitted"

/-- Densify the finished emitter blocks, preserving undeclared source block
ids as unreachable gaps.  Emitted instruction lists are retained verbatim so
the executable CFG and the core correctness certificate share one boundary. -/
def denseCoreBlocks (d : FunDecl) (st : EmitSt) : Except String (List Block) :=
  (List.range (max d.body.size st.nextId)).mapM (denseCoreBlock d st.done)

/-- Check that the append-only emitter trace contains no duplicate block
identifiers.  Densification can then select every recorded block verbatim. -/
def emittedBlocksValid : List (BlockId × Block) → Bool
  | [] => true
  | p :: rest =>
      !rest.any (fun q => q.1 == p.1) && emittedBlocksValid rest

/-- The locals written by an output block (loop-target extension). -/
def ElimOut.defsIn (out : ElimOut) (bid : BlockId) : List LocalIndex :=
  ((out.blocks[bid]?).map (fun blk => blk.instrs.flatMap instrDefs)).getD []

/-- Eliminate references from one *post-imm* declaration under a summary
table (see the module docs), into the first-order result form. -/
def elimCoreOut (sums : Summaries) (Δ : StructDecls) (d : FunDecl) :
    Except String ElimOut := do
  -- reject nested reference declarations outright
  for x in List.range d.numLocals do
    match d.locals x with
    | some (.mutRef t) =>
        if t.isRef then throw "nested references are not supported"
    | _ => pure ()
  let liveIn := liveAnalysis d
  let graphs := borrowAnalysis sums d
  unless liveStable d liveIn do
    throw "the liveness analysis does not converge"
  unless graphStable d (graphThroughBlock sums d) graphs do
    throw "the borrow-graph analysis does not converge"
  let (_, st) ← emitCoreBlocks sums Δ d liveIn graphs
    (List.range d.body.size) (initialEmitState d)
  let dense ← denseCoreBlocks d st
  unless emittedBlocksValid st.done do
    throw "internal: duplicate emitted block identifier"
  let origTys ← (List.range d.numLocals).mapM (localTy d)
  -- the finals of the ret extension: every `&mut` parameter is also
  -- returned (MVP's `call r := f(r)`)
  let finalsTys ← (mutParamsOf d).mapM (localTy d)
  pure { numParams := d.numParams
         localTys := origTys ++ st.elim.newTys
         returns := d.returns ++ finalsTys
         blocks := dense
         entry := d.body.entry
         blockSrc := st.blockSrc
         contract := d.contract }

/-- Rebuild a `FunDecl` from the first-order result, extending the given
loop metadata: split-off blocks inherit their source block's memberships,
and the targets gain everything the inserted code writes inside. -/
def ElimOut.toFunDecl (out : ElimOut)
    (loopSpecs : BlockId → Option LoopSpec) : FunDecl where
  numParams := out.numParams
  numLocals := out.localTys.length
  locals := fun t => out.localTys[t]?
  returns := out.returns
  body :=
    { blocks := fun b => out.blocks[b]?
      entry := out.entry
      size := out.blocks.length }
  loopSpecs := fun h =>
    (loopSpecs h).map fun spec =>
      let members' : BlockId → Prop := fun b' =>
        spec.members b' ∨ ∃ p ∈ out.blockSrc, p.1 = b' ∧ spec.members p.2
      { spec with
        members := members'
        valTargets := fun x => spec.valTargets x ∨
          ∃ bid, members' bid ∧ x ∈ out.defsIn bid }
  contract := out.contract

/-- Eliminate references from one *post-imm* declaration under a summary
table (see the module docs). -/
def elimCore (sums : Summaries) (Δ : StructDecls) (d : FunDecl) :
    Except String FunDecl := do
  pure ((← elimCoreOut sums Δ d).toFunDecl d.loopSpecs)

/-- Eliminate references from one function in isolation: without a
summary table, reference-passing calls are rejected.  Summarization runs
for its validation: returned references must derive from `&mut`
parameters (anything else escapes its frame). -/
def refElimFun (sigs : FunId → Option FunDecl) (Δ : StructDecls)
    (d : FunDecl) : Except String FunDecl := do
  let d ← elimImmRefs sigs d
  let _ ← summarize noSummaries d
  elimCore noSummaries Δ d

/-- The borrow-summary table of a *post-imm* function list, by Kleene
iteration from the empty table — recursion through reference-passing
calls must converge. -/
def computeSummaries (post : List FunDecl) :
    Except String (List FunSummary) := do
  let table₀ : List FunSummary := post.map fun d =>
    ⟨mutParamsOf d, List.replicate d.returns.length []⟩
  let rec iterate : Nat → List FunSummary →
      Except String (List FunSummary)
    | 0, _ => throw "recursive reference derivations do not converge"
    | fuel + 1, table => do
      let sums : Summaries := fun f => table[f]?
      let table' ← post.mapM (summarize sums)
      if table' == table then pure table
      else iterate fuel table'
  iterate (2 * post.length + 8) table₀

/-- Eliminate references from a whole program (functions listed in
`FunId` order): compute the borrow summaries, then eliminate every
function under the table. -/
def refElimProg (Δ : StructDecls) (fns : List FunDecl) :
    Except String (List FunDecl) := do
  let post ← fns.mapM (elimImmRefs fun f => fns[f]?)
  let table ← computeSummaries post
  let sums : Summaries := fun f => table[f]?
  post.mapM (elimCore sums Δ)

end MoveModel.IR
