-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Model.AbsInt
import Model.BorrowGraph
import Model.StackUsage

/-!
# Reference safety

UNTRUSTED. Transcribes `move-bytecode-verifier/src/reference_safety/`: each
local holds a reference or a value, an abstract stack holds the operands,
and a borrow graph (`Model/BorrowGraph.lean`) records which references borrow
from which, with the frame root standing for the locals and global storage.
The analysis runs on the abstract interpreter. At the end of each block the
stack must be empty, and the state is put in canonical form: the reference in
local `i` is renamed `i`, the frame root is the number of locals, and fresh
references are numbered after it.

Every instruction is transcribed, but the type-safety model admits only the
core subset so far, so only its paths are exercised. The verifier meter,
which bounds the work of this pass, is not modeled.
-/

namespace MoveBytecode.Model

open BorrowGraph

/-- `AbstractValue`. -/
inductive AbsValue where
  | reference (id : Nat)
  | nonReference
  deriving DecidableEq, Repr, Inhabited

def AbsValue.refId? : AbsValue → Option Nat
  | .reference id => some id
  | .nonReference => none

/-- `AbstractState`, with the analysis's operand stack (top first), which is
empty at the start and end of every block. -/
structure RefState where
  locals : List AbsValue
  graph : Graph
  nextId : Nat
  stack : List AbsValue
  deriving Repr, Inhabited

/-- Whether a type is a reference, and if so whether it is mutable. -/
def refKind : SignatureToken → Option Bool
  | .reference _ => some false
  | .mutableReference _ => some true
  | .bool | .u8 | .u16 | .u32 | .u64 | .u128 | .u256 | .i8 | .i16 | .i32 | .i64 | .i128
  | .i256 | .address | .signer | .vector _ | .function _ _ _ | .struct _
  | .structInstantiation _ _ | .typeParameter _ => none

/-- `AbstractState::new`. -/
def RefState.new (view : FunctionView) : VResult RefState := do
  let numLocals := view.parameters.length + view.locals.length
  let init : RefState :=
    { locals := List.replicate numLocals .nonReference, graph := [], nextId := numLocals + 1,
      stack := [] }
  let st ← view.parameters.zipIdx.foldlM (fun st (ty, i) =>
    match refKind ty with
    | some mut_ => do
      pure { st with graph := ← st.graph.newRef i mut_, locals := st.locals.set i (.reference i) }
    | none => pure st) init
  pure { st with graph := ← st.graph.newRef numLocals true }

/-- `is_canonical`. -/
def RefState.isCanonical (st : RefState) : Bool :=
  st.locals.length + 1 == st.nextId &&
    st.locals.zipIdx.all fun (v, i) => match v with
      | .reference id => id == i
      | .nonReference => true

/-- `join_` followed by `AbstractDomain::join`: a local holding a reference on
one side only is released; the graphs are joined; the state changed unless
the locals agree and the old graph covers the joined one. -/
def RefState.join (a b : RefState) : VResult (RefState × Bool) := do
  if !(a.isCanonical && b.isCanonical) || a.nextId != b.nextId
      || a.locals.length != b.locals.length then
    panicAt "join: states not canonical"
  let (selfGraph, otherGraph, locals) ← (a.locals.zip b.locals).foldlM
    (fun (sg, og, ls) (v1, v2) => match v1, v2 with
      | .reference id, .nonReference => do pure (← sg.release id, og, ls ++ [.nonReference])
      | .nonReference, .reference id => do pure (sg, ← og.release id, ls ++ [.nonReference])
      | v1, v2 =>
        if v1 == v2 then pure (sg, og, ls ++ [v1]) else panicAt "join: different references")
    (a.graph, b.graph, [])
  let joined := { a with locals, graph := ← selfGraph.join otherGraph }
  -- `&&` short-circuits in Rust: `leq` runs only when the locals agree.
  let unchanged ← if a.locals == joined.locals then a.graph.leq joined.graph else pure false
  if unchanged then pure (a, false) else pure (joined, true)

/-- The analysis's state, with the verifier's rejections and panics. -/
abbrev RefM := StateT RefState VResult

