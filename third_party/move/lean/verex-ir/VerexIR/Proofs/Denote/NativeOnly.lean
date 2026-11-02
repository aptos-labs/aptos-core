-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
import Lean

/-!
# Native-only verification

Verification is stated over native values (`doc/designs/native-verification.md`):
a contract, a verification condition, a leaf and the solver's facts mention
no runtime value. The check here names the first subterm that does; the
runtime semantics, the codecs and the agreement are where runtime values
belong, and nothing the verifier runs reaches them.
-/

namespace VerexIR.Proofs.Denote

open Lean Meta

/-- The namespaces of the runtime vocabulary: runtime values, the runtime
map snapshots that only exist over them, the codecs, and the runtime
closures a function value is implemented by. -/
def runtimeNamespaces : List Name :=
  [`VerexIR.RuntimeValue, `VerexIR.Maps, `VerexIR.Proofs.Codec, `VerexIR.Proofs.Denote.Codec,
   `VerexIR.Proofs.Denote.ClosureValue, `VerexIR.Proofs.Denote.RuntimeFunction]

/-- The runtime vocabulary outside those namespaces: the runtime state and
failures, the encodings of native values and failures and their codecs, a
function value's runtime closure, and the runtime decoders. -/
def runtimeConstants : List Name :=
  [`VerexIR.RuntimeState, `VerexIR.Proofs.RuntimeFailure, `VerexIR.Proofs.Failure.encode,
   `VerexIR.Proofs.Denote.NTy.encode, `VerexIR.Proofs.Denote.NTy.codec,
   `VerexIR.Proofs.Denote.NTy.decode?, `VerexIR.Proofs.Denote.HList.encode,
   `VerexIR.Proofs.Denote.HEnv.encode, `VerexIR.Proofs.Denote.closureOf,
   `VerexIR.Proofs.Denote.closureOfGeneric, `VerexIR.Proofs.Denote.closureRun,
   `VerexIR.Proofs.Denote.TypedFunctions.closure, `VerexIR.Proofs.Denote.TypedFunctions.ofClosure?,
   `VerexIR.Proofs.Denote.ResourceType.encode,
   `VerexIR.Proofs.Denote.rowCodec, `VerexIR.Proofs.Denote.rowCodecs,
   `VerexIR.Proofs.Denote.variantCodec, `VerexIR.Proofs.Denote.hlistCodec,
   `VerexIR.Proofs.Denote.resultCodec, `VerexIR.decodeInt?, `VerexIR.decodeBool?,
   `VerexIR.decodeString?, `VerexIR.decodeAddress?, `VerexIR.decodeSigner?,
   `VerexIR.decodeBytes?, `VerexIR.decodeUnit?, `VerexIR.decodeGhost?]

/-- Whether a constant belongs to the runtime vocabulary
(`runtimeNamespaces`, `runtimeConstants`). -/
def isRuntimeName (name : Name) : Bool :=
  runtimeNamespaces.any (·.isPrefixOf name) || runtimeConstants.contains name

/-- The runtime constants `e` mentions, each once, a shared subterm visited
once. -/
def runtimeValueMentions (e : Lean.Expr) (found : Array Name := #[]) : Array Name :=
  e.getUsedConstants.foldl (init := found) fun found name =>
    if isRuntimeName name && !found.contains name then found.push name else found

/-- The runtime constants `e` states, each once, and the first application
stating one: the constants of its applications' heads and explicit
arguments, and of its binders. An implicit or instance argument indexes a
type, as the codec a typed order or read is over (`TypedOrder`), and states
nothing; a runtime value a statement holds is an explicit argument of what
holds it. A shared subterm is visited once. -/
partial def statedRuntimeValues (e : Lean.Expr) : MetaM (Array Name × Option Lean.Expr) := do
  let state ← IO.mkRef ((#[] : Array Name), (none : Option Lean.Expr),
    (∅ : Std.HashSet Lean.Expr))
  let record (name : Name) (at_ : Lean.Expr) : MetaM Unit :=
    state.modify fun (names, witness, seen) =>
      (if names.contains name then names else names.push name, witness.orElse fun _ => some at_, seen)
  let rec visit (e : Lean.Expr) : MetaM Unit := do
    if (← state.get).2.2.contains e then return
    state.modify fun (names, witness, seen) => (names, witness, seen.insert e)
    match e with
    | .const name _ => if isRuntimeName name then record name e
    | .app .. =>
        let function := e.getAppFn
        let arguments := e.getAppArgs
        if let some name := function.constName? then
          if isRuntimeName name then record name e
        else visit function
        let info ← try some <$> getFunInfoNArgs function arguments.size catch _ => pure none
        for argument in arguments, index in [:arguments.size] do
          let explicit : Bool := match info.bind (·.paramInfo[index]?) with
            | some parameter => parameter.binderInfo.isExplicit
            | none => true
          if explicit then
            if let some name := argument.constName? then
              if isRuntimeName name then record name e
            visit argument
    | .lam _ type body _ | .forallE _ type body _ => visit type; visit body
    | .letE _ type value body _ => visit type; visit value; visit body
    | .mdata _ body | .proj _ _ body => visit body
    | _ => pure ()
  visit e
  let (names, witness, _) ← state.get
  return (names, witness)

/-- The error naming `what` and the runtime constants `e` states
(`statedRuntimeValues`) with the first application stating one; none where
it states none. -/
def runtimeValuesMessage? (what : MessageData) (e : Lean.Expr) : MetaM (Option MessageData) := do
  let (mentions, witness?) ← statedRuntimeValues e
  if mentions.isEmpty then return none
  let witness ← match witness? with
    -- A large witness is named by its head: printing a shared term in full
    -- can cost more than the leaf.
    | some witness =>
        if witness.approxDepth < 24 then withOptions (pp.explicit.set · true) (ppExpr witness)
        else pure f!"an application of {witness.getAppFn}"
    | none => pure f!"(no application)"
  return some m!"{what} mentions runtime values, which verification states natively \
    (doc/designs/native-verification.md): {mentions.toList}, first in{indentD witness}"

/-- Report the runtime constants `e` states (`runtimeValuesMessage?`) as an
error. Returns whether any was found. -/
def reportRuntimeValues (what : MessageData) (e : Lean.Expr) : MetaM Bool := do
  let some message ← runtimeValuesMessage? what e | return false
  logError message
  return true

section Authored
open Elab Tactic

/-- The runtime constants an authored proof names: each identifier, plain or
dotted (`.c`), of original syntax in `tree` that elaborated to one
(`isRuntimeName`), with the
constant, once per position. As the elaborator resolved it, so a local, a
binder or a field name is not one, and syntax a tactic expands to is not
authored. -/
partial def authoredRuntimeNames (assignment : PersistentHashMap MVarId InfoTree) (tree : InfoTree)
    (found : Array (Syntax × Name) := #[]) : Array (Syntax × Name) :=
  match tree with
  | .context _ tree => authoredRuntimeNames assignment tree found
  | .hole id => match assignment.find? id with
    | some tree => authoredRuntimeNames assignment tree found
    | none => found
  | .node info children =>
      let found := match info with
        | .ofTermInfo term =>
            match term.expr.getAppFn.constName? with
            | some name =>
                if (term.stx.isIdent || term.stx.isOfKind ``Lean.Parser.Term.dotIdent) &&
                    (term.stx.getHeadInfo matches .original ..) &&
                    isRuntimeName name && !found.any (·.1.getPos? == term.stx.getPos?) then
                  found.push (term.stx, name)
                else found
            | none => found
        | _ => found
      children.foldl (fun found child => authoredRuntimeNames assignment child found) found

/-- Run `act` with info trees recorded, and report each runtime constant an
identifier of its original syntax names (`authoredRuntimeNames`). The trees
are kept where the surrounding elaboration records them. -/
def reportAuthoredRuntimeNames {m : Type → Type} [Monad m] [MonadInfoTree m] [MonadFinally m]
    [MonadLog m] [AddMessageContext m] [MonadOptions m] (act : m α) : m α := do
  let saved ← getInfoState
  let start := saved.trees.size
  enableInfoTree
  try
    let result ← act
    let state ← getInfoState
    for tree in state.trees.toArray.extract start state.trees.size do
      for (stx, name) in authoredRuntimeNames state.assignment tree do
        logErrorAt stx m!"an authored proof names `{name}`, of the runtime vocabulary; \
          proofs are stated natively (doc/designs/native-verification.md)"
    return result
  finally
    unless saved.enabled do setInfoState saved

/-- An authored proof script, its runtime names reported
(`reportAuthoredRuntimeNames`). -/
elab "verex_authored " script:tacticSeq : tactic =>
  reportAuthoredRuntimeNames (evalTactic script)

end Authored

end VerexIR.Proofs.Denote
