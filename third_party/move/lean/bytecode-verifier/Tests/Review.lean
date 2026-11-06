-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Tests.Semantics

/-!
# Clean-room review regressions and `wrong`-reason coverage

The Phase 0 clean-room review of `Spec/` found that two modules with the same
id resolved a call into the other module's private function, and that a
caller's handle disagreeing with the callee's definition went unnoticed.
Both are `wrong` now.

The rest of the file produces every `WrongReason` the supported subset can
reach that `Tests/Semantics.lean` does not. With Phase 2a, `Tests/Semantics.lean`
produces `copyWithoutAbility` and `dropWithoutAbility` (structs without the
ability, signers, vectors of them, reading and writing through references),
`badOperand` and `unresolvedStruct`; with Phase 3, stage A, it produces
`invalidReference`, `immutableReference` and `globalWithoutKey`, and the new
failures `emptyVector`, `vectorIndex`, `resourceLocked`, `missingResource`
and `resourceExists`; with Phase 4, stage A, it produces `constraintViolation`
(a generic call, a generic struct form, and K1's handle that omits a
definition's constraint), and `badOperand` and `linkMismatch` for generic forms
and handles of the wrong arity; with Phase 4, stage B, it produces
`capturedReference`, and `typeMismatch`, `badOperand`, `stackUnderflow`,
`visibility` and the failures `resourceLocked` and `moduleLocked` through
closures; with natives with references (2026-11-03), it produces
`overlappingReferences`, `invalidReference` for a native's dangling argument,
and `nativeContract` for every result outside a native's shape; with the
table natives (2026-11-04), `nativeContract` for results outside their shapes
and for a written-back table that does not conform, and `invalidReference` for
a reference to an entry after `add_box` wrote the table back.
`unsupported` is produced for an instruction, for a type outside the subset,
for a reference as a type argument, for a native with references that is not
modeled (a table native of a module breaking the table rules included), and for
a closure over a native. Every other reason is produced there
or below.
-/

namespace MoveBytecode.Tests

open MoveBytecode

def dupA : CompiledModule :=
  mkModule 0x1 "m" [{ name := "g", returns := [.bool], visibility := .private_,
                      code := #[.ldTrue, .ret] }]

def dupB : CompiledModule :=
  mkModule 0x1 "m" [{ name := "main", returns := [.u64], code := #[.call 1, .ret] },
                    { name := "g", returns := [.u64], visibility := .private_,
                      code := #[.ldU64 1, .ret] }]

#guard runFun ⟨#[dupB]⟩ 0 0 [] == .returned [u64 1]
#guard runFun ⟨#[dupA, dupB]⟩ 1 0 [] == .wrong .unresolvedFunction

def mismatchA : CompiledModule :=
  mkModule 0x1 "a" [{ name := "f", params := [.u64], returns := [.bool],
                      code := #[.ldTrue, .ret] }]

def mismatchB : CompiledModule :=
  mkModule 0x2 "b" [{ name := "main", code := #[.ldU64 9, .call 1, .pop, .ret] }]
    [{ address := 0x1, module := "a", name := "f", params := [.u64], returns := [.u64] }]

#guard runFun ⟨#[mismatchA, mismatchB]⟩ 1 0 [] == .wrong .linkMismatch

#guard runFun (single { name := "c", returns := [.u64], code := #[.ldConst 3, .ret] }) 0 0 [] ==
  .wrong .badIndex

def isWrong (r : WrongReason) : StepResult → Bool
  | .done (.wrong r') => r == r'
  | _ => false

def frameAt (f : FunctionRef) : Frame :=
  { function := f, id := 0, pc := 0, locals := #[], stack := [], countedActive := false,
    lockEntered := false }

def stateWith (frames : List Frame) : State :=
  { frames, active := [], lockDepth := 0, storage := [], nextFrame := frames.length,
    invalidations := [], held := [] }

#guard isWrong .noFrame (step (single addFun) (stateWith []))
#guard isWrong .badFunctionRef (step (single addFun) (stateWith [frameAt ⟨3, 0, []⟩]))
#guard isWrong .nativeFrame (step ⟨#[nativeModule]⟩ (stateWith [frameAt ⟨0, 0, []⟩]))

/-! ## Phase 2b: the D2 and D3 checks can fail

Each check of D2 and D3 is falsified by something it must reject, so none
holds vacuously:

- natives that break their contract (`NativeCallRespects`): passing a
  resource on twice, consuming or creating another module's type, minting a
  signer;
- initial states no transaction produces: an unauthenticated signer, an
  argument of a type the transaction layer does not accept;
- the step a VM would take if it skipped the semantics' ability checks
  (copying or discarding a value without the ability, returning with a
  resource left in a local), or produced a value an instruction may not
  create. The semantics goes wrong at each; the ghost layer applied to the
  unchecked step shows which property that check protects;
- a shadow that does not fit its state, which is unaccounted. -/

/-! ### Natives that break their contract -/

-- `keep` creating its result instead of passing `R` on: it consumes another
-- module's `R`, which has no `drop` (D2b, and so D2c), and creates one (D3a).
#guard violations programRN 1 4 [] nNatives allFresh == ["D2b", "D3a", "D2c"]
#guard !NativeCallRespects programRN {} ⟨1, 0, []⟩ ⟨0x2, "n"⟩ [valRr] [valRr]
  (allFresh ⟨1, 0, []⟩ [valRr] [valRr])
-- An explicit exception gives `keep` the privileges of `r::R`.
def keepPrivileged : NativePolicy := { privileged := fun f s => f == ⟨1, 0, []⟩ && s == typeRr }

#guard violations programRN 1 4 [] nNatives allFresh (π := keepPrivileged) == []
#guard NativeCallRespects programRN keepPrivileged ⟨1, 0, []⟩ ⟨0x2, "n"⟩ [valRr] [valRr]
  (allFresh ⟨1, 0, []⟩ [valRr] [valRr])

-- `dup` passes one `R` on twice (D2a).
#guard violations programRN 1 5 [] nNatives nFlows == ["D2a"]
#guard !NativeCallRespects programRN {} ⟨1, 1, []⟩ ⟨0x2, "n"⟩ [valRr] [valRr, valRr]
  (nFlows ⟨1, 1, []⟩ [valRr] [valRr, valRr])

-- `forge` creates an `r::R` (D3a).
#guard violations programRN 1 6 [] nNatives nFlows == ["D3a"]
#guard !NativeCallRespects programRN {} ⟨1, 2, []⟩ ⟨0x2, "n"⟩ [] [valRr] (nFlows ⟨1, 2, []⟩ [] [valRr])

-- `sig` mints a signer the policy does not allow (D3b).
#guard violations programRN 1 7 [] nNatives nFlows == ["D3b"]
#guard !NativeCallRespects programRN {} ⟨1, 3, []⟩ ⟨0x2, "n"⟩ [] [signerAt 5] (nFlows ⟨1, 3, []⟩ [] [signerAt 5])
#guard NativeCallRespects programRN mints5 ⟨1, 3, []⟩ ⟨0x2, "n"⟩ [] [signerAt 5] (nFlows ⟨1, 3, []⟩ [] [signerAt 5])

-- A flow claiming a result node is an argument node it differs from.
#guard !NativeCallRespects ⟨#[nativeModule]⟩ {} ⟨0, 0, []⟩ ⟨0x1, "n"⟩ [u64 21] [u64 42]
  (fun _ p => .arg 0 p)

/-! ### Initial states no transaction produces (E1, E2) -/

#guard violations (withData (dataFun #[.moveLoc 0, .ret] [.signer] [] [.signer])) 0 0 [signerAt 1]
  (auth := [2]) == ["D3b"]
#guard violations (withData (dataFun #[.moveLoc 0, .ret] [.struct 0] [] [.struct 0])) 0 0
  [valS 1 true] == ["D3a"]

/-! ### Unchecked steps -/

/-- Run `n` annotated steps without natives. -/
def traceSteps (P : Program) : Nat → State → Shadow → List Event →
    Option (State × Shadow × List Event)
  | 0, s, h, evs => some (s, h, evs)
  | n + 1, s, h, evs =>
    match traceStep P noNatives allFresh s h with
    | (.running s', some (.running h', e)) => traceSteps P n s' h' (evs ++ e)
    | _ => none

/-- What a VM would do at `CopyLoc i` without the `copy` check. -/
def uncheckedCopy (i : Nat) (s : State) : Outcome :=
  match s.frames with
  | fr :: rest =>
    match fr.locals[i]? with
    | some (some v) =>
      .running { s with frames := { fr with stack := v :: fr.stack, pc := fr.pc + 1 } :: rest }
    | _ => .wrong .unavailableLocal
  | [] => .wrong .noFrame

/-- What a VM would do at `Pop` without the `drop` check. -/
def uncheckedPop (s : State) : Outcome :=
  match s.frames with
  | fr :: rest =>
    match fr.stack with
    | _ :: st => .running { s with frames := { fr with stack := st, pc := fr.pc + 1 } :: rest }
    | [] => .wrong .stackUnderflow
  | [] => .wrong .noFrame

/-- What a VM would do at the top-level `Ret` without the `drop` check on
locals. -/
def uncheckedRet (s : State) : Outcome :=
  match s.frames with
  | [fr] => .returned fr.stack.reverse s.storage
  | _ => .wrong .noFrame

/-- An instruction that pushed `v`. -/
def pushing (v : Value) (s : State) : Outcome :=
  match s.frames with
  | fr :: rest => .running { s with frames := { fr with stack := v :: fr.stack, pc := fr.pc + 1 } :: rest }
  | [] => .wrong .noFrame

/-- An instruction that popped `n` values and pushed `v`. -/
def replacing (n : Nat) (v : Value) (s : State) : Outcome :=
  match s.frames with
  | fr :: rest =>
    .running { s with frames := { fr with stack := v :: fr.stack.drop n, pc := fr.pc + 1 } :: rest }
  | [] => .wrong .noFrame

/-- What a VM would do at `WriteRef` without the `drop` check on the old
value. -/
def uncheckedWrite (s : State) : Outcome :=
  match s.frames with
  | fr :: rest =>
    match fr.stack with
    | .ref r :: v :: st =>
      let s := { s with frames := { fr with stack := st, pc := fr.pc + 1 } :: rest }
      match s.write? r.location v with
      | some s => .running s
      | none => .wrong .invalidReference
    | _ => .wrong .typeMismatch
  | [] => .wrong .noFrame

/-- After `n` steps of function 0 of `P`, where the semantics' next step is
`expected`: the D2, D3 and D4 checks that fail on the step `unchecked` takes
instead, annotated by the ghost layer. -/
def uncheckedViolations (P : Program) (n : Nat) (expected : Outcome → Bool)
    (unchecked : State → Outcome) (args : List Value := []) : Option (List String) := do
  let s₀ ← initialState P ⟨0, 0, []⟩ args
  let (h₀, evs₀) := Shadow.initial s₀
  let (s, h, evs) ← traceSteps P n s₀ h₀ evs₀
  let .done actual := step P s | none
  guard (expected actual)
  let o := unchecked s
  let (post, e) ← Shadow.coreStep P s o h
  let occ := post.occurrences o
  pure ((if Ownership.unique P occ then [] else ["D2a"]) ++
    (if Ownership.destroysNothing P {} (h.occurrences s) occ e then [] else ["D2b"]) ++
    (match o with
     | .returned _ _ => if Ownership.conserved P {} (evs ++ e) occ then [] else ["D2c"]
     | _ => []) ++
    (if e.all (Encapsulation.structsRespected {} []) then [] else ["D3a"]) ++
    (if e.all (Encapsulation.signersRespected {} []) then [] else ["D3b"]) ++
    (if e.all (Encapsulation.closuresRespected P) then [] else ["D3c"]) ++
    (if RefSafety.unnestedAfter o then [] else ["D4d"]))

def wrongWith (r : WrongReason) : Outcome → Bool
  | .wrong r' => r == r'
  | _ => false

-- Copying an `R` duplicates its identity (D2a).
#guard uncheckedViolations (withData (dataFun #[.pack 1, .stLoc 0, .copyLoc 0, .ret] [] [.struct 1])) 2
  (wrongWith .copyWithoutAbility) (uncheckedCopy 0) == some ["D2a"]
-- `Bad has copy, drop { R }` breaks L6: copying it duplicates the `R` inside.
#guard uncheckedViolations
  (withData (dataFun #[.pack 1, .pack 2, .stLoc 0, .copyLoc 0, .ret] [] [.struct 2])) 3
  (wrongWith .copyWithoutAbility) (uncheckedCopy 0) == some ["D2a"]
-- Discarding an `R`, a `Bad`, or an empty `vector<R>` (D2b).
#guard uncheckedViolations (withData (dataFun #[.pack 1, .pop, .ret])) 1
  (wrongWith .dropWithoutAbility) uncheckedPop == some ["D2b"]
#guard uncheckedViolations (withData (dataFun #[.pack 1, .pack 2, .pop, .ret])) 2
  (wrongWith .dropWithoutAbility) uncheckedPop == some ["D2b"]
#guard uncheckedViolations (withData (dataFun #[.vecPack 5 0, .pop, .ret])) 1
  (wrongWith .dropWithoutAbility) uncheckedPop == some ["D2b"]
-- `C has copy` but not `drop`: copying it is fine (no D2a), discarding one of
-- the two copies is not (D2b counts occurrences).
def copyOnly : Program :=
  ⟨#[mkModule 0x42 "m" [dataFun #[.pack 0, .stLoc 0, .copyLoc 0, .pop, .ret] [] [.struct 0]] [] []
      [{ name := "C", abilities := { copy := true } }]]⟩

#guard uncheckedViolations copyOnly 3 (wrongWith .dropWithoutAbility) uncheckedPop == some ["D2b"]
-- Returning with an `R` left in a local destroys it (D2b) and loses it (D2c).
#guard uncheckedViolations (withData (dataFun #[.pack 1, .stLoc 0, .ret] [] [.struct 1])) 2
  (wrongWith .dropWithoutAbility) uncheckedRet == some ["D2b", "D2c"]
-- A `Pack` in `0x42::m` producing a value of `0x1::a::S` (D3a), and a
-- constant decoding to a signer (D3b): neither can happen in the semantics,
-- whose `Pack` names the current module's definitions and whose decoder
-- rejects signers.
#guard uncheckedViolations (withData (dataFun #[.pack 1, .ret] [.struct 1])) 0
  (fun o => match o with | .running _ => true | _ => false)
  (pushing (.struct ⟨⟨0x1, "a"⟩, "S", []⟩ [])) == some ["D3a"]
#guard uncheckedViolations (withData (dataFun #[.ldConst 0, .ret] [.signer])
    [{ type := .signer, data := List.replicate 32 0 }]) 0
  (wrongWith .badConstant) (pushing (signerAt 0)) == some ["D3b"]

/-! ### Phase 3: reading, writing and storing through references

The semantics' checks on references protect D2 and D4 the same way: what a
VM would do without them is annotated by the ghost layer, and the checks show
the property it breaks. -/

-- Reading through a reference to an `N`, which has no `copy`, duplicates it
-- (D2a).
#guard uncheckedViolations (withRefs (dataFun #[.pack 1, .stLoc 0, .immBorrowLoc 0, .readRef, .ret] []
    [.struct 1])) 3 (wrongWith .copyWithoutAbility) (replacing 1 (.struct typeN [])) == some ["D2a"]
-- Writing over an `N`, which has no `drop`, destroys it (D2b).
#guard uncheckedViolations (withRefs (dataFun #[.pack 1, .stLoc 0, .pack 1, .mutBorrowLoc 0, .writeRef,
    .ret] [] [.struct 1])) 4 (wrongWith .dropWithoutAbility) uncheckedWrite == some ["D2b"]
-- Packing a reference into a vector stores it inside a value (D4d).
#guard uncheckedViolations (withRefs (dataFun #[.immBorrowLoc 0, .vecPack 3 1, .ret] [] [] [.u64])) 1
    (wrongWith .typeMismatch)
    (replacing 1 (.vector (.int .u64) [.ref ⟨false, .int .u64, ⟨.local_ 0 0, []⟩, 0⟩])) (args := [u64 5]) ==
  some ["D4d"]

/-! ### Phase 4, stage B: closures

A closure capturing a reference stores it inside a value (D4d); a closure
created where its target is not visible forges access to a private function
(D3c). -/

def captureRef : Program :=
  ⟨#[mkModule 0x42 "r"
      [{ name := "f", params := [.u64], code := #[.immBorrowLoc 0, .packClosure 1 1#64, .pop, .ret] },
       { name := "capRef", params := [.reference .u64, .u64], returns := [.u64],
         code := #[.moveLoc 1, .ret] }]]⟩

#guard uncheckedViolations captureRef 1 (wrongWith .capturedReference)
    (replacing 1 (.closure ⟨0, 1, []⟩ 1#64 [.ref ⟨false, .int .u64, ⟨.local_ 0 0, []⟩, 0⟩]
      [.int .u64] [.int .u64] ⟨true, true, true, false⟩)) (args := [u64 5]) ==
  some ["D4d"]
-- `0x2::b::forge` creates a closure over `0x1::a::secret`, which is private.
#guard uncheckedViolations ⟨#[visB, visA]⟩ 0 (wrongWith .visibility)
    (pushing (.closure ⟨1, 0, []⟩ 0#64 [] [] [.int .u64] ⟨true, true, false, false⟩)) ==
  some ["D3c"]

/-! ### The ghost layer changes none of the outcomes above -/

#guard agrees ⟨#[dupB]⟩ 0 0 []
#guard agrees ⟨#[dupA, dupB]⟩ 1 0 []
#guard agrees ⟨#[mismatchA, mismatchB]⟩ 1 0 []
#guard agrees (single { name := "c", returns := [.u64], code := #[.ldConst 3, .ret] }) 0 0 []

/-! ### A shadow that does not fit is unaccounted -/

#guard match (initialState (single addFun) ⟨0, 0, []⟩ [u64 2, u64 3]).map
    (fun s => runTraced (single addFun) noNatives allFresh 100 s ⟨[], [], [], 0⟩ []) with
  | some (.unaccounted _) => true
  | _ => false

/-! ## Version-gated format rules

The spec has no deserializer, so `Program.SupportedVersions` states the
format rules it gates on versions: below version 10, no function handle
carries a struct-API attribute. Exactly the seven version-10 attributes are
struct-API ones. -/

#guard [FunctionAttribute.pack, .packVariant 0, .unpack, .unpackVariant 0, .testVariant 0,
  .borrowFieldImmutable 0, .borrowFieldMutable 0].all (·.isStructApi)
#guard ![FunctionAttribute.persistent, .moduleLock].any (·.isStructApi)

def withAttributes (version : Nat) (attributes : List FunctionAttribute) : CompiledModule :=
  { mkModule 0x42 "m" [{ name := "f", code := #[.ret], attributes }] with version }

-- A struct-API attribute is outside the premise below version 10, and inside
-- it at version 10.
example : ¬ (⟨#[withAttributes 9 [.borrowFieldMutable 0]]⟩ : Program).SupportedVersions := by
  unfold Program.SupportedVersions; decide +kernel
example : (⟨#[withAttributes 10 [.borrowFieldMutable 0]]⟩ : Program).SupportedVersions := by
  unfold Program.SupportedVersions; decide +kernel
-- `persistent` is older: version 9 admits it.
example : (⟨#[withAttributes 9 [.persistent]]⟩ : Program).SupportedVersions := by
  unfold Program.SupportedVersions; decide +kernel
-- The rule is per module: a version-9 module beside a version-10 one that
-- carries the attribute is in scope; the other way round it is not.
example : (⟨#[withAttributes 10 [.borrowFieldMutable 0], withAttributes 9 []]⟩ :
    Program).SupportedVersions := by
  unfold Program.SupportedVersions; decide +kernel
example : ¬ (⟨#[withAttributes 10 [], withAttributes 9 [.borrowFieldMutable 0]]⟩ :
    Program).SupportedVersions := by
  unfold Program.SupportedVersions; decide +kernel

end MoveBytecode.Tests