namespace ReferenceSafety

def frameRoot : RefM Nat := do pure (← get).locals.length

def graphOp (f : Graph → VResult Graph) : RefM Unit := do
  let st ← get
  set { st with graph := ← f st.graph }

def query {α : Type} (f : Graph → VResult α) : RefM α := do f (← get).graph

def push (v : AbsValue) : RefM Unit := modify fun st => { st with stack := v :: st.stack }

def pop : RefM AbsValue := do
  let st ← get
  match st.stack with
  | v :: rest => set { st with stack := rest }; pure v
  | [] => panicAt "reference safety: stack underflow"

/-- `safe_assert!(pop().is_value())`. -/
def popValue : RefM Unit := do
  match ← pop with
  | .nonReference => pure ()
  | .reference _ => panicAt "reference safety: expected a value"

/-- `safe_unwrap!(pop().ref_id())`. -/
def popRef : RefM Nat := do
  match ← pop with
  | .reference id => pure id
  | .nonReference => panicAt "reference safety: expected a reference"

/-- Pop `n` operands, returned top first: Rust's
`iter().map(|_| pop()).rev().collect()` pops as it collects. The order only
decides which error is reported. -/
def popN (n : Nat) : RefM (List AbsValue) := (List.range n).mapM fun _ => pop

/-- `new_ref`. -/
def newRef (mut_ : Bool) : RefM Nat := do
  let st ← get
  set { st with graph := ← st.graph.newRef st.nextId mut_, nextId := st.nextId + 1 }
  pure st.nextId

/-- `value_for`. -/
def valueFor (ty : SignatureToken) : RefM AbsValue := do
  match refKind ty with
  | some mut_ => pure (.reference (← newRef mut_))
  | none => pure .nonReference

def release (id : Nat) : RefM Unit := graphOp (·.release id)

/-- `release_value`. -/
def releaseValue : AbsValue → RefM Unit
  | .reference id => release id
  | .nonReference => pure ()

def addCopy (parent child : Nat) : RefM Unit := graphOp (·.factor parent [] child)

def addBorrow (parent child : Nat) : RefM Unit := graphOp (·.addEdge parent ⟨false, []⟩ child)

def addFieldBorrow (parent field child : Nat) : RefM Unit :=
  graphOp (·.factor parent [.field field] child)

def addLocalBorrow (local_ id : Nat) : RefM Unit := do
  let root ← frameRoot
  graphOp (·.factor root [.local_ local_] id)

def addResourceBorrow (resource id : Nat) : RefM Unit := do
  let root ← frameRoot
  graphOp (·.addEdge root ⟨false, [.global resource]⟩ id)

def isMutable (id : Nat) : RefM Bool := query (·.isMutable id)

def isWritable (id : Nat) : RefM Bool := query (·.isWritable id)

def isReadable (id : Nat) (field? : Option Nat) : RefM Bool :=
  query (·.isReadable id (field?.map .field))

def isFreezable (id : Nat) : RefM Bool := query (·.isFreezable id none)

def isLocalBorrowed (i : Nat) : RefM Bool := do
  let root ← frameRoot
  query (·.hasConsistentBorrows root (some (.local_ i)))

def isLocalMutablyBorrowed (i : Nat) : RefM Bool := do
  let root ← frameRoot
  query (·.hasConsistentMutableBorrows root (some (.local_ i)))

def isGlobalBorrowed (s : Nat) : RefM Bool := do
  let root ← frameRoot
  query (·.hasConsistentBorrows root (some (.global s)))

def isGlobalMutablyBorrowed (s : Nat) : RefM Bool := do
  let root ← frameRoot
  query (·.hasConsistentMutableBorrows root (some (.global s)))

def localAt (i : Nat) : RefM AbsValue := do
  match (← get).locals[i]? with
  | some v => pure v
  | none => panicAt "reference safety: local out of range"

def setLocal (i : Nat) (v : AbsValue) : RefM Unit :=
  modify fun st => { st with locals := st.locals.set i v }

