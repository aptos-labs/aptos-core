-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.IR.Value
import Move.IR.State
import Move.IR.Syntax
import Move.IR.Semantics

/-!
# Reference Elimination (TACAS'22 §3.1)

The bytecode-to-bytecode transformation that rewrites the reference
operations into the value-level *mutation algebra* (`Value.mut`, the
`mkMutLoc` … `mutAddr` operations — Boogie's `$Mutation`), so that
borrow-based code becomes verifiable: verification
(`Prover.Translate.compileFun`) compiles reference operations to failing
assertions and expects reference-free input, exactly as the real prover
runs its reference-elimination passes before specification injection.

The pipeline mirrors the real prover's pass structure:

1. **Immutable-reference elimination** (`eliminate_imm_refs`): `&T`-typed
   locals become `T`-typed values — an immutable borrow is a copy, reads
   through immutable references are copies, `freeze_ref` is a read.  Only
   `&mut` remains.
2. **Liveness** (backward dataflow) — borrows are *released* where their
   reference (and every reference derived from it) dies, exactly MVP's
   livevar-driven `dying_nodes`.
3. **Borrow graph** (forward dataflow, join = union): nodes are local
   roots, global roots, and reference locals; edges record derivations
   (`direct` for copies, `field i`, `index`).  A reference reaching a
   merge point along different derivations has several in-edges — the
   candidates for its write-back.
4. **Rewriting**: borrows become mutation checkouts (`mkMutLoc x` reads
   the local, `mkMutGlobal r` reads the resource and aborts if absent,
   like the borrow), `borrow_field`/`borrow_vec_elem` become sub-mutation
   derivations, `read_ref`/`write_ref` become `getMut`/`setMut` on the
   carried value.  At each death point the mutation is **written back**
   along its in-edges: into the parent mutation (a functional
   `update_field`/`vec_set` of its payload, recovering dynamic indices
   with `mutPathIndex`), into the root local, or into global memory
   (`write_global` at `mutAddr`).  With several candidate parents, each
   write-back is guarded by the corresponding `isParent`/`isMutLoc`/
   `isMutGlobal` test — MVP's dynamic write-back dispatch — implemented by
   **block splitting**: new blocks are appended above `body.size`, so
   existing block ids, loop headers, and back-edge targets are unchanged;
   deaths on only some successor edges of a branch get their write-backs
   in **edge-split** blocks.  Loop `valTargets` are extended by every
   local the inserted code writes inside the loop.

The rejected fragment (the transformation is partial; violations report
errors): nested references; references at function boundaries (parameters,
returns, call arguments and results — the summary-based cross-call
elimination is a later stage); *exclusivity* violations that Move's borrow
checker rules out — using or writing a local root while it is borrowed,
re-borrowing into a local whose derived references are still live, and
touching a resource type (`get_global`/`move_from`/`write_global`/
`borrow_global`) or calling a function while a global borrow on it is
live (`move_to` and `exists` are allowed: they never observe the
checked-out payload).

**Deferred write-back and abort memory.**  Writes through references
update the mutation's payload; global memory sees them only at the borrow
death (the read-update-write cycle that makes the encoding alias-free).
An execution aborting *while a global borrow is live* therefore carries an
abort memory without the pending mutation — unobservable in the prover's
model (`aborts_if` is evaluated in the pre-state, and the VM discards all
effects of an aborting execution), and reflected in the correctness
statement: `refElim_correct` relates outcomes by `AgreeOutcome` — normal
returns agree exactly, aborts agree on the code.
-/

namespace Move.IR

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

/-- Strip one reference layer (the imm pass turns `&T` locals into `T`). -/
def Ty.stripRef : Ty → Ty
  | .ref t => t
  | .mutRef t => t
  | t => t

/-- Is the type a reference type? -/
def Ty.isRef : Ty → Bool
  | .ref _ | .mutRef _ => true
  | _ => false

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

/-- The payload type of a `&mut`-declared local. -/
def mutPayloadTy (d : FunDecl) (x : LocalIndex) : Except String Ty := do
  let some (.mutRef t) := d.locals x
    | throw s!"local {x} is not a mutable reference"
  pure t

/-! ## Dead-store elimination -/

/-- The locals a terminator reads. -/
def termReads : Term → List LocalIndex
  | .ret srcs => srcs
  | .abort c => [c]
  | .branch c _ _ => [c]
  | .jump _ => []

/-- Operations that cannot abort and do not touch memory (they may only be
*stuck*, which the forward simulation tolerates) — candidates for
dead-store removal. -/
def sideEffectFree : Oper → Bool
  | .lt | .le | .eq | .and | .or | .not | .pack | .unpack
  | .getField _ | .updateField _ | .vecPack | .vecLen
  | .mkMutLoc _ | .childMutField _ | .getMut | .setMut
  | .isParent _ | .mutPathIndex _ | .isMutLoc _ | .isMutGlobal _ | .mutAddr
  | .exists_ _ => true
  | .add | .sub | .mul | .div | .mod
  | .vecGet | .vecSet | .vecPush | .vecPop
  | .mkMutGlobal _ | .childMutIndex
  | .getGlobal _ | .writeGlobal _ | .moveTo _ | .moveFrom _ | .function _
  | .borrowLoc | .borrowField _ | .borrowGlobal _ | .borrowVecElem
  | .readRef | .writeRef | .freezeRef => false

/-- Dead-store elimination over the *fresh* locals (indices `≥ base`)
of one block: drops side-effect-free instructions whose destinations are
fresh and never read later in the block.  Keeps the eliminated code — and
the verification conditions computed from it — small. -/
def deadElim (base : Nat) (term : Term) (instrs : List Instr) : List Instr :=
  let termReads : List LocalIndex := termReads term
  let rec go : List Instr → List LocalIndex × List Instr
    | [] => (termReads, [])
    | i :: rest =>
      let (reads, rest') := go rest
      let keep : Bool :=
        match i with
        | .load dst _ => dst < base || reads.contains dst
        | .assign dst _ => dst < base || reads.contains dst
        | .call dsts op _ =>
            !sideEffectFree op ||
              dsts.any fun dst => dst < base || reads.contains dst
        | .nop => true
      if keep then
        let uses : List LocalIndex :=
          match i with
          | .load _ _ => []
          | .assign _ src => [src]
          | .call _ _ srcs => srcs
          | .nop => []
        let dsts : List LocalIndex :=
          match i with
          | .load dst _ => [dst]
          | .assign dst _ => [dst]
          | .call dsts _ _ => dsts
          | .nop => []
        (uses ++ reads.filter (fun x => !dsts.contains x), i :: rest')
      else (reads, rest')
  (go instrs).2

/-! ## The immutable-reference pre-pass (`eliminate_imm_refs`) -/

/-- Rewrite one instruction of the imm pass.  `origTy` classifies locals
by their *original* declared types. -/
private def elimImmInstr (d : FunDecl) (st : ElimSt) :
    Instr → Except String (ElimSt × List Instr)
  | .call [dst] .borrowLoc [x] => do
      match ← localTy d dst with
      | .ref _ => do
          if (← localTy d x).isRef then
            throw "nested references are not supported"
          pure (st, [.assign dst x])
      | _ => pure (st, [.call [dst] .borrowLoc [x]])
  | .call [dst] (.borrowField i) [t] => do
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
  | .call [dst] .freezeRef [t] => do
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
  | i => pure (st, [i])

/-- Eliminate immutable references: `&T` locals become `T` values.  Only
`&mut` references remain for the mutation pass. -/
def elimImmRefs (d : FunDecl) : Except String FunDecl := do
  let (blocks, st) ← (List.range d.body.size).foldlM
    (init := (([] : List (Option Block)), ⟨d.numLocals, []⟩))
    fun (acc, st) b =>
      match d.body.blocks b with
      | none => pure (acc ++ [none], st)
      | some blk => do
          let (st, instrs) ← blk.instrs.foldlM
            (init := (st, ([] : List Instr)))
            fun (st, is) i => do
              let (st, is') ← elimImmInstr d st i
              pure (st, is ++ is')
          pure (acc ++ [some ⟨instrs, blk.term⟩], st)
  pure { d with
    numLocals := d.numLocals + st.newTys.length
    locals := fun t =>
      if t < d.numLocals then (d.locals t).map fun ty =>
        match ty with
        | .ref u => u
        | ty => ty
      else st.newTys[t - d.numLocals]?
    body := { d.body with blocks := fun b => (blocks[b]?).join } }

/-! ## Liveness (backward dataflow) -/

private def instrUses : Instr → List LocalIndex
  | .load _ _ => []
  | .assign _ src => [src]
  | .call _ _ srcs => srcs
  | .nop => []

private def instrDefs : Instr → List LocalIndex
  | .load dst _ => [dst]
  | .assign dst _ => [dst]
  | .call dsts _ _ => dsts
  | .nop => []

private def termSuccs : Term → List BlockId
  | .jump b => [b]
  | .branch _ b₁ b₂ => [b₁, b₂]
  | .ret _ => []
  | .abort _ => []

private def sinsert (x : Nat) (s : List Nat) : List Nat :=
  if s.contains x then s else x :: s

private def sunion (a b : List Nat) : List Nat :=
  a.foldl (fun acc x => sinsert x acc) b

private def sremove (xs : List Nat) (s : List Nat) : List Nat :=
  s.filter (fun x => !xs.contains x)

private def seq (a b : List Nat) : Bool :=
  a.all b.contains && b.all a.contains

/-- The live set before an instruction, from the live set after it. -/
private def liveThroughInstr (i : Instr) (after : List Nat) : List Nat :=
  sunion (instrUses i) (sremove (instrDefs i) after)

/-- The live set at the start of a block, from the block-exit live set. -/
private def liveThroughBlock (blk : Block) (out : List Nat) : List Nat :=
  blk.instrs.foldr liveThroughInstr (sunion (termReads blk.term) out)

/-- Live-in sets per block (may-liveness, union join), by fixpoint. -/
private def liveAnalysis (d : FunDecl) : Array (List Nat) := Id.run do
  let n := d.body.size
  let mut liveIn : Array (List Nat) := Array.replicate n []
  let rounds := n * (d.numLocals + 2) + 2
  for _ in [0:rounds] do
    let mut changed := false
    for b in [0:n] do
      match d.body.blocks b with
      | none => pure ()
      | some blk =>
        let out := (termSuccs blk.term).foldl
          (fun acc s => sunion (liveIn.getD s []) acc) []
        let inn := liveThroughBlock blk out
        unless seq inn (liveIn.getD b []) do
          liveIn := liveIn.set! b inn
          changed := true
    unless changed do
      return liveIn
  return liveIn

/-! ## The borrow graph (forward dataflow, union join) -/

/-- A borrow-graph node: the root locations and the reference locals. -/
inductive BNode where
  | localRoot (x : LocalIndex)
  | globalRoot (r : ResourceId)
  | refNode (t : LocalIndex)
  deriving BEq, Repr

/-- One derivation step of a borrow edge: a field offset or a dynamic
vector index. -/
inductive BStep where
  | field (i : Nat)
  | index
  deriving BEq, Repr

/-- A borrow edge: `child` (always a reference local) was derived from
`parent` along the step path — `[]` is a direct copy/alias, one step a
field or element borrow, several a derivation summarized through a call
(MVP's hyper edges). -/
structure BEdge where
  parent : BNode
  child : LocalIndex
  path : List BStep
  deriving BEq, Repr

/-- The borrow graph: a set of edges, accumulated over all paths (the
union join of MVP's dataflow); several in-edges of one reference are the
write-back candidates resolved dynamically by `isParent`. -/
abbrev BGraph := List BEdge

private def gInsert (e : BEdge) (g : BGraph) : BGraph :=
  if g.contains e then g else e :: g

private def gUnion (a b : BGraph) : BGraph :=
  a.foldl (fun acc e => gInsert e acc) b

private def gEq (a b : BGraph) : Bool :=
  a.all b.contains && b.all a.contains

/-- The in-edges (write-back candidates) of a reference local. -/
def inEdges (g : BGraph) (t : LocalIndex) : List BEdge :=
  g.filter (·.child == t)

/-! ## Function summaries (MVP's inter-procedural borrow analysis) -/

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
private def graphStep (sums : Summaries) (d : FunDecl) (g : BGraph) :
    Instr → BGraph
  | .call [dst] .borrowLoc [x] => gInsert ⟨.localRoot x, dst, []⟩ g
  | .call [dst] (.borrowGlobal r) _ => gInsert ⟨.globalRoot r, dst, []⟩ g
  | .call [dst] (.borrowField i) [t] =>
      gInsert ⟨.refNode t, dst, [.field i]⟩ g
  | .call [dst] .borrowVecElem [t, _] => gInsert ⟨.refNode t, dst, [.index]⟩ g
  | .assign dst src =>
      if isMutLocal d src && isMutLocal d dst then
        gInsert ⟨.refNode src, dst, []⟩ g
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
                g)
            g
  | _ => g

private def graphThroughBlock (sums : Summaries) (d : FunDecl) (blk : Block)
    (g : BGraph) : BGraph :=
  blk.instrs.foldl (graphStep sums d) g

/-- Entry borrow graphs per block, by forward fixpoint with union join. -/
private def borrowAnalysis (sums : Summaries) (d : FunDecl) :
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

/-- Summarize one (post-imm) declaration under the current table: the
derivations of every returned reference, by walking ancestor chains from
the ret-point graphs up to the parameter nodes.  Chains reaching a root
or an underived non-parameter reference are verifier-illegal escapes. -/
def summarize (sums : Summaries) (d : FunDecl) :
    Except String FunSummary := do
  let graphs := borrowAnalysis sums d
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

/-! ## The rewriting emitter -/

private structure EmitSt where
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

private abbrev EM := StateT EmitSt (Except String)

private def alloc (ty : Ty) : EM LocalIndex := do
  let s ← get
  let (e, x) := s.elim.alloc ty
  set { s with elim := e }
  pure x

private def emit (i : Instr) : EM Unit :=
  modify fun s => { s with cur := s.cur ++ [i] }

private def markWritten (t : LocalIndex) : EM Unit :=
  modify fun s =>
    if s.written.contains t then s else { s with written := t :: s.written }

private def clearWritten (t : LocalIndex) : EM Unit :=
  modify fun s => { s with written := s.written.filter (· ≠ t) }

private def emitAll (is : List Instr) : EM Unit :=
  modify fun s => { s with cur := s.cur ++ is }

private def newBlockId (src : BlockId) : EM BlockId := do
  let s ← get
  set { s with
    nextId := s.nextId + 1
    blockSrc := (s.nextId, src) :: s.blockSrc }
  pure s.nextId

/-- Close the current block with `term`; continue accumulating into
`contId`. -/
private def closeBlock (term : Term) (contId : BlockId) : EM Unit :=
  modify fun s =>
    { s with
      done := (s.curId, ⟨s.cur, term⟩) :: s.done
      curId := contId
      cur := [] }

/-- Emit `body` under the guard local `g` (a diamond: branch to a fresh
body block or directly to the continuation). -/
private def emitGuarded (src : BlockId) (g : LocalIndex)
    (body : List Instr) : EM Unit := do
  let doId ← newBlockId src
  let contId ← newBlockId src
  closeBlock (.branch g doId contId) contId
  modify fun s =>
    { s with done := (doId, ⟨body, .jump contId⟩) :: s.done }

/-! ## Write-back generation -/

/-- Lift an `Except` computation into the emitter. -/
private def EM.lift {α : Type} (x : Except String α) : EM α :=
  fun s => x.map (·, s)

/-- The functional update of `cur : curTy` (the parent payload during a
write-back of `t` into `p`) along the remaining derivation path: descend
by field selection resp. element read (indices recovered from the child's
dynamic path with `mutPathIndex` at the absolute step position `k`),
replace the leaf with `t`'s payload, and rebuild with
`update_field`/`vec_set` on the way up.  Returns the local holding the
updated value. -/
private def buildUpdate (Δ : StructDecls) (p t : LocalIndex) :
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
private def wbBody (Δ : StructDecls) (d : FunDecl) (t : LocalIndex)
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

/-- The dispatch guard testing whether `t`'s dynamic location was derived
along `e` (`is_parent` with the edge path as pattern — dynamic indices as
wildcards; `is_mut_loc`/`is_mut_global` for roots). -/
private def wbGuard (t : LocalIndex) (e : BEdge) :
    EM (List Instr × LocalIndex) := do
  let g ← alloc .bool
  let pat : List (Option Nat) := e.path.map fun s =>
    match s with
    | .field i => some i
    | .index => none
  let test : Instr :=
    match e.parent with
    | .refNode p => .call [g] (.isParent pat) [p, t]
    | .localRoot x => .call [g] (.isMutLoc x) [t]
    | .globalRoot r => .call [g] (.isMutGlobal r) [t]
  pure ([test], g)

/-- Emit the write-backs of the dying reference `t`: unguarded along a
unique in-edge, otherwise one guarded step per candidate. -/
private def emitWriteBacks (Δ : StructDecls) (d : FunDecl) (src : BlockId)
    (g : BGraph) (t : LocalIndex) : EM Unit := do
  match inEdges g t with
  | [] => pure ()
  | [e] => do
      emitAll (← wbBody Δ d t e)
      if let .refNode p := e.parent then markWritten p
  | es =>
      for e in es do
        let (gis, gl) ← wbGuard t e
        emitAll gis
        emitGuarded src gl (← wbBody Δ d t e)
        if let .refNode p := e.parent then markWritten p

/-- Process the deaths among `pending` at a point where `liveNow` is the
live set: write back every pending reference that is dead and has no
pending derived reference, children first (the cascade of MVP's
`dying_nodes` ancestor chains). -/
private def processDeaths (Δ : StructDecls) (d : FunDecl) (src : BlockId)
    (g : BGraph) (liveNow : List Nat) :
    List LocalIndex → EM (List LocalIndex)
  | pending => go pending.length pending
where
  hasPendingChild (g : BGraph) (pending : List LocalIndex)
      (t : LocalIndex) : Bool :=
    pending.any fun c => c ≠ t && (inEdges g c).any (·.parent == .refNode t)
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
private def rootBorrowed (g : BGraph) (pending : List LocalIndex)
    (x : LocalIndex) : Bool :=
  pending.any fun c => (inEdges g c).any (·.parent == .localRoot x)

/-- Is the resource type `r` borrowed? -/
private def globalBorrowed (g : BGraph) (pending : List LocalIndex)
    (r : ResourceId) : Bool :=
  pending.any fun c => (inEdges g c).any (·.parent == .globalRoot r)

/-- Is any resource type borrowed? -/
private def anyGlobalBorrowed (g : BGraph) (pending : List LocalIndex) :
    Bool :=
  pending.any fun c => (inEdges g c).any fun e =>
    match e.parent with
    | .globalRoot _ => true
    | _ => false

/-- Reject direct uses of borrowed local roots (Move's exclusivity: while
`x` is mutably borrowed, `x` itself is untouchable). -/
private def checkRoots (g : BGraph) (pending : List LocalIndex)
    (xs : List LocalIndex) : EM Unit := do
  for x in xs do
    if rootBorrowed g pending x then
      throw s!"local {x} is used while mutably borrowed"

/-- The exclusivity checks of a value operation touching global memory. -/
private def checkGlobalOp (g : BGraph) (pending : List LocalIndex) :
    Oper → EM Unit
  | .getGlobal r | .moveFrom r | .writeGlobal r | .mkMutGlobal r => do
      if globalBorrowed g pending r then
        throw s!"resource {r} is accessed while mutably borrowed"
  | _ => pure ()

/-- Rewrite one instruction; returns the updated graph and pending set. -/
private def rewriteInstr (sums : Summaries) (d : FunDecl) (g : BGraph)
    (pending : List LocalIndex) :
    Instr → EM (BGraph × List LocalIndex)
  | .call [dst] .borrowLoc [x] => do
      if isMutLocal d x then throw "nested references are not supported"
      if !(isMutLocal d dst) then throw "borrow into a non-reference local"
      if rootBorrowed g pending x then
        throw s!"local {x} is borrowed while already mutably borrowed"
      if pending.contains dst then
        throw "re-borrow while derived references are live"
      emit (.call [dst] (.mkMutLoc x) [x])
      clearWritten dst
      pure (gInsert ⟨.localRoot x, dst, []⟩ g, dst :: pending)
  | .call [dst] (.borrowGlobal r) [aT] => do
      if !(isMutLocal d dst) then throw "borrow into a non-reference local"
      if globalBorrowed g pending r then
        throw s!"resource {r} is borrowed while already mutably borrowed"
      if pending.contains dst then
        throw "re-borrow while derived references are live"
      checkRoots g pending [aT]
      emit (.call [dst] (.mkMutGlobal r) [aT])
      clearWritten dst
      pure (gInsert ⟨.globalRoot r, dst, []⟩ g, dst :: pending)
  | .call [dst] (.borrowField i) [t] => do
      if !(isMutLocal d t && isMutLocal d dst) then
        throw "field borrow outside the reference discipline"
      if pending.contains dst then
        throw "re-borrow while derived references are live"
      emit (.call [dst] (.childMutField i) [t])
      clearWritten dst
      pure (gInsert ⟨.refNode t, dst, [.field i]⟩ g, dst :: pending)
  | .call [dst] .borrowVecElem [t, iT] => do
      if !(isMutLocal d t && isMutLocal d dst) then
        throw "vector element borrow outside the reference discipline"
      if pending.contains dst then
        throw "re-borrow while derived references are live"
      checkRoots g pending [iT]
      emit (.call [dst] .childMutIndex [t, iT])
      clearWritten dst
      pure (gInsert ⟨.refNode t, dst, [.index]⟩ g, dst :: pending)
  | .call [dst] .readRef [t] => do
      if !(isMutLocal d t) then throw "read through a non-reference"
      if isMutLocal d dst then throw "nested references are not supported"
      emit (.call [dst] .getMut [t])
      pure (g, pending)
  | .call [] .writeRef [t, vt] => do
      if !(isMutLocal d t) then throw "write through a non-reference"
      if isMutLocal d vt then
        throw "reference stored through a reference"
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
        if pending.contains dst then
          throw "re-borrow while derived references are live"
        emit (.assign dst src')
        clearWritten dst
        pure (gInsert ⟨.refNode src', dst, []⟩ g, dst :: pending)
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
          pure src
        checkRoots g pending
          ((srcs.filter (!isMutLocal d ·)) ++ dsts.filter (!isMutLocal d ·))
        let mut g' := g
        let mut pending' := pending
        for (dst, derivs) in dsts.zip sum.retDerivs do
          if isMutLocal d dst then do
            if pending.contains dst then
              throw "re-borrow while derived references are live"
            if derivs.isEmpty then
              throw "a returned reference has no summarized derivation"
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
      checkGlobalOp g pending op
      checkRoots g pending ((srcs.filter (!isMutLocal d ·)) ++ dsts)
      emit (.call dsts op srcs)
      pure (g, pending)

/-- Rewrite one source block: walk the instructions (with per-point death
processing), then handle terminator-edge deaths — uniformly before the
terminator where possible, in edge-split blocks otherwise. -/
private def rewriteBlock (sums : Summaries) (Δ : StructDecls) (d : FunDecl)
    (liveIn : Array (List Nat)) (b : BlockId) (blk : Block) (g₀ : BGraph) :
    EM Unit := do
  modify fun s => { s with curId := b, cur := [] }
  -- pending at entry: live references that were possibly checked out —
  -- either borrowed (in-edges) or `&mut` parameters (checked out by the
  -- caller; never written back here, they are returned as finals)
  let pending₀ := (List.range d.numLocals).filter fun t =>
    isMutLocal d t && (liveIn.getD b []).contains t &&
      (!(inEdges g₀ t).isEmpty || t < d.numParams)
  modify fun s => { s with written := pending₀ }
  -- live-after sets, per instruction
  let liveAtTerm := sunion (termReads blk.term)
    ((termSuccs blk.term).foldl (fun acc s => sunion (liveIn.getD s []) acc)
      [])
  let liveAfters := (blk.instrs.foldr
    (fun i (acc : List (List Nat)) =>
      (liveThroughInstr i (acc.headD liveAtTerm)) :: acc) []).drop 1
    ++ [liveAtTerm]
  let mut g := g₀
  let mut pending := pending₀
  for (i, liveAfter) in blk.instrs.zip liveAfters do
    let (g', pending') ← rewriteInstr sums d g pending i
    g := g'
    pending ← processDeaths Δ d b g' liveAfter pending'
  -- terminator
  match blk.term with
  | .ret srcs => do
      -- mutation-typed sources are *returned* borrows (they continue in
      -- the caller); `&mut` parameters are returned as finals (the ret
      -- extension of MVP's `call r := f(r)`) — neither is written back
      let keep := srcs.filter (isMutLocal d) ++ mutParamsOf d
      let pending' ← processDeaths Δ d b g keep pending
      let written := (← get).written
      if pending'.any (fun t => !keep.contains t && written.contains t) then
        throw "an intermediate borrow escapes through a returned reference"
      closeBlock (.ret (srcs ++ mutParamsOf d)) 0
  | .abort c => do
      -- aborts discard pending write-backs (see the module docs)
      closeBlock (.abort c) 0
  | .jump b' => do
      let pending' ← processDeaths Δ d b g (liveIn.getD b' []) pending
      let _ := pending'
      closeBlock (.jump b') 0
  | .branch c b₁ b₂ => do
      checkRoots g pending [c]
      -- deaths on both edges: write back before the branch
      let liveBoth := sunion (liveIn.getD b₁ []) (liveIn.getD b₂ [])
      let pendingB ← processDeaths Δ d b g liveBoth pending
      -- deaths on a single edge: write back in an edge-split block
      let splitEdge (target : BlockId) : EM BlockId := do
        if pendingB.all (fun t => (liveIn.getD target []).contains t) then
          pure target
        else do
          let w ← newBlockId b
          let saved ← get
          set { saved with curId := w, cur := [] }
          let pending' ← processDeaths Δ d b g (liveIn.getD target [])
            pendingB
          let _ := pending'
          closeBlock (.jump target) 0
          modify fun s => { s with curId := saved.curId, cur := saved.cur }
          pure w
      let t₁ ← splitEdge b₁
      let t₂ ← splitEdge b₂
      closeBlock (.branch c t₁ t₂) 0

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
  let st₀ : EmitSt :=
    { elim := ⟨d.numLocals, []⟩
      nextId := d.body.size
      done := []
      blockSrc := []
      curId := 0
      cur := []
      written := [] }
  let (_, st) ← ((List.range d.body.size).forM fun b =>
    match d.body.blocks b with
    | none => pure ()
    | some blk => rewriteBlock sums Δ d liveIn b blk (graphs.getD b [])) st₀
  let blocks := st.done.map fun (i, blk) =>
    (i, Block.mk (deadElim d.numLocals blk.term blk.instrs) blk.term)
  let dense ← (List.range st.nextId).mapM fun b =>
    match blocks.find? (·.1 == b) with
    | some (_, blk) => pure blk
    | none =>
        -- source gaps (undeclared block ids) are preserved as empty
        -- unreachable blocks
        if b < d.body.size && (d.body.blocks b).isNone then
          pure ⟨[], .jump b⟩
        else throw "internal: an output block id was not emitted"
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
def refElimFun (Δ : StructDecls) (d : FunDecl) : Except String FunDecl := do
  let d ← elimImmRefs d
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
  let post ← fns.mapM elimImmRefs
  let table ← computeSummaries post
  let sums : Summaries := fun f => table[f]?
  post.mapM (elimCore sums Δ)

/-! ## Correctness (stated) -/

/-- `P'` is the reference elimination of `P`: same struct declarations,
and every function eliminated. -/
def ElimProgram (P P' : Program) : Prop :=
  P'.structs = P.structs ∧
  ∀ f, (P.funs f = none ∧ P'.funs f = none) ∨
    ∃ d d', P.funs f = some d ∧ refElimFun P.structs d = .ok d' ∧
      P'.funs f = some d'

/-- Outcome agreement modulo abort memory: the eliminated program defers
write-backs to borrow death (the read-update-write cycle), so the memory
at an abort point may lack pending mutations.  This is unobservable in the
prover's model — `aborts_if` is evaluated in the pre-state, and the VM
discards all effects of an aborting execution. -/
inductive AgreeOutcome : FrameOutcome → FrameOutcome → Prop where
  | ret {m : Memory} {vals : List Value} :
      AgreeOutcome (.ret m vals) (.ret m vals)
  | abort {m m' : Memory} {code : Nat} :
      AgreeOutcome (.abort m code) (.abort m' code)

/-- **Reference elimination is a forward simulation** (stated; a next proof
target): every terminating execution of the original program from a
reference-free boundary is matched by an execution of the eliminated one —
exactly on normal returns, up to abort memory on aborts.  Together with
`prover_sound` for the eliminated program, this transports contracts back
to borrow-based code. -/
theorem refElim_correct (P P' : Program) (h : ElimProgram P P')
    (f : FunId) (m : Memory) (args : List Value)
    (hargs : ∀ v ∈ args, v.refFree)
    (o : FrameOutcome) (hexec : FunExec P f m args o) :
    ∃ o', FunExec P' f m args o' ∧ AgreeOutcome o o' := by
  sorry

end Move.IR