/-- `copy_loc`. -/
def copyLoc (i : Nat) : RefM AbsValue := do
  match ← localAt i with
  | .reference id => do
    let newId ← newRef (← isMutable id)
    addCopy id newId
    pure (.reference newId)
  | .nonReference =>
    if ← isLocalMutablyBorrowed i then reject "COPYLOC_EXISTS_BORROW_ERROR"
    else pure .nonReference

/-- `move_loc`. -/
def moveLoc (i : Nat) : RefM AbsValue := do
  let old ← localAt i
  setLocal i .nonReference
  match old with
  | .reference id => pure (.reference id)
  | .nonReference =>
    if ← isLocalBorrowed i then reject "MOVELOC_EXISTS_BORROW_ERROR"
    else pure .nonReference

/-- `st_loc`. -/
def stLoc (i : Nat) (v : AbsValue) : RefM Unit := do
  let old ← localAt i
  setLocal i v
  match old with
  | .reference id => release id
  | .nonReference =>
    if ← isLocalBorrowed i then reject "STLOC_UNSAFE_TO_DESTROY_ERROR" else pure ()

/-- `freeze_ref`. -/
def freezeRef (id : Nat) : RefM AbsValue := do
  if !(← isFreezable id) then reject "FREEZEREF_EXISTS_MUTABLE_BORROW_ERROR"
  let frozen ← newRef false
  addCopy id frozen
  release id
  pure (.reference frozen)

/-- `comparison`. -/
def comparison (v1 v2 : AbsValue) : RefM AbsValue := do
  match v1, v2 with
  | .reference id1, .reference id2 =>
    if !(← isReadable id1 none) || !(← isReadable id2 none) then
      reject "READREF_EXISTS_MUTABLE_BORROW_ERROR"
    release id1
    release id2
  | .nonReference, .nonReference => pure ()
  | .reference _, .nonReference | .nonReference, .reference _ =>
    panicAt "comparison: a reference and a value"
  pure .nonReference

/-- `read_ref`. -/
def readRef (id : Nat) : RefM AbsValue := do
  if !(← isReadable id none) then reject "READREF_EXISTS_MUTABLE_BORROW_ERROR"
  release id
  pure .nonReference

/-- `write_ref`. -/
def writeRef (id : Nat) : RefM Unit := do
  if !(← isWritable id) then reject "WRITEREF_EXISTS_BORROW_ERROR"
  release id

/-- `borrow_loc`. -/
def borrowLoc (mut_ : Bool) (i : Nat) : RefM AbsValue := do
  if !mut_ && (← isLocalMutablyBorrowed i) then reject "BORROWLOC_EXISTS_BORROW_ERROR"
  let root ← frameRoot
  if mut_ && (← query (·.hasFullBorrows root)) then reject "BORROWLOC_EXISTS_BORROW_ERROR"
  let newId ← newRef mut_
  addLocalBorrow i newId
  pure (.reference newId)

/-- `borrow_field`. -/
def borrowField (mut_ : Bool) (id field : Nat) : RefM AbsValue := do
  let bad ← if mut_ then query (·.hasFullBorrows id)
    else do pure !(← isReadable id (some field))
  if bad then reject "BORROWFIELD_EXISTS_MUTABLE_BORROW_ERROR"
  let newId ← newRef mut_
  addFieldBorrow id field newId
  release id
  pure (.reference newId)

/-- `borrow_global`. -/
def borrowGlobal (mut_ : Bool) (s : Nat) : RefM AbsValue := do
  if (mut_ && (← isGlobalBorrowed s)) || (← isGlobalMutablyBorrowed s) then
    reject "GLOBAL_REFERENCE_ERROR"
  let newId ← newRef mut_
  addResourceBorrow s newId
  pure (.reference newId)

/-- `move_from`. -/
def moveFrom (s : Nat) : RefM AbsValue := do
  if ← isGlobalBorrowed s then reject "GLOBAL_REFERENCE_ERROR" else pure .nonReference

def refIdOf (v : AbsValue) : RefM Nat :=
  match v with
  | .reference id => pure id
  | .nonReference => panicAt "reference safety: expected a reference"

/-- `vector_op`. -/
def vectorOp (vector : AbsValue) (mut_ : Bool) : RefM Unit := do
  let id ← refIdOf vector
  -- `is_writable` asserts a mutable reference; Rust's `&&` short-circuits.
  if mut_ then
    if !(← isWritable id) then reject "VEC_UPDATE_EXISTS_MUTABLE_BORROW_ERROR"
  release id

/-- `vector_element_borrow`. -/
def vectorElementBorrow (vector : AbsValue) (mut_ : Bool) : RefM AbsValue := do
  let vecId ← refIdOf vector
  if !mut_ && !(← isReadable vecId none) then
    reject "VEC_BORROW_ELEMENT_EXISTS_MUTABLE_BORROW_ERROR"
  if mut_ then
    if !(← isWritable vecId) then reject "VEC_BORROW_ELEMENT_EXISTS_MUTABLE_BORROW_ERROR"
  let elemId ← newRef mut_
  addBorrow vecId elemId
  release vecId
  pure (.reference elemId)

/-- `core_call`: a returned mutable reference borrows from every mutable
argument, an immutable one from every reference argument; the arguments are
then released. -/
def coreCall (args : List AbsValue) (resultTys : List SignatureToken) : RefM (List AbsValue) := do
  let (all, muts) ← args.foldlM (fun (all, muts) v => do
    match v with
    | .nonReference => pure (all, muts)
    | .reference id =>
      if ← isMutable id then
        if !(← isWritable id) then reject "CALL_BORROWED_MUTABLE_REFERENCE_ERROR"
        pure (insertNat id all, insertNat id muts)
      else pure (insertNat id all, muts)) ([], [])
  let values ← resultTys.mapM fun ty => do
    match refKind ty with
    | some true =>
      let id ← newRef true
      muts.forM fun p => addBorrow p id
      pure (.reference id)
    | some false =>
      let id ← newRef false
      all.forM fun p => addBorrow p id
      pure (.reference id)
    | none => pure .nonReference
  all.forM release
  pure values

/-- `ret`: release the locals' references; nothing may still borrow from the
frame, and no returned mutable reference may be borrowed. -/
def ret (values : List AbsValue) : RefM Unit := do
  let released := (← get).locals.foldl (fun s v => match v with
    | .reference id => insertNat id s
    | .nonReference => s) []
  released.forM release
  let root ← frameRoot
  if ← query (·.hasConsistentBorrows root none) then
    reject "UNSAFE_RET_LOCAL_OR_RESOURCE_STILL_BORROWED"
  values.forM fun v => do
    match v with
    | .reference id =>
      if ← isMutable id then
        if !(← isWritable id) then reject "RET_BORROWED_MUTABLE_REFERENCE_ERROR"
    | .nonReference => pure ()

/-- `construct_canonical_state`, after `safe_assert!(stack.is_empty())`. -/
def canonicalize : RefM Unit := do
  let st ← get
  if !st.stack.isEmpty then panicAt "reference safety: stack not empty at block end"
  let root := st.locals.length
  let idMap := st.locals.zipIdx.foldl (fun m (v, i) => match v with
    | .reference old => NatMap.insert m old i
    | .nonReference => m) ([(root, root)] : NatMap Nat)
  let locals := st.locals.zipIdx.map fun (v, i) => match v with
    | .reference _ => AbsValue.reference i
    | .nonReference => .nonReference
  set { st with locals, graph := st.graph.remap idMap, nextId := root + 1 }

variable (m : CompiledModule) (view : FunctionView)

/-- The definition a function handle's name names (`name_def_map`, the last
definition winning). -/
def nameDef (name : Nat) : VResult (Option Nat) := do
  let named ← m.functionDefs.toList.zipIdx.filterM fun (fd, _) => do
    pure ((← lookup m.functionHandles fd.function "function_handle_at").name == name)
  pure (named.getLast?.map (·.2))

/-- The structs a call acquires: the annotations of the definition of this
module whose handle is exactly the callee's. -/
def acquiredBy (fh : FunctionHandle) : VResult (List Nat) := do
  match ← nameDef m fh.name with
  | some idx =>
    let fd ← lookup m.functionDefs idx "function_def_at"
    let h ← lookup m.functionHandles fd.function "function_handle_at"
    pure (if fh == h then fd.acquiresGlobalResources.foldl (fun s x => insertNat x s) [] else [])
  | none => pure []

/-- `call`. -/
def call (fh : FunctionHandle) : RefM Unit := do
  let params ← lookup m.signatures fh.parameters "signature_at"
  let args ← popN params.length
  let acquired ← acquiredBy m fh
  let returns ← lookup m.signatures fh.returns "signature_at"
  acquired.forM fun s => do
    if ← isGlobalBorrowed s then reject "GLOBAL_REFERENCE_ERROR"
  let values ← coreCall args returns
  values.forM push

/-- The field a `BorrowFieldMutable` attribute names, the first winning. -/
def borrowFieldMutable (attrs : List FunctionAttribute) : Option Nat :=
  attrs.findSome? fun a => match a with
    | .borrowFieldMutable off => some off
    | .persistent | .moduleLock | .pack | .packVariant _ | .unpack | .unpackVariant _
    | .testVariant _ | .borrowFieldImmutable _ => none

/-- `call_or_specialize_borrow_field_mut`: a call to a struct-API borrow is a
mutable field borrow. -/
def callOrSpecialize (fh : FunctionHandle) : RefM Unit := do
  match borrowFieldMutable fh.attributes with
  | some off => do
    let id ← popRef
    push (← borrowField true id off)
  | none => call m fh

/-- `clos_pack`: the captured arguments are values. -/
def closPack (fh : FunctionHandle) (mask : BitVec 64) : RefM Unit := do
  let params ← lookup m.signatures fh.parameters "signature_at"
  let argc := (List.range params.length).countP (fun i => mask.getLsbD i)
  (List.range argc).forM fun _ => popValue
  push .nonReference

/-- `vec_element_type`. -/
def vecElementType (idx : Nat) : RefM SignatureToken := do
  match (← lookup m.signatures idx "signature_at").head? with
  | some ty => pure ty
  | none => reject "VERIFIER_INVARIANT_VIOLATION"

/-- `fun_type`. -/
def funType (idx : Nat) : RefM (List SignatureToken × List SignatureToken) := do
  match (← lookup m.signatures idx "signature_at").head? with
  | some ty =>
    match ty with
    | .function args results _ => pure (args, results)
    | .bool | .u8 | .u16 | .u32 | .u64 | .u128 | .u256 | .i8 | .i16 | .i32 | .i64 | .i128
    | .i256 | .address | .signer | .vector _ | .struct _ | .structInstantiation _ _
    | .reference _ | .mutableReference _ | .typeParameter _ => reject "VERIFIER_INVARIANT_VIOLATION"
  | none => reject "VERIFIER_INVARIANT_VIOLATION"

def structDef (idx : Nat) : RefM StructDefinition := lookup m.structDefs idx "struct_def_at"

/-- `pack` and `pack_variant`. -/
def packFields (n : Nat) : RefM Unit := do
  (List.range n).forM fun _ => popValue
  push .nonReference

/-- `unpack` and `unpack_variant`. -/
def unpackFields (n : Nat) : RefM Unit := do
  popValue
  (List.range n).forM fun _ => push .nonReference

def variantFields (h : Nat) : RefM Nat := do
  let handle ← lookup m.structVariantHandles h "struct_variant_handle_at"
  pure (fieldCount (← structDef m handle.structIndex).fieldInformation (some handle.variant))

def structFields (d : Nat) : RefM Nat := do
  pure (fieldCount (← structDef m d).fieldInformation none)

def instDef (i : Nat) : RefM Nat := do
  pure (← lookup m.structDefInstantiations i "struct_instantiation_at").def_

def variantInstHandle (i : Nat) : RefM Nat := do
  pure (← lookup m.structVariantInstantiations i "struct_variant_instantiation_at").handle

def fieldOf (h : Nat) : RefM Nat := do
  pure (← lookup m.fieldHandles h "field_handle_at").field

def variantFieldOf (h : Nat) : RefM Nat := do
  pure (← lookup m.variantFieldHandles h "variant_field_handle_at").field

/-- `execute_inner`. -/
def executeInner : Bytecode → RefM Unit
  | .pop => do releaseValue (← pop)
  | .copyLoc i => do push (← copyLoc i)
  | .moveLoc i => do push (← moveLoc i)
  | .stLoc i => do stLoc i (← pop)
  | .freezeRef => do push (← freezeRef (← popRef))
  | .eq | .neq => do
    let v1 ← pop
    let v2 ← pop
    push (← comparison v1 v2)
  | .readRef => do push (← readRef (← popRef))
  | .writeRef => do
    let id ← popRef
    popValue
    writeRef id
  | .mutBorrowLoc i => do push (← borrowLoc true i)
  | .immBorrowLoc i => do push (← borrowLoc false i)
  | .mutBorrowField h => do
    let id ← popRef
    push (← borrowField true id (← fieldOf m h))
  | .mutBorrowFieldGeneric i => do
    let fi ← lookup m.fieldInstantiations i "field_instantiation_at"
    let id ← popRef
    push (← borrowField true id (← fieldOf m fi.handle))
  | .immBorrowField h => do
    let id ← popRef
    push (← borrowField false id (← fieldOf m h))
  | .immBorrowFieldGeneric i => do
    let fi ← lookup m.fieldInstantiations i "field_instantiation_at"
    let id ← popRef
    push (← borrowField false id (← fieldOf m fi.handle))
  | .mutBorrowVariantField h => do
    let id ← popRef
    push (← borrowField true id (← variantFieldOf m h))
  | .mutBorrowVariantFieldGeneric i => do
    let fi ← lookup m.variantFieldInstantiations i "variant_field_instantiation_at"
    let id ← popRef
    push (← borrowField true id (← variantFieldOf m fi.handle))
  | .immBorrowVariantField h => do
    let id ← popRef
    push (← borrowField false id (← variantFieldOf m h))
  | .immBorrowVariantFieldGeneric i => do
    let fi ← lookup m.variantFieldInstantiations i "variant_field_instantiation_at"
    let id ← popRef
    push (← borrowField false id (← variantFieldOf m fi.handle))
  | .mutBorrowGlobal d => do
    popValue
    push (← borrowGlobal true d)
  | .mutBorrowGlobalGeneric i => do
    popValue
    push (← borrowGlobal true (← instDef m i))
  | .immBorrowGlobal d => do
    popValue
    push (← borrowGlobal false d)
  | .immBorrowGlobalGeneric i => do
    popValue
    push (← borrowGlobal false (← instDef m i))
  | .moveFrom d => do
    popValue
    push (← moveFrom d)
  | .moveFromGeneric i => do
    popValue
    push (← moveFrom (← instDef m i))
  | .call h => do callOrSpecialize m (← lookup m.functionHandles h "function_handle_at")
  | .callGeneric i => do
    let fi ← lookup m.functionInstantiations i "function_instantiation_at"
    callOrSpecialize m (← lookup m.functionHandles fi.handle "function_handle_at")
  | .ret => do
    let values ← popN view.returns.length
    ret values.reverse
  | .branch _ | .nop | .castU8 | .castU16 | .castU32 | .castU64 | .castU128 | .castU256
  | .castI8 | .castI16 | .castI32 | .castI64 | .castI128 | .castI256 | .negate | .not
  | .exists_ _ | .existsGeneric _ => pure ()
  | .brTrue _ | .brFalse _ | .abort => popValue
  | .abortMsg => do
    popValue
    popValue
  | .moveTo _ | .moveToGeneric _ => do
    popValue
    releaseValue (← pop)
  | .ldTrue | .ldFalse => do push (← valueFor .bool)
  | .ldU8 _ => do push (← valueFor .u8)
  | .ldU16 _ => do push (← valueFor .u16)
  | .ldU32 _ => do push (← valueFor .u32)
  | .ldU64 _ => do push (← valueFor .u64)
  | .ldU128 _ => do push (← valueFor .u128)
  | .ldU256 _ => do push (← valueFor .u256)
  | .ldI8 _ => do push (← valueFor .i8)
  | .ldI16 _ => do push (← valueFor .i16)
  | .ldI32 _ => do push (← valueFor .i32)
  | .ldI64 _ => do push (← valueFor .i64)
  | .ldI128 _ => do push (← valueFor .i128)
  | .ldI256 _ => do push (← valueFor .i256)
  | .ldConst i => do push (← valueFor (← lookup m.constantPool i "constant_at").type)
  | .add | .sub | .mul | .mod | .div | .bitOr | .bitAnd | .xor | .shl | .shr | .or | .and
  | .lt | .gt | .le | .ge => do
    popValue
    popValue
    push .nonReference
  | .pack d => do packFields (← structFields m d)
  | .packGeneric i => do packFields (← structFields m (← instDef m i))
  | .unpack d => do unpackFields (← structFields m d)
  | .unpackGeneric i => do unpackFields (← structFields m (← instDef m i))
  | .testVariant h => do
    let _ ← lookup m.structVariantHandles h "struct_variant_handle_at"
    push (← readRef (← popRef))
  | .testVariantGeneric i => do
    let h ← variantInstHandle m i
    let _ ← lookup m.structVariantHandles h "struct_variant_handle_at"
    push (← readRef (← popRef))
  | .packVariant h => do packFields (← variantFields m h)
  | .packVariantGeneric i => do packFields (← variantFields m (← variantInstHandle m i))
  | .unpackVariant h => do unpackFields (← variantFields m h)
  | .unpackVariantGeneric i => do unpackFields (← variantFields m (← variantInstHandle m i))
  | .packClosure h mask => do closPack m (← lookup m.functionHandles h "function_handle_at") mask
  | .packClosureGeneric i mask => do
    let fi ← lookup m.functionInstantiations i "function_instantiation_at"
    closPack m (← lookup m.functionHandles fi.handle "function_handle_at") mask
  | .callClosure idx => do
    let (argTys, resultTys) ← funType m idx
    let _ ← pop
    let args ← popN argTys.length
    let values ← coreCall args resultTys
    values.forM push
  | .vecPack idx n => do
    (List.range n).forM fun _ => popValue
    push (← valueFor (.vector (← vecElementType m idx)))
  | .vecLen _ => do
    vectorOp (← pop) false
    push (← valueFor .u64)
  | .vecImmBorrow _ => do
    popValue
    push (← vectorElementBorrow (← pop) false)
  | .vecMutBorrow _ => do
    popValue
    push (← vectorElementBorrow (← pop) true)
  | .vecPushBack _ => do
    popValue
    vectorOp (← pop) true
  | .vecPopBack idx => do
    vectorOp (← pop) true
    push (← valueFor (← vecElementType m idx))
  | .vecUnpack idx n => do
    popValue
    let ty ← vecElementType m idx
    (List.range n).forM fun _ => do push (← valueFor ty)
  | .vecSwap _ => do
    popValue
    popValue
    vectorOp (← pop) true

/-- `TransferFunctions::execute`. -/
def execute (st : RefState) (instr : Bytecode) (offset blockEnd : Nat) : VResult RefState := do
  let ((), st) ← (do
    executeInner m view instr
    if offset == blockEnd then canonicalize : RefM Unit).run st
  pure st

end ReferenceSafety

def referenceAnalysis (m : CompiledModule) (view : FunctionView) : Analysis RefState where
  join := RefState.join
  execute := ReferenceSafety.execute m view

/-- `reference_safety::verify`. The join adds edges among the locals'
references and the frame root, at most eleven times per pair before the
pair's set collapses, or turns a local's reference into a value, which bounds
the ascending chains. -/
def verifyReferenceSafety (m : CompiledModule) (view : FunctionView) : VResult Unit := do
  let initial ← RefState.new view
  let n := initial.locals.length + 1
  let _ ← analyzeFunction (referenceAnalysis m view) view.cfg view.code.code initial
    (n * n * (maxEdgeSetSize + 1) + n)
  pure ()

end MoveBytecode.Model
