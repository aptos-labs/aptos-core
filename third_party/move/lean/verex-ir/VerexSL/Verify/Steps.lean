-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Verify.Members
import VerexSL.Verify.SpecForm
import VerexIR.Proofs.Denote.FunctionValueLemmas

/-!
# The proof language

`verify f by block` lowers the steps of its block to the tactics a script
writes (`doc/plan/proof-language.md`, "Semantics and lowering"): the
lowering is a translation, not a tactic of its own.
-/

namespace VerexSL.Verify

open Lean Elab Command
open VerexIR.Validation (ValidatedUnit ValidatedNamespace)
open VerexSL.Contract (findFunction?)

/-- How a step's fact is proved (`doc/plan/proof-language.md`, "Closers"). -/
declare_syntax_cat verexCloser (behavior := symbol)
/-- The bounded deciders over the facts in scope; the default closer. -/
syntax (name := closerDecide) &"decide" : verexCloser
/-- Linear arithmetic over the facts it reads, without case splits. -/
syntax (name := closerArith) &"arith" : verexCloser
/-- Evaluation of closed maps, literal vectors and ground arithmetic. -/
syntax (name := closerEval) &"eval" : verexCloser
/-- The automatic leaf, charged as automatic work. -/
syntax (name := closerSolve) &"solve" : verexCloser
/-- Equality through applications and uninterpreted terms, with linear
arithmetic over the facts in scope; no case split. -/
syntax (name := closerCongr) &"congr" : verexCloser
/-- A polynomial identity over the integers, the facts in scope hidden. -/
syntax (name := closerRing) &"ring" : verexCloser
/-- Lemmas at the stated arguments, then `decide`. -/
syntax (name := closerApply) &"apply" verexExpr,+ : verexCloser
/-- Lean tactics on the fact's goal. -/
syntax (name := closerLean) &"lean" " => " Lean.Parser.Tactic.tacticSeq : verexCloser

/-- The steps of a nested block. -/
syntax verexStepBlock := "{" ppIndent((ppLine verexProofStep)*) ppLine "}"

/-- The block's steps under the fact's negation, until `decide` finds the
contradiction. -/
syntax (name := closerContradiction) &"contradiction" verexStepBlock : verexCloser

/-- `on k, … when d, … { … }` proves the obligations it names, on the paths
its selector takes. -/
syntax (name := stepOn) &"on" verexKey,+ (&"when" verexDecision,+)? verexStepBlock :
  verexProofStep
/-- `on _ { … }` proves every obligation no earlier `on` names. -/
syntax (name := stepOnDefault) &"on" "_" verexStepBlock : verexProofStep
/-- `at p { … }` places facts at a program point (`at entry`, `at exit`,
`at loop k entry|step`): owed once there, inherited by every obligation
after it. -/
syntax (name := stepAt) &"at" verexPoint verexStepBlock : verexProofStep
/-- `after s { … }` places facts where a path continues after a site (`after
call k of g`, `after op k`, `after invoke k`). -/
syntax (name := stepAfter) &"after" verexSite verexStepBlock : verexProofStep
/-- `let x := e` binds `e` for the following steps. -/
syntax (name := stepLet) "let " ident " := " verexExpr : verexProofStep
/-- `let V { f, … } := e` owes `e is V` and binds each listed field `f` of
its payload for the following steps. -/
syntax (name := stepLetVariant) "let " ident " { " ident,+ " }" " := " verexExpr : verexProofStep
/-- `assert e by c` owes `e`, proved by `c` (`decide` without `by`), and gives
it to the following steps. -/
syntax (name := stepAssert) &"assert" verexExpr (" by " verexCloser)? : verexProofStep
/-- `assume e` gives `e` without proof, for the enclosing obligation only;
reported as unsupported until it has a lowering. -/
syntax (name := stepAssume) &"assume" verexExpr : verexProofStep
/-- `apply L(a), …` owes each premise by `decide` and gives the conclusions. -/
syntax (name := stepApply) &"apply" verexExpr,+ : verexProofStep
/-- `split e` continues once per case of a Boolean or a variant value. -/
syntax (name := stepSplit) &"split" verexExpr (&"in" verexExpr)? : verexProofStep
/-- `calc(e₁ op e₂ op …)` asserts each step of the chain and gives its
conclusion. -/
syntax (name := stepCalc) &"calc" "(" verexExpr ")" (" by " verexCloser)? : verexProofStep
/-- `forall x : T where g { … }` gives every fact the block asserts under the
binders and the guard. -/
syntax (name := stepForall) &"forall" verexQuantifierBinder,+ (&"where" verexExpr)?
  verexStepBlock : verexProofStep
/-- `witness e, …` supplies the witnesses of an existential obligation. -/
syntax (name := stepWitness) &"witness" verexExpr,+ : verexProofStep
/-- `if e { … } else { … }` guards steps by a condition without splitting the
obligation's proof. -/
syntax (name := stepIf) "if " verexExpr verexStepBlock (" else " verexStepBlock)? :
  verexProofStep

private def kwLemmaItem := Lean.Parser.nonReservedSymbol "lemma" true
private def kwRequiresItem := Lean.Parser.nonReservedSymbol "requires" true
private def kwEnsuresItem := Lean.Parser.nonReservedSymbol "ensures" true
private def kwDecreasesItem := Lean.Parser.nonReservedSymbol "decreases" true

/-- `lemma L<T, …>(x : T, …) requires … ensures … by block` states a lemma in
the specification vocabulary beside the source, proved by its steps: a
`spec lemma` of the module whose proof is the block. -/
syntax (name := verexLemmaProofItem) kwLemmaItem ident ("<" ident,+ ">")? "(" verexParameter,* ")"
  (ppLine kwRequiresItem verexExpr)* (ppLine kwEnsuresItem verexExpr)+
  (ppLine kwDecreasesItem verexExpr)? " by" verexProofBlock : verexItem

/-! ## Step expressions

A step's expression is lowered by the module's own lowering, in an
unregistered copy of the module that declares one specification function per
expression, whose parameters are the names in scope at the step: the
function's locals, each also as `x'` (its value after the step of the
obligation) and `old_x` (its value at entry, what `old(x)` reads), and the
binders of the steps around it. The registered unit is unchanged. -/

/-- A step's expression to lower: the expression, its block's `let`s
substituted, with the binders the steps around it introduce. -/
structure StepExpression where
  expression : Syntax
  binders : Array (String × Syntax)
  /-- An `apply`'s lemma formula rather than a Boolean. -/
  apply : Bool := false
  /-- The specification type of its value: `Bool` but for a witness, which
  has its existential binder's, and for a split expression, whose type its
  lowering infers. -/
  type : Option String := some "Bool"

/-- A step expression lowered: the copy it is lowered in, the expression, the
locals of the declaration holding it (the scope, as its parameters), and,
for an application of a built-in lemma, the lemma's theorem. -/
structure LoweredStep where
  unit : ValidatedUnit
  expression : VerexIR.ExprId
  locals : Array VerexIR.LocalDecl
  builtin : Option Name := none

/-- The lowered step expressions of the module being verified, by the file
and range of their expressions. -/
private initialize loweredStepsExtension :
    EnvExtension (Array ((String × Nat × Nat) × LoweredStep)) ←
  registerEnvExtension (pure #[])

/-- Where a step's expression is, as `loweredStepsExtension` keys it. -/
def stepKey (file : String) (expression : Syntax) : Option (String × Nat × Nat) := do
  let start ← expression.getPos? true
  let stop ← expression.getTailPos? true
  pure (file, start.byteIdx, stop.byteIdx)

/-- The role functions of the maps of the module being verified, which its
messages print goals with (`specRoles`). -/
private initialize specRolesExtension : EnvExtension String ← registerEnvExtension (pure "")

/-- Run `x` with goals printed in spec form (`VerexSL.SpecForm`), the
module's maps under their role functions: a message built in it keeps its
options. -/
def withSpecForm [Monad m] [MonadEnv m] [MonadWithOptions m] (x : m α) : m α := do
  let roles := specRolesExtension.getState (← getEnv)
  withOptions (SpecForm.options roles) x

/-- The lowering of the step expression at `key`. -/
def loweredStep? (environment : Environment) (key : String × Nat × Nat) : Option LoweredStep :=
  (loweredStepsExtension.getState environment).findSome? fun (k, step) =>
    if k == key then some step else none

/-- What a built-in lemma's parameter is: the map, a key or a value at the
map's types, an integer, a vector, or an element at the vector's type. -/
inductive LemmaParameterKind where
  | map | key | value | int | vector | element
  deriving BEq, Inhabited

/-- A parameter of a built-in lemma: its name in the lemma's item, its kind,
and the binder of the lemma's theorem it fills. -/
structure LemmaParameter where
  name : String
  kind : LemmaParameterKind
  binder : Name
  deriving Inhabited

/-- A lemma of intrinsic maps (`doc/plan/proof-language.md`, "Lemmas"): its
name from the owner's role functions, the `SpecMap` theorem it is, the roles
it reads, and its parameters. Its statement is the theorem's, at an
application's arguments (`lemmaInstance`); the `spec lemma` item a module's
copy declares for it types those arguments. -/
structure MapLemma where
  name : (role : VerexIR.MapSpecRole → String) → String
  theorem_ : Name
  roles : Array VerexIR.MapSpecRole
  parameters : Array LemmaParameter

/-- The word of a role function: its name without `spec_`. -/
def roleWord (name : String) : String :=
  if name.startsWith "spec_" then (name.drop 5).toString else name

/-- A built-in lemma's name: `lemma_` and the words of its parts, a role
function by its word (`lemma_len_remove`). -/
def lemmaName (parts : List String) : String :=
  "lemma_" ++ "_".intercalate (parts.map roleWord)

/-- The lemmas of intrinsic maps, each declared for an owner binding every
role it reads. -/
def mapLemmas : Array MapLemma :=
  let map : LemmaParameter := { name := "m", kind := .map, binder := `map }
  let key (name : String) (binder : Name := `key) : LemmaParameter := { name, kind := .key, binder }
  let int (name : String) (binder : Name) : LemmaParameter := { name, kind := .int, binder }
  let value : LemmaParameter := { name := "v", kind := .value, binder := `value }
  let keys : LemmaParameter := { name := "keys", kind := .vector, binder := `keys }
  #[{ name := fun r => lemmaName [r .hasKey, r .len, "search", "absurd"]
      theorem_ := ``VerexIR.Proofs.Denote.StepLemmas.contains_key_len_search_absurd
      roles := #[.hasKey, .len], parameters := #[map, keys, key "k"] },
    { name := fun r => lemmaName [r .get, r .set], theorem_ := ``VerexIR.SpecMap.valueAt_update
      roles := #[.get, .set], parameters := #[map, key "k", value, key "p" `probe] },
    { name := fun r => lemmaName [r .hasKey, r .set], theorem_ := ``VerexIR.SpecMap.hasKey_update
      roles := #[.hasKey, .set], parameters := #[map, key "k", value, key "p" `probe] },
    { name := fun r => lemmaName [r .len, r .set], theorem_ := ``VerexIR.SpecMap.size_update
      roles := #[.len, .set], parameters := #[map, key "k", value] },
    { name := fun r => lemmaName [r .keyAt, r .set, "present"]
      theorem_ := ``VerexIR.SpecMap.keyAt_update_present
      roles := #[.keyAt, .set], parameters := #[map, key "k", value, int "i" `index] },
    { name := fun r => lemmaName [r .get, r .del], theorem_ := ``VerexIR.SpecMap.valueAt_remove
      roles := #[.get, .del], parameters := #[map, key "k", key "p" `probe] },
    { name := fun r => lemmaName [r .hasKey, r .del], theorem_ := ``VerexIR.SpecMap.hasKey_remove
      roles := #[.hasKey, .del], parameters := #[map, key "k", key "p" `probe] },
    { name := fun r => lemmaName [r .len, r .del], theorem_ := ``VerexIR.SpecMap.size_remove
      roles := #[.len, .del], parameters := #[map, key "k"] },
    { name := fun r => lemmaName [r .keyAt, r .del], theorem_ := ``VerexIR.SpecMap.keyAt_remove_ordered
      roles := #[.keyAt, .del, .rank], parameters := #[map, key "k", int "i" `index] },
    { name := fun r => lemmaName [r .len, r .new_], theorem_ := ``VerexIR.SpecMap.size_empty
      roles := #[.len, .new_], parameters := #[] },
    { name := fun r => lemmaName [r .hasKey, r .new_], theorem_ := ``VerexIR.SpecMap.hasKey_empty
      roles := #[.hasKey, .new_], parameters := #[key "k"] },
    { name := fun r => lemmaName [r .hasKey, r .keyAt], theorem_ := ``VerexIR.SpecMap.hasKey_keyAt
      roles := #[.hasKey, .keyAt], parameters := #[map, int "i" `index] },
    { name := fun r => lemmaName [r .rank, r .keyAt], theorem_ := ``VerexIR.SpecMap.rank_keyAt
      roles := #[.rank, .keyAt], parameters := #[map, int "i" `index] },
    { name := fun r => lemmaName [r .keyAt, r .rank], theorem_ := ``VerexIR.SpecMap.keyAt_rank
      roles := #[.keyAt, .rank], parameters := #[map, key "k"] },
    { name := fun r => lemmaName [r .keyAt, "eq_iff"], theorem_ := ``VerexIR.SpecMap.keyAt_eq_keyAt_iff
      roles := #[.keyAt], parameters := #[map, int "i" `first, int "j" `second] },
    { name := fun r => lemmaName [r .rank, "lt", r .len], theorem_ := ``VerexIR.SpecMap.rank_lt_size
      roles := #[.rank, .len], parameters := #[map, key "k"] },
    { name := fun r => lemmaName [r .rank, "nonneg"], theorem_ := ``VerexIR.SpecMap.rank_nonneg
      roles := #[.rank], parameters := #[map, key "k"] },
    { name := fun r => lemmaName [r .rank, "lt", r .rank], theorem_ := ``VerexIR.SpecMap.rank_lt_rank
      roles := #[.rank], parameters := #[map, key "a" `a, key "b" `b] },
    { name := fun r => lemmaName [r .keyAt, "lt"], theorem_ := ``VerexIR.SpecMap.keyAt_lt_of_lt
      roles := #[.keyAt], parameters := #[map, int "first" `first, int "second" `second] },
    { name := fun r => lemmaName [r .keyAt, r .set, "before"],
      theorem_ := ``VerexIR.SpecMap.keyAt_update_before
      roles := #[.keyAt, .set, .rank], parameters := #[map, key "k", value, int "i" `index] },
    { name := fun r => lemmaName [r .keyAt, r .set, "after"],
      theorem_ := ``VerexIR.SpecMap.keyAt_update_after
      roles := #[.keyAt, .set, .rank], parameters := #[map, key "k", value, int "i" `index] }]

/-- A built-in lemma over integers and vectors (`doc/plan/proof-language.md`,
"Lemmas"): its name, its theorem, and its parameters. Its statement is the
theorem's at an application's arguments (`lemmaInstance`). -/
structure BuiltinLemma where
  name : String
  theorem_ : Name
  parameters : Array LemmaParameter

/-- The arithmetic and vector lemmas. -/
def builtinLemmas : Array BuiltinLemma :=
  let param (name : String) (kind : LemmaParameterKind) : LemmaParameter :=
    { name, kind, binder := Name.mkSimple name }
  let ints (names : List String) := names.toArray.map (param · .int)
  let entry (name : String) (theorem_ : Name) (parameters : Array LemmaParameter) : BuiltinLemma :=
    { name, theorem_ := `VerexIR.Proofs.Denote.StepLemmas ++ theorem_, parameters }
  let v := param "v" .vector
  let e := param "e" .element
  #[entry "lemma_mul_nonneg" `mul_nonneg (ints ["a", "b"]),
    entry "lemma_mul_mono" `mul_mono (ints ["a", "b", "c", "d"]),
    entry "lemma_mul_strict_mono" `mul_strict_mono (ints ["a", "b", "c"]),
    entry "lemma_div_nonneg" `div_nonneg (ints ["x", "c"]),
    entry "lemma_div_le" `div_le (ints ["x", "q", "c"]),
    entry "lemma_div_lt" `div_lt (ints ["x", "q", "c"]),
    entry "lemma_le_div" `le_div (ints ["x", "q", "c"]),
    entry "lemma_div_mono" `div_mono (ints ["x", "y", "c"]),
    entry "lemma_div_mul_le" `div_mul_le (ints ["x", "c"]),
    entry "lemma_div_add_mul" `div_add_mul (ints ["x", "b", "c"]),
    entry "lemma_mod_small" `mod_small (ints ["x", "c"]),
    entry "lemma_shl_mul" `shl_mul (ints ["x", "k"]),
    entry "lemma_shr_div" `shr_div (ints ["x", "k"]),
    entry "lemma_vector_len_remove" `vector_len_remove #[v, param "i" .int],
    entry "lemma_vector_index_remove" `vector_index_remove #[v, param "i" .int, param "k" .int],
    entry "lemma_vector_len_pop_back" `vector_len_pop_back #[v],
    entry "lemma_vector_index_pop_back" `vector_index_pop_back #[v, param "k" .int],
    entry "lemma_vector_len_swap" `vector_len_swap #[v, param "i" .int, param "j" .int],
    entry "lemma_vector_index_swap" `vector_index_swap
      #[v, param "i" .int, param "j" .int, param "k" .int],
    entry "lemma_vector_len_swap_remove" `vector_len_swap_remove #[v, param "i" .int],
    entry "lemma_vector_index_swap_remove" `vector_index_swap_remove
      #[v, param "i" .int, param "k" .int],
    entry "lemma_vector_len_push_back" `vector_len_push_back #[v, e],
    entry "lemma_vector_index_push_back" `vector_index_push_back #[v, e, param "k" .int],
    entry "lemma_vector_len_update" `vector_len_update #[v, param "i" .int, e],
    entry "lemma_vector_index_update" `vector_index_update
      #[v, param "i" .int, e, param "k" .int],
    entry "lemma_vector_len_concat" `vector_len_concat #[v, param "w" .vector],
    entry "lemma_vector_index_concat" `vector_index_concat #[v, param "w" .vector, param "k" .int],
    entry "lemma_vector_len_slice" `vector_len_slice #[v, param "a" .int, param "b" .int],
    entry "lemma_vector_index_slice" `vector_index_slice
      #[v, param "a" .int, param "b" .int, param "k" .int],
    entry "lemma_vector_index_of" `vector_index_of #[v, e],
    entry "lemma_vector_contains_index" `vector_contains_index #[v, param "i" .int],
    entry "lemma_vector_contains_remove" `vector_contains_remove #[v, param "i" .int, e],
    entry "lemma_vector_ext" `vector_ext #[v, param "w" .vector],
    entry "lemma_stored_invariants_frame" `stored_invariants_frame #[]]

/-- The `spec lemma` item of a built-in lemma, a signature that types an
application's arguments: a vector's elements at a type parameter. -/
def builtinLemmaItem (lemma : BuiltinLemma) : String :=
  let typeOf : LemmaParameterKind → String
    | .vector => "Vector<T>" | .element => "T" | _ => "Int"
  let generic := lemma.parameters.any fun parameter =>
    parameter.kind == .vector || parameter.kind == .element
  let parameters := ", ".intercalate (lemma.parameters.toList.map fun parameter =>
    s!"{parameter.name} : {typeOf parameter.kind}")
  s!"spec lemma {lemma.name}{if generic then " {T}" else ""}({parameters}) where\n  ensures true"

/-- The names of the lemmas of a namespace's intrinsic maps, each with its
`SpecMap` theorem: those whose roles an owner binds. -/
def namespaceMapLemmaNames (ns : ValidatedNamespace) : Array (String × Name) := Id.run do
  let nameOf (reference : VerexIR.QualifiedRef) : Option String :=
    (ns.tables.names[reference.name.index]?).map (·.name)
  let mut found := #[]
  for intrinsic in ns.intrinsics do
    unless intrinsic.model == "map" do continue
    let bound (role : VerexIR.MapSpecRole) : Option String :=
      (intrinsic.specBindings.find? (·.role == role.name)).bind fun binding =>
        nameOf binding.target
    for mapLemma in mapLemmas do
      unless mapLemma.roles.all (fun role => (bound role).isSome) do continue
      found := found.push (mapLemma.name fun role => (bound role).getD "", mapLemma.theorem_)
  return found

/-- The role functions of a unit's maps as `verex.specRoles` states them,
each role by its source name; a role two maps name differently is left
out. -/
def specRoles (unit : ValidatedUnit) : String := Id.run do
  let mut bound : Array (String × String) := #[]
  let mut conflicting : Array String := #[]
  for ns in unit.namespaces do
    for intrinsic in ns.intrinsics do
      unless intrinsic.model == "map" do continue
      for binding in intrinsic.specBindings do
        let some name := (ns.tables.names[binding.target.name.index]?).map (·.name) | continue
        if conflicting.contains binding.role then continue
        match bound.find? (·.1 == binding.role) with
        | none => bound := bound.push (binding.role, name)
        | some (_, known) =>
            if known != name then
              bound := bound.filter (·.1 != binding.role)
              conflicting := conflicting.push binding.role
  return ",".intercalate (bound.toList.map fun (role, name) => s!"{role}={name}")

/-- The lemmas of a namespace's intrinsic maps, each with its name, its
`SpecMap` theorem, and its `spec lemma` item: a signature that types an
application's arguments, the map at the owner's type, a key and a value at
its type parameters. -/
def namespaceMapLemmas (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (printing : ValidatedNamespace := ns) (included : VerexIR.NameId → Bool := fun _ => true) :
    Except String (Array (String × Name × String)) := do
  let nameOf (reference : VerexIR.QualifiedRef) : Option String :=
    (ns.tables.names[reference.name.index]?).map (·.name)
  let mut found := #[]
  for intrinsic in ns.intrinsics do
    unless intrinsic.model == "map" && included intrinsic.owner do continue
    let bound (role : VerexIR.MapSpecRole) : Option String :=
      (intrinsic.specBindings.find? (·.role == role.name)).bind fun binding =>
        nameOf binding.target
    -- The map, key and value types, from the reading of a value.
    let some get := bound .get | continue
    let some getFunction := ns.specFunctions.find? fun declaration =>
        (ns.tables.names[declaration.name.index]?.map (·.name)) == some get | continue
    let generics := getFunction.signature.generics
    let typeText (id : VerexIR.TypeId) := Print.specificationTypeText unit printing generics id
    let some mapParameter := getFunction.signature.parameters[0]? | continue
    let some keyParameter := getFunction.signature.parameters[1]? | continue
    let some valueResult := getFunction.signature.results[0]? | continue
    let mapType ← typeText mapParameter.typeUse.typeId
    let keyType ← typeText keyParameter.typeUse.typeId
    let valueType ← typeText valueResult.typeId
    let typeOf : LemmaParameterKind → String
      | .map => mapType | .key | .element => keyType | .value => valueType | .int => "Int"
      | .vector => s!"Vector<{keyType}>"
    let binders ← Print.genericBindersText unit generics
    for mapLemma in mapLemmas do
      unless mapLemma.roles.all (fun role => (bound role).isSome) do continue
      let name := mapLemma.name fun role => (bound role).getD ""
      let parameters := ", ".intercalate (mapLemma.parameters.toList.map fun parameter =>
        s!"{parameter.name} : {typeOf parameter.kind}")
      found := found.push (name, mapLemma.theorem_,
        s!"spec lemma {name}{if binders.isEmpty then "" else " " ++ binders}({parameters}) where" ++
          "\n  ensures true")
  return found

/-- The name of a storage index's head that is one unqualified identifier
without type arguments: `l` of `l[i]`, which the parser reads as a storage
index and the lowering as an index where `l` names a value. -/
private def plainStorageHead? (stx : Syntax) : Option Name := do
  guard (stx.isOfKind ``verexStorageIndexExpr)
  let head := stx[0]
  guard (head.isOfKind ``verexNamedStorageHead)
  let rec parts : Syntax → Array Syntax
    | stx@(.ident ..) => #[stx]
    | stx@(.atom ..) => #[stx]
    | .node _ _ arguments => arguments.flatMap parts
    | .missing => #[]
  match parts head with
  | #[ident@(.ident ..)] => some ident.getId
  | _ => none

/-- The component a result expression names: `k` of `result.k` and of
`spec.result[k]`. -/
private def resultComponent? (stx : Syntax) : Option Nat :=
  if stx.isOfKind ``verexSpecificationResultExpr then stx[2].isNatLit?
  else if stx.isOfKind ``verexFieldExpr && stx[0].isOfKind ``verexLocalExpr &&
      stx[0][0].getId == `result && stx[2].isOfKind ``verexNumericFieldIdentifier then
    stx[2][0].isNatLit?
  else none

/-- An expression with the `let`s before it substituted and `old(x)` read as
the parameter `old_x`. -/
private partial def stepExpression (lets : Array (Name × Syntax)) (stx : Syntax) : Syntax :=
  -- Parenthesized at the name's place, so that an expression around it
  -- keeps a range from its first token to its last.
  let atom (text : String) := Syntax.atom (SourceInfo.fromRef stx (canonical := true)) text
  if stx.isOfKind ``verexLocalExpr then
    match lets.find? (·.1 == stx[0].getId) with
    | some (_, value) => mkNode ``verexParenExpr #[atom "(", value, atom ")"]
    | none => stx
  else if let some (_, value) := (plainStorageHead? stx).bind fun name => lets.find? (·.1 == name) then
    -- A `let` name indexed, `l[i]`: an index of the bound value.
    mkNode ``verexIndexExpr #[mkNode ``verexParenExpr #[atom "(", value, atom ")"], stx[1],
      stepExpression lets stx[2], stx[3], mkNullNode]
  else if let some k := resultComponent? stx then
    -- A component of several results, `result.k` or `spec.result[k]`.
    mkNode ``verexLocalExpr #[mkIdentFrom stx (Name.mkSimple s!"result_{k}") (canonical := true)]
  else if stx.isOfKind ``verexSpecificationOldExpr && stx[2].isOfKind ``verexLocalExpr then
    let name := stx[2][0].getId
    mkNode ``verexLocalExpr #[mkIdentFrom stx (Name.mkSimple s!"old_{name}") (canonical := true)]
  else match stx with
    | .node info kind arguments => .node info kind (arguments.map (stepExpression lets))
    | _ => stx

/-- An `apply` formula's guards and the lemma application they guard:
`c ==> L(a)`, read through parentheses around the application. -/
partial def formulaGuards (formula : Syntax) : Array Syntax × Syntax :=
  if formula.isOfKind ``verexImpliesExpr then
    let (guards, application) := formulaGuards formula[2]
    (#[formula[0]] ++ guards, application)
  else if formula.isOfKind ``verexParenExpr then
    let (guards, application) := formulaGuards formula[1]
    if guards.isEmpty then (#[], formula) else (guards, application)
  else (#[], formula)

/-- `∀ (x : T; …), φ` of an `apply`: its typed binders and `φ`; none for
any other formula. -/
def quantifiedFormula? (formula : Syntax) : Option (Array (String × Syntax) × Syntax) := do
  let formula := if formula.isOfKind ``verexParenExpr then formula[1] else formula
  guard (formula.isOfKind ``verexQuantifierExpr)
  let keyword := ((formula[0].reprint).getD "").trim
  guard (keyword == "∀" || keyword == "forall")
  let binders := formula.getArgs.flatMap fun part =>
    if part.isOfKind ``verexQuantifierTypeBinderSyntax ||
        part.isOfKind ``verexQuantifierBinderSyntax then #[part]
    else part.getArgs.flatMap fun inner =>
      if inner.isOfKind ``verexQuantifierTypeBinderSyntax ||
          inner.isOfKind ``verexQuantifierBinderSyntax then #[inner]
      else inner.getArgs.filter fun binder =>
        binder.isOfKind ``verexQuantifierTypeBinderSyntax ||
          binder.isOfKind ``verexQuantifierBinderSyntax
  let typed ← binders.mapM fun binder => do
    guard (binder.isOfKind ``verexQuantifierTypeBinderSyntax)
    guard (binder[0].isOfKind ``verexVariableBindingPattern)
    pure (binder[0][1].getId.toString, binder[2])
  guard (!typed.isEmpty)
  pure (typed, formula[formula.getNumArgs - 1])

/-- The expressions of an `apply` formula: its guards, Boolean, and the
lemma application they guard. -/
private def applyExpressions (lets : Array (Name × Syntax)) (binders : Array (String × Syntax))
    (formula : Syntax) : Array StepExpression :=
  let (introduced, formula) := (quantifiedFormula? formula).getD (#[], formula)
  let binders := binders ++ introduced
  let (guards, application) := formulaGuards formula
  guards.map (fun guard => { expression := stepExpression lets guard, binders }) |>.push
    { expression := stepExpression lets application, binders, apply := true }

/-- The steps of a block or of a nested block. -/
def blockSteps (block : Syntax) : Array Syntax :=
  if block.isOfKind ``verexProofBlock then block[0].getArgs else block[1].getArgs

/-- The relations of a `calc` chain, with their symbols. -/
private def chainRelations : Array (SyntaxNodeKind × String) := #[
  (``verexEqualExpr, "=="), (``verexNotEqualExpr, "!="), (``verexLessExpr, "<"),
  (``verexLessEqualExpr, "<="), (``verexGreaterExpr, ">"), (``verexGreaterEqualExpr, ">=")]

/-- A chain's first operand and its links' relations, symbols and right
operands: the left spine of its relations, which parse left-associated. -/
private partial def chainSpine (stx : Syntax) (links : List (SyntaxNodeKind × Syntax × Syntax)) :
    Syntax × List (SyntaxNodeKind × Syntax × Syntax) :=
  if chainRelations.any (·.1 == stx.getKind) then
    chainSpine stx[0] ((stx.getKind, stx[1], stx[2]) :: links)
  else (stx, links)

/-- The relation links compose to: `==` of equations, an order where every
link is the order or an equation, strict where one link is; none where the
links run in opposite directions or one is `!=`. -/
private def composedRelation (relations : Array SyntaxNodeKind) : Option SyntaxNodeKind :=
  let up := relations.any fun r => r == ``verexLessExpr || r == ``verexLessEqualExpr
  let down := relations.any fun r => r == ``verexGreaterExpr || r == ``verexGreaterEqualExpr
  if relations.contains ``verexNotEqualExpr || (up && down) then none
  else if up then some (if relations.contains ``verexLessExpr then ``verexLessExpr else ``verexLessEqualExpr)
  else if down then
    some (if relations.contains ``verexGreaterExpr then ``verexGreaterExpr else ``verexGreaterEqualExpr)
  else some ``verexEqualExpr

/-- A `calc` chain `e₀ op₁ e₁ … opₙ eₙ`: its links `eᵢ₋₁ opᵢ eᵢ`, and its
conclusion `e₀ op eₙ`, `op` the relation the links compose to. A chain of
one link is its own conclusion. -/
def calcChain (chain : Syntax) : Except String (Array Syntax × Syntax) := do
  let (first, links) := chainSpine chain []
  let links := links.toArray
  if links.isEmpty then throw "a `calc` chain relates two expressions or more"
  if links.size == 1 then return (#[chain], chain)
  let operands := #[first] ++ links.map (·.2.2)
  let linked := links.zipIdx.map fun ((relation, symbol, right), index) =>
    mkNode relation #[operands[index]!, symbol, right]
  let some relation := composedRelation (links.map (·.1))
    | throw "the chain's links compose to no relation: they run in opposite directions or one is `!=`"
  let some (_, symbol) := chainRelations.find? (·.1 == relation) | throw "the chain has no relation"
  return (linked, mkNode relation #[first, mkAtom symbol, operands.back!])

/-- The test `e is V` a `let V { … } := e` owes, spelled from the variant's
name to the value's end: its range is the step's, apart from the value's. -/
def variantTest (step : Syntax) : Syntax :=
  let value := step[6]
  let atEnd (text : String) := Syntax.atom (SourceInfo.fromRef value (canonical := true)) text
  let test := mkNode ``verexVariantTestSurfaceExpr
    #[value, atEnd "is", mkIdentFrom value step[1].getId (canonical := true), mkNullNode]
  mkNode ``verexParenExpr
    #[Syntax.atom (SourceInfo.fromRef step[1] (canonical := true)) "(", test, atEnd ")"]

/-- The field `f` of the value `e` of a `let V { … } := e`, spelled at the
field's name: `(e).f`. -/
def variantField (value field : Syntax) : Syntax :=
  let atom (text : String) := Syntax.atom (SourceInfo.fromRef field (canonical := true)) text
  mkNode ``verexFieldExpr #[mkNode ``verexParenExpr #[atom "(", value, atom ")"], atom ".",
    mkNode ``verexNamedFieldIdentifier #[field]]

/-- The binders of a `forall` step: their names and types. -/
private def forallBinders (step : Syntax) : Array (String × Syntax) :=
  step[1].getSepArgs.filterMap fun binder => do
    guard (binder.isOfKind ``verexQuantifierTypeBinderSyntax)
    let pattern := binder[0]
    guard (pattern.isOfKind ``verexVariableBindingPattern)
    pure (pattern[1].getId.toString, binder[2])

/-- The expressions of a block's steps, in order: what `assert`, `calc`, a
guard and a split of an expression state, and the witnesses of an
`on` block's obligation, at the types `witnesses` gives that block. A split
of a name splits the value in scope; it lowers nothing. -/
partial def blockExpressions (block : Syntax) (lets : Array (Name × Syntax) := #[])
    (binders : Array (String × Syntax) := #[]) (witnesses : Syntax → Array String := fun _ => #[])
    (types : Array String := #[]) (keyBinders : Syntax → Array (String × Syntax) := fun _ => #[]) :
    Array StepExpression := Id.run do
  let mut found := #[]
  let mut lets := lets
  for step in blockSteps block do
    let expression (stx : Syntax) : StepExpression :=
      { expression := stepExpression lets stx, binders }
    if step.isOfKind ``stepOn then
      found := found ++ blockExpressions step[3] lets (binders ++ keyBinders step) witnesses
        (witnesses step)
    else if step.isOfKind ``stepOnDefault || step.isOfKind ``stepAt || step.isOfKind ``stepAfter then
      found := found ++ blockExpressions step[2] lets binders witnesses
    else if step.isOfKind ``stepLet then
      lets := lets.push (step[1].getId, stepExpression lets step[3])
    else if step.isOfKind ``stepLetVariant then
      -- The value, untyped; the test it owes; each field, whose lowering
      -- checks that the variant has it.
      let value := stepExpression lets step[6]
      found := found.push { expression := value, binders, type := none }
      found := found.push (expression (variantTest step))
      for field in step[3].getSepArgs do
        found := found.push { expression := variantField value field, binders, type := none }
        lets := lets.push (field.getId, variantField value field)
    else if step.isOfKind ``stepAssert then
      found := found.push (expression step[1])
      if step[2].getNumArgs == 2 && step[2][1].isOfKind ``closerApply then
        found := found ++ step[2][1][1].getSepArgs.flatMap (applyExpressions lets binders)
      if step[2].getNumArgs == 2 && step[2][1].isOfKind ``closerContradiction then
        found := found ++ blockExpressions step[2][1][1] lets binders
    else if step.isOfKind ``stepApply then
      found := found ++ step[1].getSepArgs.flatMap (applyExpressions lets binders)
    else if step.isOfKind ``stepSplit then
      -- A range split reads a name in scope and literal bounds; any other
      -- split's expression is lowered for its type.
      unless step[2].getNumArgs > 0 do
        found := found.push { expression step[1] with type := none }
    else if step.isOfKind ``stepCalc then
      if let .ok (links, conclusion) := calcChain step[2] then
        found := found ++ links.map expression
        if links.size > 1 then found := found.push (expression conclusion)
    else if step.isOfKind ``stepWitness then
      found := found ++ step[1].getSepArgs.zipIdx.map fun (witness, position) =>
        { expression witness with type := some ((types[position]?).getD "Bool") }
    else if step.isOfKind ``stepIf then
      found := found.push (expression step[1])
      found := found ++ blockExpressions step[2] lets binders
      if step[3].getNumArgs == 2 then found := found ++ blockExpressions step[3][1] lets binders
    else if step.isOfKind ``stepForall then
      let inner := binders ++ forallBinders step
      if step[2].getNumArgs == 2 then
        found := found.push { expression := stepExpression lets step[2][1], binders := inner }
      found := found ++ blockExpressions step[3] lets inner
  return found

/-- Syntax with its positions shifted by `base`: into the band of its source
(`sourceBand`), where the lowering of the module's copy records it. -/
private partial def shifted (base : Nat) : Syntax → Syntax
  | .node info kind arguments => .node (shiftInfo info) kind (arguments.map (shifted base))
  | .atom info value => .atom (shiftInfo info) value
  | .ident info raw name preresolved => .ident (shiftInfo info) raw name preresolved
  | other => other
where
  shiftInfo : SourceInfo → SourceInfo
    | .original leading start trailing stop =>
        .original leading ⟨start.byteIdx + base⟩ trailing ⟨stop.byteIdx + base⟩
    | .synthetic start stop canonical => .synthetic ⟨start.byteIdx + base⟩ ⟨stop.byteIdx + base⟩ canonical
    | .none => .none

/-- Give every node of `stx` the range of `reference`, so that a diagnostic
of a step item's scope is reported at its step; the expression spliced into
it keeps its own ranges. -/
private partial def atRangeOf (reference : Syntax) (stx : Syntax) : Syntax :=
  let info := match reference.getPos? true, reference.getTailPos? true with
    | some start, some stop => SourceInfo.synthetic start stop
    | _, _ => SourceInfo.none
  let rec go : Syntax → Syntax
    | .node _ kind arguments => .node info kind (arguments.map go)
    | .atom _ value => .atom info value
    | .ident _ raw name preresolved => .ident info raw name preresolved
    | other => other
  go stx

/-- The bands of the sources a module's copy lowers besides the module's
own (`CompilationUnit.sources`): the proof file's step items, then the map
lemmas, laid out one after another. -/
private def proofBand : Nat := sourceBand
private def lemmaBand : Nat := 2 * sourceBand


/-- The declaration of the callee `name` of a function's body, with its
namespace. -/
def calleeDeclaration? (unit : ValidatedUnit) (namespaceIndex : Nat) (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) (name : String) :
    Option (ValidatedNamespace × VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) := Id.run do
  let .structured root := declaration.body | return none
  let mut pending := #[root]
  let mut visited : Std.HashSet Nat := {}
  while let some id := pending.back? do
    pending := pending.pop
    if visited.contains id.index then continue
    visited := visited.insert id.index
    let some expression := ns.expressions[id.index]? | continue
    if let .operation (.call (.function reference)) _ _ _ := expression.kind then
      if let some handle := VerexIR.SemanticOperations.resolveFunction? unit ⟨namespaceIndex⟩ reference then
        if let some calleeNs := unit.namespaces[handle.namespaceId.index]? then
          if let some callee := calleeNs.functions[handle.functionId.index]? then
            if (calleeNs.tables.names[callee.name.index]?).map (·.name) == some name then
              return some (calleeNs, callee)
    pending := pending ++ VerexIR.Validation.expressionChildren expression.kind
  return none

/-- The clause a key names, with the namespace holding it: a clause of the
function by its kind and number (`clauseNames`: `ensures 2`, `invariant 1
step`), or a callee's precondition by its number (`requires 1 of g`,
`preconditionNumbers`). -/
def clauseOf? (unit : ValidatedUnit) (namespaceIndex : Nat) (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (key : VerexIR.Proofs.Denote.ObligationKey) :
    Option (ValidatedNamespace × VerexIR.FunctionDecl VerexIR.Validation.FunctionBody ×
      VerexIR.ExprId) := do
  let index ← key.index
  if key.kind == "requires" then
    let (calleeNs, callee) ← calleeDeclaration? unit namespaceIndex ns declaration (← key.callee)
    let (range, _) ← (preconditionNumbers unit callee).find? (·.2 == index)
    let condition ← callee.contract.conditions.find? fun condition =>
      condition.kind == .requires && Contract.locRange unit condition.loc == range
    return (calleeNs, callee, condition.expression)
  let (range, _) ← (clauseNames unit ns declaration).find? (·.2 == s!"{key.kind}_{index}")
  let inBody := match declaration.body with
    | .structured root => (bodySpecifications ns root fun _ => true).flatMap (·.2.conditions)
    | _ => #[]
  let condition ← (declaration.contract.conditions ++ inBody).find? fun condition =>
    Contract.locRange unit condition.loc == range
  pure (ns, declaration, condition.expression)

/-- Whether an expression reads a map: it or a subexpression has an intrinsic
map's type. -/
def readsMapValue (unit : ValidatedUnit) (ns : ValidatedNamespace) (root : VerexIR.ExprId) : Bool :=
  Id.run do
    let mut pending := #[root]
    let mut visited : Std.HashSet Nat := {}
    while let some id := pending.back? do
      pending := pending.pop
      if visited.contains id.index then continue
      visited := visited.insert id.index
      let some expression := ns.expressions[id.index]? | continue
      if let some type := unit.tables.types[expression.typeId.index]? then
        if (Contract.mapModel? unit type).isSome then return true
      pending := pending ++ VerexIR.Validation.expressionChildren expression.kind
    return false

/-- The names of the binders of a clause's leading universal quantifiers,
under its premises, in order: the names a leaf introduces for them. -/
partial def clauseBinders (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) (clause : VerexIR.ExprId) :
    Array String :=
  match ns.expressions[clause.index]? with
  | some { kind := .quantifier .forall binders _ _ body, .. } =>
      let names := binders.filterMap fun binder => do
        let { kind := .variable localId, .. } ← ns.patterns[binder.pattern.index]? | none
        (declaration.locals[localId.index]?).map (·.name)
      names ++ clauseBinders ns declaration body
  | some { kind := .operation (.primitive .implies) _ #[_, body] _, .. } =>
      clauseBinders ns declaration body
  | _ => #[]

/-- What a leaf of a clause needs of preparation and scope: whether the
clause reads a map (none where the key names no clause: a data or module
invariant, a lemma's precondition), and the names of its binders. -/
structure ClauseNeeds where
  reads : Option Bool
  binders : Array String := #[]
  deriving Inhabited

/-- The names of the leading universal binders of the data invariants of
the unit's types (`clauseBinders`): what a data invariant's leaf introduces
under its source names, at a construction or a mutation's end. -/
def dataInvariantBinders (unit : ValidatedUnit) : Array String := Id.run do
  let mut names := #[]
  for ns in unit.namespaces do
    for declaration in ns.structs do
      for condition in declaration.contract.conditions do
        unless condition.kind == .structInvariant do continue
        let carrier : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody :=
          { (default : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) with
            locals := declaration.locals }
        for name in clauseBinders ns carrier condition.expression do
          unless names.contains name do names := names.push name
  return names

/-- The names of a clause's leading binders over the state domain, its
state labels. -/
partial def stateBinders (ns : ValidatedNamespace) (clause : VerexIR.ExprId) : Array String :=
  match ns.expressions[clause.index]? with
  | some { kind := .quantifier .forall binders _ _ body, .. } =>
      let names := binders.filterMap fun binder => do
        let label ← binder.label
        (ns.tables.names[label]?).map (·.name)
      names ++ stateBinders ns body
  | some { kind := .operation (.primitive .implies) _ #[_, body] _, .. } => stateBinders ns body
  | _ => #[]

/-- The data invariants' binders (`dataInvariantBinders`) with their types
as specification syntax, a state label's binder of the state domain. -/
def dataInvariantBinderTypes (unit : ValidatedUnit) : CommandElabM (Array (String × Syntax)) := do
  let mut found := #[]
  for ns in unit.namespaces do
    for declaration in ns.structs do
      for condition in declaration.contract.conditions do
        unless condition.kind == .structInvariant do continue
        let carrier : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody :=
          { (default : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) with
            locals := declaration.locals }
        for name in clauseBinders ns carrier condition.expression do
          if found.any (·.1 == name) then continue
          let some localDecl := declaration.locals.find? (·.name == name) | continue
          -- A state label's binder ranges over the state domain.
          let text := if (stateBinders ns condition.expression).contains localDecl.name then
              some "StateDomain"
            else (Print.specificationTypeText unit ns declaration.generics
              localDecl.type.typeId).toOption
          let some text := text | continue
          let .ok type := Parser.runParserCategory (← getEnv) `verexType text | continue
          found := found.push (name, type)
  return found

/-- What the clause a key names needs (`ClauseNeeds`); a data invariant's
leaf, the binders of the data invariants. -/
def clauseNeeds (unit : ValidatedUnit) (namespaceIndex : Nat) (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody)
    (key : VerexIR.Proofs.Denote.ObligationKey) : ClauseNeeds :=
  if key.kind == "data invariant" then
    { reads := none, binders := dataInvariantBinders unit } else
  match clauseOf? unit namespaceIndex ns declaration key with
  | some (clauseNs, owner, clause) =>
      { reads := some (readsMapValue unit clauseNs clause), binders := clauseBinders clauseNs owner clause }
  | none => { reads := none }

/-- What an `on` step's obligations need, by the step's position: for an
`on` of keys, their clauses' needs joined (a map read where one reads one,
the binders of each); for `on _`, the function's clauses' needs. -/
inductive OnNeeds where
  | keys (needs : ClauseNeeds)
  | clauses (table : Array (VerexIR.Proofs.Denote.ObligationKey × ClauseNeeds))
  deriving Inhabited

private initialize onNeedsExtension : EnvExtension (Array ((String × Nat) × OnNeeds)) ←
  registerEnvExtension (pure #[])

/-- The types of the binders of the existentials a clause states under its
universal quantifiers and premises, in order: what a `witness` supplies. -/
partial def witnessTypes (ns : ValidatedNamespace) (typeText : VerexIR.TypeId → Except String String)
    (clause : VerexIR.ExprId) : Except String (Array String) := do
  let some expression := ns.expressions[clause.index]? | return #[]
  match expression.kind with
  | .quantifier .forall _ _ _ body => witnessTypes ns typeText body
  | .operation (.primitive .implies) _ #[_, body] _ => witnessTypes ns typeText body
  | .quantifier .exists .. => existentials clause
  | _ => return #[]
where
  existentials (id : VerexIR.ExprId) : Except String (Array String) := do
    let some { kind := .quantifier .exists binders _ _ body, .. } := ns.expressions[id.index]?
      | return #[]
    let types ← binders.mapM fun binder => do
      let some pattern := ns.patterns[binder.pattern.index]? | throw "a binder has no pattern"
      typeText pattern.typeId
    return types ++ (← existentials body)

/-- The type arguments of the `call`-th call of `callee` in a function's body,
in source order; with no number, of its one call. -/
def calleeTypeArguments? (unit : ValidatedUnit) (namespaceIndex : Nat) (ns : ValidatedNamespace)
    (declaration : VerexIR.FunctionDecl VerexIR.Validation.FunctionBody) (callee : String)
    (call : Option Nat) : Option (Array VerexIR.TypeId) := Id.run do
  let .structured root := declaration.body | return none
  let mut found : Array (Nat × Array VerexIR.TypeId) := #[]
  let mut pending := #[root]
  let mut visited : Std.HashSet Nat := {}
  while let some id := pending.back? do
    pending := pending.pop
    if visited.contains id.index then continue
    visited := visited.insert id.index
    let some expression := ns.expressions[id.index]? | continue
    if expression.kind matches .spec _ then continue
    if let .operation (.call (.function reference)) instantiations _ _ := expression.kind then
      if let some handle := VerexIR.SemanticOperations.resolveFunction? unit ⟨namespaceIndex⟩ reference then
        if let some calleeNs := unit.namespaces[handle.namespaceId.index]? then
          if let some decl := calleeNs.functions[handle.functionId.index]? then
            if (calleeNs.tables.names[decl.name.index]?).map (·.name) == some callee then
              let arguments := instantiations.filterMap fun
                | .typeArg typeUse => some typeUse.typeId
                | _ => none
              found := found.push ((Contract.locRange unit expression.loc).startByte, arguments)
    pending := pending ++ VerexIR.Validation.expressionChildren expression.kind
  let ordered := found.qsort (·.1 < ·.1)
  match call with
  | some number => return (ordered[number - 1]?).map (·.2)
  | none => return if ordered.size == 1 then (ordered[0]?).map (·.2) else none

/-- The name a step item's template holds its expression's place by. -/
private def placeholder : Name := `verex_step_expression

/-- The template with each placeholder replaced by its expression. -/
private def spliceAll (replacements : Array (Name × Syntax)) (stx : Syntax) : Syntax :=
  Id.run <| stx.replaceM fun stx => pure do
    guard (stx.isOfKind ``verexLocalExpr)
    (replacements.find? (·.1 == stx[0].getId)).map (·.2)

/-- The template with its placeholder replaced by `expression`. -/
private def splice (expression : Syntax) : Syntax → Syntax :=
  spliceAll #[(placeholder, expression)]

/-- The item a step's expression lowers in, named `«f__step_k»`: a
specification function whose body is the expression, or, for a lemma
formula, a lemma whose proof applies it; its parameters are the scope. -/
private def stepItem (function : String) (index : Nat) (binders : String)
    (scope : Array (String × String)) (step : StepExpression) : CommandElabM Syntax := do
  let binders := if binders.isEmpty then "" else " " ++ binders
  let stepBinders := step.binders.map fun (name, type) =>
    (name, (type.reprint.getD "Int").trimAscii.toString)
  let parameters := scope.filter (fun (name, _) => !stepBinders.any (·.1 == name)) ++ stepBinders
  let declared := ", ".intercalate (parameters.toList.map fun (name, type) => s!"«{name}» : {type}")
  let text := if step.apply then
      s!"spec lemma «{function}__step_{index}»{binders}({declared}) where\n  ensures true\n  \
        proof\n    apply {placeholder}"
    else
      let result := (step.type.map (" : " ++ ·)).getD ""
      s!"spec fun «{function}__step_{index}»{binders}({declared}){result} := {placeholder}"
  match Parser.runParserCategory (← getEnv) `verexItem text with
  | .ok item => return splice step.expression (atRangeOf step.expression item)
  | .error message => throwErrorAt step.expression m!"the step's scope does not parse: {message}"

/-- Lower the step expressions of a module's `verify` items, each in the
scope of its function, in one copy of the module (`compileNamespaceWith`,
not registered), and record them for the steps' tactics. A diagnostic of a
step expression is reported at it. -/
def lowerStepExpressions (related : Environment → ValidatedUnit → Array (Array String))
    (unit : ValidatedUnit) (moduleCommand : Syntax) (moduleFile : Option ParsedFile)
    (items : Array (Syntax × String × Syntax × String))
    (proofLemmas : Array (Syntax × String) := #[]) : CommandElabM Unit := do
  modifyEnv (specRolesExtension.setState · (specRoles unit))
  let mut synthesized := #[]
  let mut pending := #[]
  -- The position of each pending expression's item among `synthesized`.
  let mut pendingItems : Array Nat := #[]
  for (item, function, block, file) in items do
    let some (namespaceIndex, ns, _, declaration) := findFunction? unit function | continue
    let scope ← match functionScope unit ns declaration (readsResult block) with
      | .ok scope => pure (scope.map fun (name, type, _) => (name, type))
      | .error message => throwErrorAt item message
    -- The witnesses of an `on` block's obligation have the types of its
    -- existential's binders.
    let mut witnessed : Array (Nat × Array String) := #[]
    for step in blockSteps block do
      unless step.isOfKind ``stepOn do continue
      unless (blockSteps step[3]).any (·.isOfKind ``stepWitness) do continue
      let keys := step[1].getSepArgs
      -- A selector takes paths of the one clause: its existential is the
      -- same on each of them.
      unless keys.size == 1 do
        throwErrorAt step "an `on` block with a `witness` names one obligation"
      let key ← VerexIR.Proofs.Denote.keyPattern keys[0]!
      let some (clauseNs, owner, clause) := clauseOf? unit namespaceIndex ns declaration key
        | throwErrorAt keys[0]! "a `witness` supplies a clause's existential; this key names no clause"
      -- A callee's types at the call's type arguments, spelled in the
      -- function's scope.
      let typeText : VerexIR.TypeId → Except String String ← if key.kind == "requires" then
          let some callee := key.callee | throwErrorAt keys[0]! "a precondition key names its callee"
          let some arguments := calleeTypeArguments? unit namespaceIndex ns declaration callee key.call
            | throwErrorAt keys[0]! m!"`{callee}` is called more than once: the key names the call \
                (`at call j`)"
          let texts ← match arguments.mapM (Print.specificationTypeText unit ns
              declaration.signature.generics) with
            | .ok texts => pure texts
            | .error message => throwErrorAt keys[0]! message
          pure (Print.instantiatedTypeText unit clauseNs owner.signature.generics texts)
        else pure (Print.specificationTypeText unit clauseNs declaration.signature.generics)
      let types ← match witnessTypes clauseNs typeText clause with
        | .ok types => pure types
        | .error message => throwErrorAt keys[0]! message
      if let some position := step.getPos? then witnessed := witnessed.push (position.byteIdx, types)
    -- What each `on` step's obligations need (`OnNeeds`).
    for step in blockSteps block do
      let some position := step.getPos? | continue
      if step.isOfKind ``stepOn then
        let mut joined : ClauseNeeds := { reads := some false }
        for keySyntax in step[1].getSepArgs do
          let key ← VerexIR.Proofs.Denote.keyPattern keySyntax
          let needs := clauseNeeds unit namespaceIndex ns declaration key
          let reads := match joined.reads, needs.reads with
            | some true, _ => some true
            | _, some true => some true
            | some false, some false => some false
            | _, _ => none
          joined := { reads, binders := joined.binders ++ needs.binders.filter (!joined.binders.contains ·) }
        modifyEnv (onNeedsExtension.modifyState · (·.push ((file, position.byteIdx), .keys joined)))
      else if step.isOfKind ``stepOnDefault then
        let mut table := #[]
        for (_, name) in clauseNames unit ns declaration do
          let parts := name.splitOn "_"
          let some index := parts.getLast?.bind (·.toNat?) | continue
          let key : VerexIR.Proofs.Denote.ObligationKey :=
            { kind := "_".intercalate parts.dropLast, index := some index }
          table := table.push (key, clauseNeeds unit namespaceIndex ns declaration key)
        for (_, callee, _, _) in (match declaration.body with
            | .structured root => callSites unit ns namespaceIndex root
            | _ => #[]) do
          let some (_, calleeDeclaration) := calleeDeclaration? unit namespaceIndex ns declaration callee
            | continue
          for (_, index) in preconditionNumbers unit calleeDeclaration do
            let key : VerexIR.Proofs.Denote.ObligationKey :=
              { kind := "requires", index := some index, callee := some callee }
            table := table.push (key, clauseNeeds unit namespaceIndex ns declaration key)
        modifyEnv (onNeedsExtension.modifyState · (·.push ((file, position.byteIdx), .clauses table)))
    let witnesses (step : Syntax) : Array String :=
      ((step.getPos?).bind fun position => witnessed.find? (·.1 == position.byteIdx)).map (·.2)
        |>.getD #[]
    let binders ← match Print.genericBindersText unit declaration.signature.generics with
      | .ok text => pure text
      | .error message => throwErrorAt item message
    -- An `on data invariant` block reads the data invariants' binders.
    let invariantBinders ← dataInvariantBinderTypes unit
    let keyBinders (step : Syntax) : Array (String × Syntax) :=
      if step[1].getSepArgs.any (·.isOfKind ``VerexIR.Proofs.Denote.keyDataInvariant) then
        invariantBinders.filter fun (name, _) => !scope.any (·.1 == name)
      else #[]
    for step in blockExpressions block (witnesses := witnesses) (keyBinders := keyBinders) do
      let index := synthesized.size
      -- In the proof file's band: its positions are not the module's.
      synthesized := synthesized.push
        (shifted proofBand (← stepItem function index binders scope step))
      pending := pending.push (step, s!"{function}__step_{index}", file)
      pendingItems := pendingItems.push index
  -- A lemma item is a `spec lemma` of the copy, its clauses at their places
  -- in the proof file; its steps lower in the scope of its parameters.
  let mut lemmaDeclarations := #[]
  for (item, file) in proofLemmas do
    let name := item[1].getId.toString (escape := false)
    let generics := if item[2].getNumArgs > 0 then item[2][1].getSepArgs.map
      (·.getId.toString (escape := false)) else #[]
    let binders := " ".intercalate (generics.toList.map fun generic => "{" ++ generic ++ "}")
    let parameters := item[4].getSepArgs
    let scope ← parameters.mapM fun parameter => do
      let some identifier := parameter.find? (·.isIdent)
        | throwErrorAt parameter "a lemma's parameter is named"
      let some type := parameter[parameter.getNumArgs - 1].reprint
        | throwErrorAt parameter "a lemma's parameter has a type"
      pure (identifier.getId.toString (escape := false), type.trimAscii.toString)
    let clauses := item[6].getArgs.map (("requires", ·[1])) ++ item[7].getArgs.map (("ensures", ·[1]))
    let placeholderOf (position : Nat) : Name := Name.mkSimple s!"verex_lemma_clause_{position}"
    let measure := if item[8].getNumArgs > 0 then
      s!" decreases {placeholderOf clauses.size}" else ""
    let declared := ", ".intercalate (scope.toList.map fun (name, type) => s!"«{name}» : {type}")
    let lines := clauses.zipIdx.toList.map fun ((kind, _), position) =>
      s!"\n  {kind} {placeholderOf position}"
    let text := s!"spec lemma «{name}»{if binders.isEmpty then "" else " " ++ binders}({declared})\
      {measure} where" ++ String.join lines
    let template ← match Parser.runParserCategory (← getEnv) `verexItem text with
      | .ok template => pure template
      | .error message => throwErrorAt item m!"the lemma does not parse as a `spec lemma`: {message}"
    let replacements := clauses.zipIdx.map (fun ((_, expression), position) =>
      (placeholderOf position, expression)) ++
      (if item[8].getNumArgs > 0 then #[(placeholderOf clauses.size, item[8][1])] else #[])
    synthesized := synthesized.push (shifted proofBand (spliceAll replacements (atRangeOf item template)))
    lemmaDeclarations := lemmaDeclarations.push (name, item, file)
    for step in blockExpressions item[10] do
      let index := synthesized.size
      synthesized := synthesized.push (shifted proofBand (← stepItem name index binders scope step))
      pending := pending.push (step, s!"{name}__step_{index}", file)
      pendingItems := pendingItems.push index
  if synthesized.isEmpty then return
  -- The built-in lemmas an `apply` names: those of the module's maps and of
  -- the maps of other modules its expressions hold, its own first, then the
  -- arithmetic and vector lemmas; a name the module declares stays its own.
  let some own := unit.namespaces[0]? | return
  let declared := (own.lemmas.map (·.name) ++ own.specFunctions.map (·.name) ++
    own.functions.map (·.name)).filterMap fun name => (unit.tables.names[name.index]?).map (·.name)
  let nominalOf? (type : VerexIR.TypeId) : Option VerexIR.NameId :=
    match unit.tables.types[type.index]? with
    | some (.nominal name _) => some name
    | some (.reference reference) => match unit.tables.types[reference.referent.index]? with
      | some (.nominal name _) => some name
      | _ => none
    | _ => none
  let held (owner : VerexIR.NameId) : Bool :=
    own.expressions.any fun expression => nominalOf? expression.typeId == some owner
  let mut moduleLemmas : Array (String × Name × String) := #[]
  for ns in unit.namespaces do
    let found ← match namespaceMapLemmas unit ns own
        (if ns.identity == own.identity then fun _ => true else held) with
      | .ok found => pure found
      | .error message => throwErrorAt moduleCommand message
    for entry@(name, _, _) in found do
      unless declared.contains name || moduleLemmas.any (·.1 == name) do
        moduleLemmas := moduleLemmas.push entry
  for lemma in builtinLemmas do
    unless declared.contains lemma.name || moduleLemmas.any (·.1 == lemma.name) do
      moduleLemmas := moduleLemmas.push (lemma.name, lemma.theorem_, builtinLemmaItem lemma)
  let environment ← getEnv
  -- The lemmas laid out one after another in their band.
  let mut lemmaItems := #[]
  let mut offset := lemmaBand
  for (_, _, text) in moduleLemmas do
    match Parser.runParserCategory environment `verexItem text with
    | .ok item => lemmaItems := lemmaItems.push (shifted offset item)
    | .error message => throwErrorAt moduleCommand m!"a map lemma does not parse: {message}"
    offset := offset + text.utf8ByteSize + 1
  let proofFiles := (pending.map (·.2.2) ++ lemmaDeclarations.map (·.2.2)).foldl (init := #[])
    fun found file => if found.contains file then found else found.push file
  let some proofFile := proofFiles[0]? | return
  unless proofFiles.size == 1 do
    throwErrorAt moduleCommand "the step expressions of a module come from one proof file"
  -- Each source fits its band.
  let fits (stx : Syntax) : Bool := (stx.getTailPos? true).all (·.byteIdx < sourceBand)
  unless fits moduleCommand && pending.all (fits ·.1.expression) &&
      lemmaDeclarations.all (fits ·.2.1) && offset < lemmaBand + sourceBand do
    throwErrorAt moduleCommand m!"VEREX-SOURCE-RANGE: a source exceeds {sourceBand} bytes"
  let last := moduleCommand.getNumArgs - 1
  -- The module keeps its own range: the items spliced into it are of other
  -- sources.
  let range := match moduleCommand.getPos? true, moduleCommand.getTailPos? true with
    | some start, some stop => SourceInfo.synthetic start stop (canonical := true)
    | _, _ => SourceInfo.none
  let extendedWith (items : Array Syntax) :=
    (moduleCommand.setArg last
      (mkNullNode (moduleCommand[last].getArgs ++ lemmaItems ++ items))).setInfo range
  let diagnostics ← IO.mkRef (#[] : Array CompileDiagnostic)
  let compile (stx : Syntax) (source : CompilationUnit) : CommandElabM ValidatedUnit :=
    match compileUnit { source with sources := #[proofFile, "<map lemmas>"] } with
    | .ok validated => pure validated
    | .error found => do diagnostics.set found; throwErrorAt stx "the step expressions do not lower"
  let attempt (items : Array Syntax) := do
    try some <$> inParsedFile moduleFile (compileNamespaceWith related (extendedWith items) compile)
    catch error =>
      if (← diagnostics.get).isEmpty then logException error
      pure none
  -- A diagnostic in the proof file's band is reported at the step expression
  -- or the lemma item whose range holds it.
  let holds (stx : Syntax) : Option (Nat × Nat) → Bool
    | some (start, stop) => start / sourceBand == 1 &&
        (stx.getPos? true).any (·.byteIdx + proofBand ≤ start) &&
        (stx.getTailPos? true).any (stop ≤ ·.byteIdx + proofBand)
    | none => false
  let mut lowered ← attempt synthesized
  -- An expression that does not lower reports itself alone: the others
  -- lower without it. Lowering stops at a first error, so each attempt may
  -- name another.
  let mut reported : Array CompileDiagnostic := #[]
  let mut failing : Array Nat := #[]
  for _ in [0:pending.size] do
    if lowered.isSome then break
    let found ← diagnostics.get
    reported := reported ++ found
    let more := (pending.zipIdx.filter fun ((step, _, _), position) =>
      !failing.contains position && found.any fun (range, severity, _) =>
        (match severity with | .error => true | _ => false) && holds step.expression range).map (·.2)
    if more.isEmpty then break
    failing := failing ++ more
    let dropped := failing.map (pendingItems[·]!)
    let kept := (synthesized.zipIdx.filter fun (_, position) => !dropped.contains position).map (·.1)
    diagnostics.set #[]
    lowered ← attempt kept
  if lowered.isSome || !failing.isEmpty then
    diagnostics.set (reported ++ (← diagnostics.get))
  let lowerable := (pending.zipIdx.filter fun (_, position) => !failing.contains position).map (·.1)
  for (range, severity, message) in ← diagnostics.get do
    let at? := ((pending.find? fun (step, _, _) => holds step.expression range).map
      (·.1.expression)) <|> ((lemmaDeclarations.find? fun (_, item, _) => holds item range).map (·.2.1))
    logAt (at?.getD moduleCommand) message severity
  let some (copy, _, _) := lowered | return
  let some ns := copy.namespaces[0]? | return
  let nameOf (name : VerexIR.NameId) := (copy.tables.names[name.index]?).map (·.name)
  let mut recorded := #[]
  -- What each lemma item's steps apply: its recursion group.
  let lemmaReference (name : String) : Option VerexIR.QualifiedRef :=
    (ns.lemmas.find? (nameOf ·.name == some name)).map fun lemma => { namespaceId := ⟨0⟩, name := lemma.name }
  let mut stepApplications : StepApplications := #[]
  for ((step : StepExpression), (name : String), (file : String)) in lowerable do
    let some key := stepKey file step.expression | continue
    if step.apply then
      let some lemma := ns.lemmas.find? (nameOf ·.name == some name) | continue
      let some condition := lemma.proof[0]? | continue
      -- The lemma it applies: a map lemma is proved where it is applied.
      let applied : Option VerexIR.QualifiedRef :=
        (ns.expressions[condition.expression.index]?).bind fun expression =>
          match expression.kind with
          | .operation (.specification (.lemma reference _)) .. => some reference
          | _ => none
      let builtin := applied.bind fun reference => do
        let appliedName ← nameOf reference.name
        (moduleLemmas.find? (·.1 == appliedName)).map (·.2.1)
      -- A lemma of the module is applied by its definitions, which its own
      -- verification declared; a built-in lemma is expanded where it is
      -- applied.
      if let (some reference, none) := (applied, builtin) then
        try discard <| liftTermElabM (Contract.ensureLemmaDefinitions copy reference)
        catch error =>
          logErrorAt step.expression m!"the lemma's statement does not translate: \
            {error.toMessageData}"
          continue
        -- A step of a lemma item applying a lemma.
        if let some (owner, _, _) := lemmaDeclarations.find? fun (owner, _, _) =>
            name.startsWith s!"{owner}__step_" then
          if let some ownerReference := lemmaReference owner then
            stepApplications := match stepApplications.findIdx? (·.1 == ownerReference) with
              | some position => stepApplications.modify position fun (o, applied) =>
                  (o, if applied.contains reference then applied else applied.push reference)
              | none => stepApplications.push (ownerReference, #[reference])
      recorded := recorded.push (key,
        { unit := copy, expression := condition.expression, locals := lemma.locals, builtin })
    else
      let some function := ns.specFunctions.find? (nameOf ·.name == some name) | continue
      let some body := function.body | continue
      recorded := recorded.push (key, { unit := copy, expression := body, locals := function.locals })
  modifyEnv (loweredStepsExtension.modifyState · (· ++ recorded))
  -- The lemma items, proved by their blocks, before the `verify` items that
  -- apply them.
  let roots := lemmaDeclarations.filterMap fun (name, _, _) => lemmaReference name
  let blocks := lemmaDeclarations.filterMap fun (name, item, _) =>
    (lemmaReference name).map (·, item[10])
  verifyLemmas copy #[] roots (lemmaDeclarations.map fun (name, item, _) => (name, item[1]))
    moduleCommand blocks stepApplications

/-! ## Steps at a leaf -/

open Meta Tactic VerexIR.Proofs.Denote

/-- The carriers the leaf's native values are at: those of a local of a
native type, else the target's family's, where the leaf holds its values
typed only. -/
private def leafCarriers? : MetaM (Option Lean.Expr) := do
  let mut family := none
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let type ← instantiateMVars decl.type
    if type.isAppOfArity ``NTy.carrier 2 then return some (type.getArg! 0)
    if family.isNone && type.isAppOfArity ``Skolems 2 then
      family := some (mkApp3 (mkConst ``Skolems.toCarriers) (type.getArg! 0) (type.getArg! 1)
        decl.toExpr)
  return family

/-- The last local of a name that is no hypothesis: where a leaf holds the
name's value. -/
private def namedLocal? (name : String) : MetaM (Option LocalDecl) := do
  (← getLCtx).foldlM (init := none) fun found decl => do
    if decl.userName != Name.mkSimple name || decl.isImplementationDetail then return found
    return if ← isProp decl.type then found else some decl

/-- The native types of a row. -/
private def rowTypes : NRow → List NTy
  | .nil => []
  | .cons τ rest => τ :: rowTypes rest

/-- The value a leaf holds for the name `x` as a native value of `τ`, built
from the parts it holds under the names the leaf gives them, where the name
has no scope entry, a value the leaf holds only in parts: the local `x`
where its type is `τ`'s carrier; a named map's carrier, the map written back
(`x_read`); a structure's or a tuple's, the row of its components' values,
each under the name the leaf gives the component (`componentNames`: `x_f`
of a field `f`, `x_k` of a tuple's `k`-th, an intrinsic map owner's map `x`
itself). Built from the leaf's locals and native constructors only. -/
private partial def partsValue (unit? : Option ValidatedUnit) (carriers : Lean.Expr) (x : String)
    (τ : NTy) : MetaM (Option Lean.Expr) := do
  let expected := Contract.carrierType carriers (← Contract.quoteNTy τ)
  let named ← namedLocal? x
  if let some decl := named then
    if ← isDefEq decl.type expected then return some decl.toExpr
  match τ with
  | .table .. =>
      let some decl := named | return none
      for candidate in ← getLCtx do
        let some (_, lhs, rhs) := (← instantiateMVars candidate.type).eq? | continue
        if rhs == decl.toExpr && lhs.isAppOfArity ``VerexIR.SpecMap.ofEntries 7 &&
            (lhs.getArg! 6).isAppOfArity ``VerexIR.SpecMap.writeBack 6 then
          return some (lhs.getArg! 6)
      return none
  | .struct source _ fields =>
      let some unit := unit? | return none
      let types := rowTypes fields
      row (componentNames unit source x types.length) types
  | .tuple elements =>
      let types := rowTypes elements
      row ((List.range types.length).map (s!"{x}_{·}")) types
  | _ => return none
where
  /-- The row of the components' values, ending in the unit. -/
  row (names : List String) (types : List NTy) : MetaM (Option Lean.Expr) := do
    let mut values := #[]
    for name in names, type in types do
      let some value ← partsValue unit? carriers name type | return none
      values := values.push value
    let mut built := mkConst ``PUnit.unit [1]
    for value in values.reverse do built ← mkAppM ``Prod.mk #[value, built]
    return some built

/-- Hide the scope entries and the facts' marks of a goal: none is a fact a
decider reads. A substitution reintroduces the entries it rewrites,
visible. -/
def hideEntries (goal : MVarId) : MetaM MVarId := goal.withContext do
    for decl in ← getLCtx do
      if !decl.isImplementationDetail &&
          (decl.type.isAppOfArity ``ScopeEntry 4 || decl.type.isAppOfArity ``BlockFact 1) then
        goal.setFVarKind decl.fvarId .implDetail
    return goal

/-- A value with the projections of pairs it applies reduced: an entry of a
reference holds its current value, the first of the pair a case split
rewrites the reference to. -/
private partial def pairProjections (value : Lean.Expr) : Lean.Expr :=
  match value.getAppFn, value.getAppArgs with
  | .const ``Prod.fst _, #[_, _, pair] =>
      let pair := pairProjections pair
      if pair.isAppOfArity ``Prod.mk 4 then pair.getArg! 2 else mkApp value.appFn! pair
  | .const ``Prod.snd _, #[_, _, pair] =>
      let pair := pairProjections pair
      if pair.isAppOfArity ``Prod.mk 4 then pair.getArg! 3 else mkApp value.appFn! pair
  | _, _ => value

/-- The values of the names in scope at a goal, each its latest binding's
(`ScopeEntry`); `old_x`, what `old(x)` reads, is the earliest binding of
`x`, its value at the function's entry, where no binding is named `old_x`. -/
private def scopeEntries : MetaM (Array (String × Lean.Expr)) := do
  let bindings ← (← getLCtx).foldlM (init := (#[] : Array (String × Nat × Lean.Expr × Nat × Lean.Expr)))
    fun found decl => do
      let some (name, index, value) := scopeEntry? (← instantiateMVars decl.type) | return found
      match found.findIdx? (·.1 == name) with
      | some position =>
          let (_, latest, latestValue, earliest, earliestValue) := found[position]!
          let (latest, latestValue) := if latest < index then (index, value) else (latest, latestValue)
          let (earliest, earliestValue) :=
            if index < earliest then (index, value) else (earliest, earliestValue)
          return found.set! position (name, latest, latestValue, earliest, earliestValue)
      | none => return found.push (name, index, value, index, value)
  let latest := bindings.map fun (name, _, value, _, _) => (name, value)
  let entryValues := bindings.filterMap fun (name, _, _, _, value) =>
    let oldName := "old_" ++ name
    if bindings.any (·.1 == oldName) then none else some (oldName, value)
  return latest ++ entryValues

/-- The translation context of a step's module at a goal's frame, no name
bound. -/
def stepBaseContext (step : LoweredStep) (reference : Syntax) : MetaM Contract.Context := do
  let some ns := step.unit.namespaces[0]? | throwErrorAt reference "the step's module is missing"
  -- A lemma over integers alone holds no native value: its steps read none.
  let some carriers ← leafCarriers?
    | return { unit := step.unit, namespaceId := ⟨0⟩, ns, locals := #[], results := #[] }
  let some (_, _, frame) := Contract.carriersFrame? carriers
    | throwErrorAt reference "the obligation's carriers have no frame"
  let executable ← (← getLCtx).foldlM (init := none) fun found decl => do
    if found.isSome then return found
    return if (← instantiateMVars decl.type).isAppOfArity ``VerexIR.Validation.ExecutableUnit 1 then
      some decl.toExpr else none
  return {
    unit := step.unit, namespaceId := ⟨0⟩, ns, locals := #[], results := #[],
    types := some (← Contract.frameTypes frame), executable }

/-- The translation context of a step's expression at a goal: its scope's
names bound to their values, at the leaf's frame. -/
def stepContext (step : LoweredStep) (reference : Syntax) : MetaM Contract.Context := do
  let base ← stepBaseContext step reference
  let entries ← scopeEntries
  for index in referencedLocals base.ns step.expression do
    let some localDecl := step.locals[index]? | continue
    if entries.any (·.1 == localDecl.name) then continue
    let some carriers ← leafCarriers? | continue
    if (← partsValue (some step.unit) carriers localDecl.name
        (← base.nativeType localDecl.type.typeId)).isNone then
      throwErrorAt reference m!"`{localDecl.name}` is not in scope at this obligation; in scope: \
        {", ".intercalate ((entries.map (·.1)).filter (!·.contains '$')).toList}"
  let mut locals := #[]
  let mut localTypes := #[]
  for localDecl in step.locals do
    let some type := base.valueTypeOf? localDecl.type.typeId
      | throwErrorAt reference m!"the type of `{localDecl.name}` is unknown"
    localTypes := localTypes.push type
    -- An entry holds a leaf's native value, or a binder's value of its
    -- specification type, an integer's an `Int`; a value the leaf holds only
    -- in parts is built from them.
    let native? ← match entries.find? (·.1 == localDecl.name) with
      | some (_, value) => pure (some (pairProjections value))
      | none => do
          let some carriers ← leafCarriers? | pure none
          partsValue (some step.unit) carriers localDecl.name (← base.nativeType localDecl.type.typeId)
    let value? ← native?.mapM fun value => do
      if ← isDefEq (← inferType value) (mkConst ``Int) then pure value
      else (Contract.domainOf type).binderOfNative value
    locals := locals.push value?
  -- The memory the step reads where it stands, and the entry memory `old`
  -- reads (`stateScopeName`).
  let state := (entries.find? (·.1 == stateScopeName)).map (·.2)
  let oldState := (entries.find? (·.1 == "old_" ++ stateScopeName)).map (·.2) <|> state
  -- A name in scope holding a memory, a data invariant's binder over the
  -- state domain, is the state label of its name (`S |~ e`).
  let mut stateLabels : List (Nat × Contract.LabelState) := []
  for localDecl in step.locals, value? in locals do
    let some value := value? | continue
    unless (← whnfR (← inferType value)).isAppOf ``VerexIR.Proofs.Denote.Memory do continue
    let some label := step.unit.tables.names.findIdx? (·.name == localDecl.name) | continue
    let labelState : Contract.LabelState := { memory := value, locals }
    stateLabels := (label, labelState) :: stateLabels
  return { base with
    locals, localTypes, oldLocals := locals, localNames := step.locals.map (·.name),
    state, oldState, stateLabels }

/-- Whether a proposition compares integers. -/
private def comparesIntegers (proposition : Lean.Expr) : MetaM Bool := do
  let proposition := proposition.consumeMData
  let operands? := match_expr proposition with
    | LT.lt type _ _ _ => some type
    | LE.le type _ _ _ => some type
    | Eq type _ _ => some type
    | _ => none
  match operands? with
  | some type => isDefEq type (mkConst ``Int)
  | none => return false

/-- Close a goal whose context states one Boolean both `true` and `false`. -/
elab "verex_bool_conflict" : tactic => focusGoal do
  let goal ← getMainGoal
  goal.withContext do
    let mut trues : Array (Lean.Expr × Lean.Expr) := #[]
    let mut falses : Array (Lean.Expr × Lean.Expr) := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let some (type, value, literal) := (← instantiateMVars decl.type).eq? | continue
      unless type.isConstOf ``Bool do continue
      if literal.isConstOf ``Bool.true then trues := trues.push (value, decl.toExpr)
      else if literal.isConstOf ``Bool.false then falses := falses.push (value, decl.toExpr)
    for (value, holds) in trues do
      for (other, fails) in falses do
        if ← isDefEq value other then
          let contradiction ← mkAppOptM ``Bool.eq_false_iff #[some other]
          let notTrue ← mkAppM ``Iff.mp #[contradiction, fails]
          goal.assign (← mkAbsurd (← goal.getType) holds notTrue)
          replaceMainGoal []
          return
    throwError "no Boolean is stated both true and false"

/-- Close a goal whose context holds `False`, or a proposition beside its
negation, by their spelling: one pass over the context. -/
elab "verex_negation_conflict" : tactic => focusGoal do
  let goal ← getMainGoal
  goal.withContext do
    let mut stated : Std.HashMap Lean.Expr Lean.Expr := {}
    let mut negated : Array (Lean.Expr × Lean.Expr) := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let type ← instantiateMVars decl.type
      if type.isConstOf ``False then
        goal.assign (← mkFalseElim (← goal.getType) decl.toExpr)
        replaceMainGoal []
        return
      match type.not? with
      | some inner => negated := negated.push (inner, decl.toExpr)
      | none => stated := stated.insert type decl.toExpr
    for (inner, refutes) in negated do
      if let some holds := stated[inner]? then
        goal.assign (← mkAbsurd (← goal.getType) holds refutes)
        replaceMainGoal []
        return
    throwError "no hypothesis contradicts another"

/-- `decide` at a step or an obligation of a block, on its normal form: the
goal is a hypothesis or reflexive by its spelling, contradicts the context
(a proposition and its negation, a Boolean both true and false), is an
instance of a quantified hypothesis (its premises by `omega`), or is linear
arithmetic; no search and no budget. -/
syntax (name := blockDecideTactic) "verex_block_decide" : tactic

/-- The vector and element of a membership stated by its positions,
`∃ i, 0 ≤ i ∧ i < xs.size ∧ xs[i.toNat]? = some t`. -/
private def positionalMembership? (e : Lean.Expr) : MetaM (Option (Lean.Expr × Lean.Expr)) := do
  let_expr Exists _ body := e | return none
  let .lam _ _ matrix _ := body | return none
  let_expr And _ rest := matrix | return none
  let_expr And _ read := rest | return none
  let_expr Eq _ lookup found := read | return none
  let_expr Option.some _ t := found | return none
  unless lookup.isAppOfArity ``GetElem?.getElem? 7 do return none
  let xs := lookup.getArg! 5
  unless (← inferType xs).isAppOf ``Array do return none
  if xs.hasLooseBVars || t.hasLooseBVars then return none
  let reading ← mkAppM ``VerexIR.Proofs.Denote.mem_iff_exists_int_index #[xs, t]
  let some (_, positional) := (← inferType reading).iff? | return none
  return if ← withReducible (isDefEq positional e) then some (xs, t) else none

/-- Close a goal its context states, by spelling (`doc/plan/proof-language.md`,
"Closers": the facts decide what they state): a hypothesis of the goal's
statement, a reflexive equation, `True`, or a hypothesis `False` or `¬True`,
as a case a split refutes leaves. One pass over the context. -/
private partial def statedProof? (target : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let target := (← instantiateMVars target).consumeMData
  if target.isConstOf ``True then return some (mkConst ``True.intro)
  if let some (_, lhs, rhs) := target.eq? then
    if lhs == rhs then return some (← mkEqRefl lhs)
  -- A disjunct stated.
  if target.isAppOfArity ``Or 2 then
    let (left, right) := (target.getArg! 0, target.getArg! 1)
    if let some proof ← statedProof? left then
      return some (mkApp3 (mkConst ``Or.inl) left right proof)
    if let some proof ← statedProof? right then
      return some (mkApp3 (mkConst ``Or.inr) left right proof)
  -- The same statement, up to its instances' and annotations' spelling.
  let same (type wanted : Lean.Expr) : MetaM Bool := do
    if type == wanted then return true
    unless type.getAppFn == wanted.getAppFn && type.getAppNumArgs == wanted.getAppNumArgs do
      return false
    withReducible (isDefEq type wanted)
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let type := (← instantiateMVars decl.type).consumeMData
    if ← same type target then return some decl.toExpr
    -- An equation stated the other way round.
    if let some (_, lhs, rhs) := target.eq? then
      if let some (_, lhs', rhs') := type.eq? then
        if lhs' == rhs && rhs' == lhs then return some (← mkEqSymm decl.toExpr)
    -- A disequation stated the other way round.
    if target.isAppOfArity ``Not 1 && type.isAppOfArity ``Not 1 then
      if let some (_, lhs, rhs) := target.appArg!.eq? then
        if let some (_, lhs', rhs') := type.appArg!.eq? then
          if lhs' == rhs && rhs' == lhs then return some (← mkAppM ``Ne.symm #[decl.toExpr])
    if type.isConstOf ``False then return some (← mkFalseElim target decl.toExpr)
    if type.isAppOfArity ``Not 1 && type.appArg!.isConstOf ``True then
      return some (← mkFalseElim target (mkApp decl.toExpr (mkConst ``True.intro)))
  -- A membership, an atom in the block's normal form, stated by its
  -- positions, as the leaf's normalization reads it, and the converse.
  if target.isAppOfArity ``Membership.mem 5 && (target.getArg! 1).isAppOf ``Array then
    let reading ← mkAppM ``VerexIR.Proofs.Denote.mem_iff_exists_int_index
      #[target.getArg! 3, target.getArg! 4]
    let some (_, positional) := (← inferType reading).iff? | return none
    if let some proof ← statedProof? positional then
      return some (← mkAppM ``Iff.mpr #[reading, proof])
    -- Or at a position a local names, each part stated, as a split left it.
    let_expr Exists _ body := positional | return none
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      unless (← instantiateMVars decl.type).isConstOf ``Int do continue
      let parts := body.beta #[decl.toExpr]
      let some (low, rest) := parts.and? | continue
      let some (high, read) := rest.and? | continue
      let some lowProof ← statedProof? low | continue
      let some highProof ← statedProof? high | continue
      let some readProof ← statedProof? read | continue
      let proof := mkApp4 (mkConst ``And.intro) low rest lowProof
        (mkApp4 (mkConst ``And.intro) high read highProof readProof)
      let witness ← mkAppOptM ``Exists.intro #[none, body, decl.toExpr, proof]
      return some (← mkAppM ``Iff.mpr #[reading, witness])
  else if target.isAppOfArity ``Not 1 && target.appArg!.isAppOfArity ``Membership.mem 5 &&
      (target.appArg!.getArg! 1).isAppOf ``Array then
    let reading ← mkAppM ``VerexIR.Proofs.Denote.forall_position_some_ne_iff
      #[target.appArg!.getArg! 3, target.appArg!.getArg! 4]
    let some (positional, _) := (← inferType reading).iff? | return none
    if let some proof ← statedProof? positional then
      return some (← mkAppM ``Iff.mp #[reading, proof])
  else if let some (xs, t) ← positionalMembership? target then
    let atom ← mkAppM ``Membership.mem #[xs, t]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      if ← withReducible (isDefEq (← instantiateMVars decl.type) atom) then
        let reading ← mkAppM ``VerexIR.Proofs.Denote.mem_iff_exists_int_index #[xs, t]
        let proof ← mkAppM ``Iff.mp #[reading, decl.toExpr]
        return some (← mkExpectedTypeHint proof target)
  return none

/-- Close a goal its context states (`statedProof?`). -/
private def closeByStatement (goal : MVarId) : MetaM Bool := goal.withContext do
  let some proof ← statedProof? (← goal.getType) | return false
  goal.assign proof
  return true

def blockDecide : TacticM (TSyntax `tactic) :=
  `(tactic| verex_block_decide)

/-- `congr` (`closerCongr`): the goal in the block's normal form, a
conjunction part by part, each by congruence closure and linear integer
arithmetic alone (`lia` with no case split: no instantiation, no ring, no
extensionality). -/
syntax (name := blockCongrTactic) "verex_block_congr" : tactic

/-- `ring` (`closerRing`): the goal in the block's normal form, a conjunction
part by part, each a polynomial identity by the `grind` ring module alone,
the facts in scope hidden: no case split, no instantiation, bounded by
`ringSteps` and `ringMaxDegree`. -/
syntax (name := blockRingTactic) "verex_block_ring" : tactic

/-- The tactic a closer stands for. -/
def closerTactic (closer : Syntax) : TacticM (TSyntax `tactic) := do
  if closer.isOfKind ``closerDecide then blockDecide
  else if closer.isOfKind ``closerArith then `(tactic| verex_omega_unsplit)
  else if closer.isOfKind ``closerEval then `(tactic| verex_typed_close)
  else if closer.isOfKind ``closerSolve then `(tactic| verex_solve)
  else if closer.isOfKind ``closerCongr then `(tactic| verex_block_congr)
  else if closer.isOfKind ``closerRing then `(tactic| verex_block_ring)
  else if closer.isOfKind ``closerLean then
    let script : TSyntax ``Lean.Parser.Tactic.tacticSeq := ⟨closer[2]⟩
    `(tactic| ($script))
  else throwErrorAt closer "this closer is not supported yet"

/-- A proof that a hypothesis's statement refutes `condition`, read
syntactically: it is `¬condition`, or the canonical negation of an integer
comparison (`b ≤ a` of `a < b`, `b < a` of `a ≤ b`). -/
private def refutation? (condition statement proof : Lean.Expr) : MetaM (Option Lean.Expr) := do
  if statement.not? == some condition then return some proof
  -- `b ≤ a` refutes `a < b`, `b < a` refutes `a ≤ b`, over the integers.
  let flipped (statement : Lean.Expr) : Option (Bool × Lean.Expr × Lean.Expr) :=
    match_expr statement with
    | LE.le type _ x y => if type.isConstOf ``Int then some (true, x, y) else none
    | LT.lt type _ x y => if type.isConstOf ``Int then some (false, x, y) else none
    | _ => none
  let some (conditionLe, a, b) := flipped condition | return none
  let some (statementLe, b', a') := flipped statement | return none
  unless conditionLe != statementLe && a == a' && b == b' do return none
  let negation := if conditionLe then ``Int.not_le else ``Int.not_lt
  return some (← mkAppM ``Iff.mpr #[← mkAppOptM negation #[a, b], proof])

/-- A proof of `condition` or of its negation from the hypotheses that state
it, read syntactically (`refutation?`); a conjunction holds where each
conjunct is stated and fails where one is refuted. -/
private partial def statedDecision? (condition : Lean.Expr) : MetaM (Option (Bool × Lean.Expr)) := do
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let statement ← instantiateMVars decl.type
    if statement == condition then return some (true, decl.toExpr)
    if let some refuted ← refutation? condition statement decl.toExpr then
      return some (false, refuted)
  let_expr And left right := condition | return none
  match ← statedDecision? left, ← statedDecision? right with
  | some (true, holds), some (true, holdsToo) =>
      return some (true, ← mkAppM ``And.intro #[holds, holdsToo])
  | some (false, refuted), _ => return some (false, ← mkAppM ``not_and_of_not_left #[right, refuted])
  | _, some (false, refuted) => return some (false, ← mkAppM ``not_and_of_not_right #[left, refuted])
  | _, _ => return none

/-- A conditional whose condition, or its negation, the hypotheses state,
reduced to its branch: the facts decide the conditionals they state, by
their spelling, without arithmetic. -/
simproc_decl reduceStatedIte (ite _ _ _) := fun e => do
  let_expr ite α condition decision whenTrue whenFalse := e | return .continue
  -- The conditional's own arguments, at its universe: a branch's type is
  -- the conditional's, also where it is spelled unfolded.
  let .const _ levels := e.getAppFn | return .continue
  match ← statedDecision? condition with
  | some (true, holds) =>
      let reduced := mkAppN (mkConst ``if_pos levels)
        #[condition, decision, holds, α, whenTrue, whenFalse]
      return .done { expr := whenTrue, proof? := some reduced }
  | some (false, refuted) =>
      let reduced := mkAppN (mkConst ``if_neg levels)
        #[condition, decision, refuted, α, whenTrue, whenFalse]
      return .done { expr := whenFalse, proof? := some reduced }
  | none => return .continue

/-- The name of the fact a block's step gives (`fact_k`, `split_k`, a nested
block's `fact_k_j`); a mark beside it (`BlockFact`), not its name, makes it a
fact of the block. -/
def blockFactName (kind : String) (index : String) (position : Option Nat := none) : Name :=
  Name.mkSimple s!"{kind}_{index}{(position.map (s!"_{·}")).getD ""}"

/-- The facts of the block in a context: the hypotheses the marks name; a
mark whose hypothesis is gone names nothing. -/
private def markedFacts : MetaM (Array FVarId) := do
  let lctx ← getLCtx
  lctx.foldlM (init := #[]) fun found decl => do
    unless decl.isImplementationDetail && decl.type.isAppOfArity ``BlockFact 1 do return found
    let .lit (.strVal name) := decl.type.appArg! | return found
    match lctx.findFromUserName? (Name.mkSimple name) with
    | some fact => return if fact.isImplementationDetail then found else found.push fact.fvarId
    | none => return found

/-- Mark a goal's hypothesis as a fact of the block (`BlockFact`). -/
private def markFact (goal : MVarId) (fact : FVarId) : MetaM MVarId := do
  let name ← goal.withContext do return mkStrLit (← fact.getDecl).userName.toString
  let (mark, goal) ← (← goal.assert `mark (mkApp (mkConst ``BlockFact) name)
    (mkApp (mkConst ``BlockFact.intro) name)).intro1P
  goal.setFVarKind mark .implDetail
  return goal

/-- The block's normal form of a fact or goal (`doc/plan/proof-language.md`,
"Closers"): a named map's carrier read as the map (`x_read`), then, with the
denotation's normal form, the leaf's definitions of named values folded to
their names (`m'_eq` right to left) and the block's earlier facts as the
rewrites they state, a Boolean fact deciding the conditionals it tests. A
fact of a generic lemma's instance is read in the leaf's terms by the
transport normal form first. The goal after, none where it is closed. -/
private structure BlockRules where
  /-- The named maps' bridges (`x_read`). -/
  reads : Array FVarId
  /-- The definitions of named values, read right to left. -/
  definitions : Array FVarId
  /-- The definitions that state the name on the right, read left to
  right. -/
  definitionsForward : Array FVarId := #[]
  /-- The block's facts. -/
  facts : Array FVarId
  /-- The leaf's Boolean facts (`b = true`, `b = false`), which decide the
  conditionals they test. -/
  booleans : Array FVarId := #[]
  /-- The leaf's equations of two names, each read toward the name declared
  first, so that rewriting by several ends: one name per value. -/
  aliases : Array FVarId := #[]
  aliasesReversed : Array FVarId := #[]
  /-- The block's facts read right to left (`factRule`). -/
  factsReversed : Array FVarId := #[]

/-- A local's name in the normal form: the local, or an integer local's
value. -/
private def localName? (e : Lean.Expr) : Option FVarId := match e with
  | .fvar x => some x
  | _ => if e.isAppOfArity ``VerexIR.SpecInt.val 3 then
      match e.appArg! with | .fvar x => some x | _ => none
    else none

/-- How an equation's side ranks for its orientation: a value (a literal),
then a name, then any other term. A fact rewrites its higher side to its
lower one: a term folds into the name it equals, a name evaluates to its
value. -/
private def sideRank (e : Lean.Expr) : Nat :=
  if (localName? e).isSome then 1
  else if e.int?.isSome || e.nat?.isSome || e.isConstOf ``Bool.true || e.isConstOf ``Bool.false then 0
  else 2

/-- How a fact of the block rewrites: left to right, right to left, or not
at all. -/
private inductive FactRule where
  | forward | reversed | none
  deriving BEq

/-- The rewriting of a fact (`doc/plan/proof-language.md`, "Closers"): an
equation from its higher side to its lower (`sideRank`), of two names
toward the one declared first; no rewriting for a name equated to a term
holding it (`a = C[a]`), whose rewriting would not end. -/
private def factRule (statement : Lean.Expr) : MetaM FactRule := do
  let some (_, lhs, rhs) := statement.eq? | return .forward
  let holds (side other : Lean.Expr) : Bool :=
    match localName? side with | some x => other.containsFVar x | none => false
  if holds lhs rhs || holds rhs lhs then return .none
  let (left, right) := (sideRank lhs, sideRank rhs)
  if left < right then return .reversed
  if left > right then return .forward
  match localName? lhs, localName? rhs with
  | some x, some y =>
      if x == y then return .none
      return if (← x.getDecl).index < (← y.getDecl).index then .reversed else .forward
  | _, _ => return .forward

/-- The local a quantified equation names, `∀ d, lhs d = x`, where `x` is a
local the left side does not hold: an invocation's result named by its
value at every default (`ResultOf … default = r`). -/
private def resultDefinition? (decl : LocalDecl) : MetaM (Option FVarId) := do
  let type ← instantiateMVars decl.type
  unless type.isForall do return none
  forallTelescopeReducing type fun _ body => do
    let some (_, lhs, .fvar x) := body.eq? | return none
    unless lhs.isAppOf ``VerexIR.Proofs.ResultOf do return none
    if lhs.containsFVar x then return none
    return some x

/-- Whether `lhs = rhs` states a memory local's entry as an option literal
(`m resource key = some v` or `= none`), as a path's split of a storage read
does. -/
private def memoryCase? (lhs rhs : Lean.Expr) : MetaM Bool := do
  let .fvar memory := lhs.getAppFn | return false
  unless lhs.getAppNumArgs == 2 do return false
  unless rhs.isAppOf ``Option.some || rhs.isAppOf ``Option.none do return false
  return (← whnfR (← memory.getType)).isAppOf ``VerexIR.Proofs.Denote.Memory

/-- The rules of the block's normal form at a goal, besides `except`. -/
private def blockRules (goal : MVarId) (except : Option Name := none) : MetaM BlockRules := do
  -- A goal's rules are a function of its context: computed once per goal.
  let cache := (← targetState).stepBlockRules
  if except.isNone then
    if let some parts := (← cache.get)[goal]? then
      if parts.size == 8 then
        return {
          reads := parts[0]!
          definitions := parts[1]!
          definitionsForward := parts[2]!
          facts := parts[3]!
          booleans := parts[4]!
          aliases := parts[5]!
          aliasesReversed := parts[6]!
          factsReversed := parts[7]! }
  let rules ← scanRules goal except
  if except.isNone then
    cache.modify (·.insert goal #[rules.reads, rules.definitions, rules.definitionsForward,
      rules.facts, rules.booleans, rules.aliases, rules.aliasesReversed, rules.factsReversed])
  return rules
where
  scanRules (goal : MVarId) (except : Option Name) : MetaM BlockRules := goal.withContext do
    let marked ← markedFacts
    (← getLCtx).foldlM (init := { reads := #[], definitions := #[], facts := #[] }) fun rules decl => do
      if some decl.userName == except then return rules
      if marked.contains decl.fvarId then
        return match ← factRule (← instantiateMVars decl.type) with
          | .forward => { rules with facts := rules.facts.push decl.fvarId }
          | .reversed => { rules with factsReversed := rules.factsReversed.push decl.fvarId }
          | .none => rules
      if let some x ← resultDefinition? decl then
        unless decl.isImplementationDetail || (← isProp (← x.getDecl).type) do
          return { rules with definitionsForward := rules.definitionsForward.push decl.fvarId }
      let some (type, lhs, rhs) := (← instantiateMVars decl.type).eq? | return rules
      -- A memory's entry the path splits on, read as the case's value: a
      -- step's storage read reads it (`stateScopeName`).
      if ← memoryCase? lhs rhs then
        unless decl.isImplementationDetail do
          return { rules with definitionsForward := rules.definitionsForward.push decl.fvarId }
      if rhs.isFVar && lhs.isAppOfArity ``VerexIR.SpecMap.ofEntries 7 then
        -- A hidden bridge still reads its carrier.
        return { rules with reads := rules.reads.push decl.fvarId }
      if decl.isImplementationDetail then return rules
      if type.isConstOf ``Bool && (rhs.isConstOf ``Bool.true || rhs.isConstOf ``Bool.false) &&
          !lhs.isFVar then
        return { rules with booleans := rules.booleans.push decl.fvarId }
      -- A name or term equated to a literal, as a case split states it: a
      -- rule toward the literal (substitution in a leaf is total).
      if sideRank lhs ≥ 1 && sideRank rhs == 0 then
        return { rules with facts := rules.facts.push decl.fvarId }
      if sideRank lhs == 0 && sideRank rhs ≥ 1 then
        return { rules with factsReversed := rules.factsReversed.push decl.fvarId }
      -- A named value's definition: the local, or an integer's value,
      -- equated to a term without it.
      let defined? := match lhs with
        | .fvar x => some x
        | _ => if lhs.isAppOfArity ``VerexIR.SpecInt.val 3 then
            match lhs.appArg! with | .fvar x => some x | _ => none
          else none
      -- Its term is no name itself: folding it shrinks the term, so the
      -- folds of several definitions end.
      if let (some x, some y) := (defined?, localName? rhs) then
        if x != y then
          let older := (← y.getDecl).index < (← x.getDecl).index
          return if older then { rules with aliases := rules.aliases.push decl.fvarId }
            else { rules with aliasesReversed := rules.aliasesReversed.push decl.fvarId }
      if let some x := defined? then
        let named ← x.getDecl
        -- A term folds into its name as a fact's does (`factRule`): a value
        -- is no definition, a name's value is not folded back into it.
        if !named.userName.hasMacroScopes && sideRank rhs == 2 && !rhs.containsFVar x &&
            !(← isProp named.type) then
          return { rules with definitions := rules.definitions.push decl.fvarId }
        -- A value no proof can name is its term or literal: substituted
        -- (substitution in a leaf is total).
        if named.userName.hasMacroScopes && sideRank rhs != 1 && !rhs.containsFVar x &&
            !(← isProp named.type) then
          return { rules with definitionsForward := rules.definitionsForward.push decl.fvarId }
      -- The same definition, the name on the right.
      if let some x := localName? rhs then
        let named ← x.getDecl
        if defined?.isNone && !named.userName.hasMacroScopes && sideRank lhs == 2 &&
            !lhs.containsFVar x && !(← isProp named.type) then
          return { rules with definitionsForward := rules.definitionsForward.push decl.fvarId }
      return rules

/-- A simp call of the block's normal form: rules alone, the transport
normal form (where a generic lemma's frame occurs), or the denotation's
normal form. -/
private inductive StepCall where
  | rules | transport | normal
  deriving BEq

/-- The calls of the block's normal form: the reading of carriers, then the
denotation's normal form. -/
private def blockCalls (instantiated : Bool) : StepCall × StepCall :=
  (if instantiated then .transport else .rules, .normal)

/-- The rules reading a membership by its positions, which the step normal
form does not apply: a membership is an atom there. -/
private def stepPositional : Array Name :=
  #[``VerexIR.Proofs.Denote.mem_iff_exists_int_index, ``VerexIR.Proofs.Denote.findIndex?_eq_none_iff,
    ``VerexIR.Proofs.Denote.findIndex?_isSome_iff]

/-- A simp call's context and simprocs, built once per target. -/
private def cachedSimpContext (call : StepCall) : TacticM (Simp.Context × Simp.SimprocsArray) := do
  let key := match call with | .rules => 0 | .transport => 1 | .normal => 2
  let cache := (← targetState).stepSimpContexts
  if let some (_, context, simprocs) := (← cache.get).find? (·.1 == key) then
    return (context, simprocs)
  let syntax_ ← match call with
    | .rules => `(tactic| simp only [])
    | .transport => `(tactic| simp only [vir_transport])
    -- Membership an atom (`stepPositional`), a search for an element read
    -- as the atom.
    | .normal => `(tactic| simp only [vir_denote, vir_denote_norm, vir_denote_spec, vir_denote_eval,
        Prod.fst, Prod.snd, reduceStatedIte, VerexIR.Proofs.Denote.findIndex?_eq_none_mem_iff,
        VerexIR.Proofs.Denote.findIndex?_isSome_mem_iff, VerexIR.Proofs.Denote.exists_mem_val_eq_iff, VerexIR.Proofs.Denote.forall_mem_val_ne_iff,
        VerexIR.Proofs.Denote.exists_position_some_iff, VerexIR.Proofs.Denote.forall_position_some_ne_iff, VerexIR.Proofs.Denote.exists_position_eq_iff,
        VerexIR.Proofs.Denote.forall_position_ne_iff])
  let { ctx, simprocs, .. } ← mkSimpContext syntax_ (eraseLocal := false)
  let theorems := if call != .normal then ctx.simpTheorems else
    ctx.simpTheorems.map fun set => stepPositional.foldl (init := set) fun set name =>
      set.eraseCore (.decl name)
  let ctx ← Simp.mkContext { ctx.config with failIfUnchanged := false } theorems
    ctx.congrTheorems
  cache.modify (·.push (key, ctx, simprocs))
  return (ctx, simprocs)

/-- The simp theorems of a hypothesis as a rule, read right to left where
`inverse`, before its subterms unless `post`: built once per target
(`stepRuleTheorems`), as they are a function of the hypothesis. -/
private def ruleTheorems (rule : FVarId) (inverse post : Bool) : TacticM (Array SimpTheorem) := do
  let cache := (← targetState).stepRuleTheorems
  if let some theorems := (← cache.get)[(rule, inverse, post)]? then return theorems
  let proof ← if inverse then pure (mkFVar rule) else pure (← orientedHypothesis rule).1
  let theorems ← mkSimpTheoremFromExpr (.fvar rule) #[] proof (inv := inverse) (post := post)
  cache.modify (·.insert (rule, inverse, post) theorems)
  return theorems

/-- The context of a call with hypotheses as rules besides its lemmas, some
read right to left (`preReversed`: before their subterms, so that a term
folds into the name of the largest definition it is). The rules are
assembled per goal from their theorems (`ruleTheorems`). -/
private def withRules (call : StepCall) (rules reversed : Array FVarId)
    (contextual : Bool := false) (preReversed : Bool := false) :
    TacticM (Simp.Context × Simp.SimprocsArray) := do
  let (ctx, simprocs) ← cachedSimpContext call
  let config := if contextual then { ctx.config with contextual := true } else ctx.config
  if rules.isEmpty && reversed.isEmpty then
    return (← Simp.mkContext config ctx.simpTheorems ctx.congrTheorems, simprocs)
  let mut extra : SimpTheorems := {}
  for rule in rules do
    for theorem_ in ← ruleTheorems rule false true do extra := extra.addSimpTheorem theorem_
  for rule in reversed do
    for theorem_ in ← ruleTheorems rule true !preReversed do extra := extra.addSimpTheorem theorem_
  return (← Simp.mkContext config (ctx.simpTheorems.push extra) ctx.congrTheorems, simprocs)

/-- A side condition from the step's rules, else as the leaf's normalization
decides it (`goalDischarge`): one normal form for steps and leaves. -/
private def stepDischarge (goal : MVarId) : MetaM Simp.Discharge := do
  let context ← goalDischarge goal
  return fun condition => do
    if let some proof ← Simp.dischargeDefault? condition then return some proof
    context condition

/-- `simp` at a goal's hypotheses and, when asked, its target, by a cached
call with rules. The goal after, none where it is closed. -/
private def simpCached (goal : MVarId) (call : StepCall) (rules reversed : Array FVarId)
    (targets : Array FVarId) (simplifyTarget : Bool) (preReversed : Bool := false) :
    TacticM (Option MVarId) := do
  setGoals [goal]
  goal.withContext do
    let (ctx, simprocs) ← withRules call rules reversed (preReversed := preReversed)
    -- A target is simplified without itself as a rule (`simpGoal`).
    let (result, _) ← simpGoal goal ctx simprocs (discharge? := some (← stepDischarge goal))
      (simplifyTarget := simplifyTarget)
      (fvarIdsToSimp := targets)
    return result.map (·.2)

/-- The rules of the block's normal form, and those of them read right to
left. -/
private def BlockRules.normalRules (rules : BlockRules) : Array FVarId × Array FVarId :=
  (rules.facts ++ rules.reads ++ rules.booleans ++ rules.aliases ++ rules.definitionsForward,
    rules.definitions ++ rules.aliasesReversed ++ rules.factsReversed)

/-- The phases of the block's normal form over `start` (a goal or a term):
the reading of the bridges, the identity where no bridge reads and no frame
occurs, then the normal form, where a bridge in its spelling reads the
carrier it gives. `phase` takes the call, its rules, and whether it is the
normal form; none where a phase closes the goal. -/
private def normalPhases {α : Type} (rules : BlockRules) (instantiated : Bool) (start : α)
    (phase : α → StepCall → Array FVarId → Array FVarId → Bool → TacticM (Option α)) :
    TacticM (Option α) := do
  let (reading, normalCall) := blockCalls instantiated
  let some read ← if rules.reads.isEmpty && !instantiated then pure (some start)
      else phase start reading rules.reads #[] false
    | return none
  let (forward, reversed) := rules.normalRules
  phase read normalCall forward reversed true

private def normalizeFact (goal : MVarId) (hypothesis : Option Name)
    (rules? : Option BlockRules := none) : TacticM (Option MVarId) := do
  let targets (goal : MVarId) : MetaM (Array FVarId) := goal.withContext do
    let some name := hypothesis | return #[]
    return ((← getLCtx).findFromUserName? name).map (#[·.fvarId]) |>.getD #[]
  let rules ← match rules? with
    | some rules => pure rules
    | none => blockRules goal hypothesis
  let instantiated ← goal.withContext do
    let statement ← match hypothesis with
      | some name => match (← getLCtx).findFromUserName? name with
        | some decl => instantiateMVars decl.type
        | none => pure (mkConst ``True)
      | none => instantiateMVars (← goal.getType)
    return (statement.find? (·.isConstOf ``Skolems.instantiate)).isSome
  normalPhases rules instantiated goal fun goal call forward reversed _ => do
    simpCached goal call forward reversed (← targets goal) hypothesis.isNone

/-- Whether a term may hold an instance of another, by its head: what a
rewrite by the other could change. -/
private def mayHold (term pattern : Lean.Expr) : Bool :=
  match pattern.getAppFn with
  | .fvar x => term.containsFVar x
  | .const name _ => (term.find? (·.isAppOf name)).isSome
  | _ => true

/-- What a rewrite by a pattern changes is read off its head (`mayHold`): a
local, a constant, or any other. -/
private inductive PatternHead where
  | fvar (x : FVarId)
  | const (name : Name)
  | other
  deriving BEq, Hashable

private def PatternHead.of (pattern : Lean.Expr) : PatternHead :=
  match pattern.getAppFn with
  | .fvar x => .fvar x
  | .const name _ => .const name
  | _ => .other

/-- The constants and locals a term holds, collected once, against which
`mayHold` is a membership test. -/
private structure HeldHeads where
  constants : NameSet
  locals : FVarIdSet

private def HeldHeads.of (term : Lean.Expr) : HeldHeads :=
  { constants := term.getUsedConstantsAsSet, locals := (collectFVars {} term).fvarSet }

/-- `mayHold` of the term these heads are of and a pattern of this head. -/
private def HeldHeads.mayHold (held : HeldHeads) : PatternHead → Bool
  | .fvar x => held.locals.contains x
  | .const name => held.constants.contains name
  | .other => true

/-- Run `x` in the step mode `change` gives, the mode before restored
after it, also where it fails. -/
private def withStepMode {α : Type} (change : StepMode → StepMode) (x : TacticM α) :
    TacticM α := do
  let mode := (← targetState).stepMode
  let saved ← mode.get
  mode.set (change saved)
  try x finally mode.set saved

/-- Whether the steps run at a program point (`StepMode.pointFacts`). -/
private def atPoint : CoreM Bool := return (← (← targetState).stepMode.get).pointFacts

/-- Whether a scope entry's name keeps its value's definition as stated: one
a binding site gave (a source `let`, the exit's `result`) only where the
block reads it. -/
private def keepsDefinition (name : String) : CoreM Bool := do
  unless (← (← targetState).bindingNames.get).contains name do return true
  match (← (← targetState).stepMode.get).blockReads with
  | some reads => return reads.contains name
  | none => return true

/-- The leaf's definitions of named values in normal form with each other:
each one's term with the others folded to their names. -/
private def reduceDefinitions (goal : MVarId) (changed : Option (Array Name) := none) :
    TacticM MVarId := do
  if ← atPoint then return goal
  let sides (goal : MVarId) (definitions : Array FVarId) :
      MetaM (Array (FVarId × Name × Lean.Expr)) :=
    goal.withContext do definitions.filterMapM fun definition => do
      let some (_, lhs, rhs) := (← instantiateMVars (← definition.getType)).eq? | return none
      let term := if (localName? rhs).isSome && (localName? lhs).isNone then lhs else rhs
      return some (definition, (← definition.getDecl).userName, term)
  let rules ← blockRules goal
  let all ← sides goal (rules.definitions ++ rules.definitionsForward)
  -- After a change, the changed definitions and those whose terms may hold
  -- theirs.
  let held := all.map fun (_, _, term) => HeldHeads.of term
  let heads := all.map fun (_, _, term) => PatternHead.of term
  let candidates : Array (FVarId × Name × Lean.Expr × HeldHeads × PatternHead) :=
    let indexed := (all.zip (held.zip heads)).map fun ((definition, name, term), (held, head)) =>
      (definition, name, term, held, head)
    match changed with
    | none => indexed
    | some changed =>
        let changedHeads := indexed.filterMap fun (_, name, _, _, head) =>
          if changed.contains name then some head else none
        indexed.filter fun (_, name, _, held, _) =>
          changed.contains name || changedHeads.any held.mayHold
  -- Only a definition whose term holds another's, or that an alias renames,
  -- changes.
  let aliased := !rules.aliases.isEmpty || !rules.aliasesReversed.isEmpty
  -- A scope name's own definition is kept as stated: the others fold into
  -- its name, never it into theirs.
  let scopedNames ← goal.withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      let some (name, _, value) := scopeEntry? (← instantiateMVars decl.type) | return found
      unless ← keepsDefinition name do return found
      return match localName? value with
        | some x => found.push x
        | none => found
  let definesScoped (definition : FVarId) : MetaM Bool := goal.withContext do
    let some (_, lhs, rhs) := (← instantiateMVars (← definition.getType)).eq? | return false
    return [lhs, rhs].any fun side => (localName? side).any scopedNames.contains
  -- The definitions by their terms' heads, each definition once per entry.
  let headCounts : Std.HashMap PatternHead Nat :=
    heads.foldl (fun counts head => counts.insert head (counts.getD head 0 + 1)) {}
  let entries : Std.HashMap FVarId Nat :=
    all.foldl (fun counts (definition, _, _) => counts.insert definition (counts.getD definition 0 + 1)) {}
  -- Whether another definition's term has a head the term holds.
  let holdsOther (definition : FVarId) (held : HeldHeads) (own : PatternHead) : Bool :=
    let others (head : PatternHead) : Nat :=
      headCounts.getD head 0 - (if head == own then entries.getD definition 0 else 0)
    others .other > 0 ||
      held.locals.toList.any (fun x => others (.fvar x) > 0) ||
      held.constants.toList.any (fun name => others (.const name) > 0)
  let targets ← candidates.filterMapM fun (definition, _, _, held, own) => do
    if ← definesScoped definition then return none
    return if aliased || holdsOther definition held own then some definition else none
  if targets.isEmpty then return goal
  -- All at once, each without itself (`simpGoal`), the definitions read
  -- before their subterms: a term folds into the largest definition it is,
  -- as folding them one after another does.
  let some next ← simpCached goal .rules (rules.aliases ++ rules.definitionsForward)
      (rules.definitions ++ rules.aliasesReversed) targets false (preReversed := true)
    | throwError "a definition refutes the leaf"
  return next

/-- `simp` of a term at a goal's context, with rules besides the call's
lemmas, some of them read right to left. -/
private def simpTerm (goal : MVarId) (call : StepCall) (rules reversed : Array FVarId)
    (e : Lean.Expr) (contextual : Bool := false) : TacticM Simp.Result := do
  setGoals [goal]
  goal.withContext do
    let (ctx, simprocs) ← withRules call rules reversed contextual
    let methods := Simp.mkDefaultMethodsCore simprocs
    let (result, _) ← Simp.main e ctx (methods := { methods with discharge? := ← stepDischarge goal })
    return result

/-- A statement in the block's normal form at a goal (`normalizeFact`), once:
an implication's premises decide its conclusion's conditionals. The normal
form, and the proof that the statement equals it, none where it is the
statement. -/
def normalForm (goal : MVarId) (e : Lean.Expr) (rules? : Option BlockRules := none) :
    TacticM (Lean.Expr × Option Lean.Expr) := do
  let rules ← match rules? with
    | some rules => pure rules
    | none => blockRules goal
  let e ← instantiateMVars e
  let (forward, reversed) := rules.normalRules
  let locals ← goal.withContext do return (← getLCtx).foldl (init := #[]) (·.push ·.fvarId)
  let key := (e, rules.reads, forward, reversed, locals)
  let cache := (← targetState).stepNormalForms
  if !e.hasMVar then
    if let some found := (← cache.get)[key]? then
      setGoals [goal]
      return found
  let instantiated := (e.find? (·.isConstOf ``Skolems.instantiate)).isSome
  let some result ← normalPhases rules instantiated ({ expr := e } : Simp.Result)
      fun read call forward reversed normal => do
    let result ← simpTerm goal call forward reversed read.expr (contextual := normal && e.isArrow)
    goal.withContext do return some (← read.mkEqTrans result)
    | unreachable!
  let found := (← instantiateMVars result.expr, ← result.proof?.mapM instantiateMVars)
  unless e.hasMVar || found.1.hasMVar || found.2.any (·.hasMVar) do
    cache.modify (·.insert key found)
  return found

/-- Substitute a hypothesis equating a local with a term not holding it over
the whole context, as preparation substitutes values; of two locals, the
later one goes. The goal after, none where the hypothesis is no such
equation. -/
private def substituteFact (goal : MVarId) (fact : FVarId) : MetaM (Option MVarId) :=
  goal.withContext do
    let some (_, lhs, rhs) := (← instantiateMVars (← fact.getType)).eq? | return none
    let eliminable (side other : Lean.Expr) : MetaM Bool := do
      unless side.isFVar && !other.containsFVar side.fvarId! do return false
      return !(← side.fvarId!.getDecl).isLet
    -- Of two locals the later one goes; a local and a term leave the term.
    let rightLater ← if lhs.isFVar && rhs.isFVar then
        pure (decide ((← rhs.fvarId!.getDecl).index > (← lhs.fvarId!.getDecl).index))
      else pure true
    if rightLater && (← eliminable rhs lhs) then
      return (← substCore? goal fact (symm := true)).map (·.2)
    else if ← eliminable lhs rhs then
      return (← substCore? goal fact (symm := false)).map (·.2)
    else return none

/-- Settle a fact of a goal in the block's normal form (`factRule`): a name
equated to a value or to another name is substituted; a term equated to a
name is folded into the name over the context, once; any other fact rewrites
no hypothesis. The goal after, none where it is closed. -/
private def settle (goal : MVarId) (fact : FVarId) (statement : Lean.Expr) :
    TacticM (Option MVarId) := do
  -- At a program point a fact is only given: the walk goes on over the goal.
  if ← atPoint then return some goal
  let some (_, lhs, rhs) := statement.eq? | return some goal
  let rule ← goal.withContext (factRule statement)
  if rule == .none then return some goal
  let (high, low) := if rule == .forward then (lhs, rhs) else (rhs, lhs)
  -- Only an equation with a name side, or of a term and a literal, rewrites
  -- the context.
  unless sideRank low ≤ 1 || sideRank high == 1 do return some goal
  if high.isFVar && sideRank low ≤ 1 then
    -- A substitution is the one step that changes the target: where it
    -- does, the target is normalized again, once.
    let mentions ← goal.withContext do
      let target ← instantiateMVars (← goal.getType)
      return [high, low].any fun side => side.isFVar && target.containsFVar side.fvarId!
    if let some substituted ← substituteFact goal fact then
      let substituted ← hideEntries substituted
      -- The hypotheses the substitution changed, in the normal form again:
      -- what a substituted value simplifies (`0 + amt`).
      let original ← goal.withContext getLCtx
      let changed ← substituted.withContext do
        (← getLCtx).foldlM (init := #[]) fun found decl => do
          if decl.isImplementationDetail || original.contains decl.fvarId then return found
          return if ← isProp decl.type then found.push decl.fvarId else found
      let some substituted ← (if changed.isEmpty then pure (some substituted)
        else simpCached substituted .normal #[] #[] changed false) | return none
      if mentions then return ← normalizeFact substituted none
      return some substituted
  -- The hypotheses holding the side that goes, rewritten once.
  let targets ← goal.withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      if decl.isImplementationDetail || decl.fvarId == fact then return found
      let type ← instantiateMVars decl.type
      let holds := match localName? high with
        | some x => type.containsFVar x
        | none => mayHold type high
      return if holds && (← isProp type) then found.push decl.fvarId else found
  -- A term's literal value rewrites the target too.
  let literal := sideRank low == 0
  if targets.isEmpty && !literal then return some goal
  let (rules, reversed) := if rule == .forward then (#[fact], #[]) else (#[], #[fact])
  simpCached goal .rules rules reversed targets literal

/-- A case fact that a memory's entry is some value names the value: `S k`
with `(S k).isSome` is `S k = some v` (`Option.isSome_iff_exists`), the
memory case the block's rules read the entry by (`memoryCase?`). -/
private def nameSomeReads (goal : MVarId) (fact : FVarId) : MetaM (MVarId × Bool) := do
  let parts ← goal.withContext do typedConjuncts (mkFVar fact)
  let mut goal := goal
  let mut named := false
  for (proof, part) in parts do
    let some (_, lhs, rhs) := part.eq? | continue
    unless rhs.isConstOf ``Bool.true && lhs.isAppOfArity ``Option.isSome 2 do continue
    let read := lhs.appArg!
    let isMemoryRead ← goal.withContext do
      let .fvar memory := read.getAppFn | return false
      unless read.getAppNumArgs == 2 do return false
      return (← whnfR (← memory.getType)).isAppOf ``VerexIR.Proofs.Denote.Memory
    unless isMemoryRead do continue
    let witness ← goal.withContext do
      mkAppM ``Iff.mp #[← mkAppOptM ``Option.isSome_iff_exists #[none, read], proof]
    let (stated, next) ← (← goal.assert `read_some (← goal.withContext (inferType witness))
      witness).intro1P
    let subgoals ← next.cases stated #[{ varNames := [`read_value, `read_case] }]
    let some subgoal := subgoals[0]? | continue
    goal := subgoal.mvarId
    named := true
  return (goal, named)

/-- Settle the hypothesis `name` of a goal, normalized (`settle`). The goal
after, none where it is closed. -/
private def settleFact (goal : MVarId) (name : Name) : TacticM (Option MVarId) := do
  let some fact ← goal.withContext do return ((← getLCtx).findFromUserName? name).map (·.fvarId)
    | return some goal
  -- A read the fact states some value is named, and the fact read by it.
  let (goal, named) ← nameSomeReads goal fact
  if named then
    let some goal ← normalizeFact goal (some name) | return none
    return some (← markFact goal ((← goal.withContext do getLocalDeclFromUserName name).fvarId))
  let some next ← settle goal fact (← goal.withContext do instantiateMVars (← fact.getType))
    | return none
  -- A substituted fact is gone; one that stays is marked.
  if (← next.getDecl).lctx.contains fact then return some (← markFact next fact)
  return some next

/-- Give a goal the fact `name : statement`, proved by `proof`, in normal
form: an equation of a local and a term not holding it is substituted once
over the context; any other fact is a rule of the block's normal form and
rewrites no hypothesis. The goal after, none where the fact refutes it. -/
private def giveFact (goal : MVarId) (name : Name) (statement proof : Lean.Expr) :
    TacticM (Option MVarId) := do
  if statement.isConstOf ``True then return some goal
  if statement.isConstOf ``False then
    goal.assign (← goal.withContext (mkFalseElim (← goal.getType) proof))
    return none
  let (_, next) ← (← goal.assert name statement proof).intro1P
  settleFact next name

/-- A fact's proof carried along the equation of its statement to its normal
form. The equation's left side is the statement up to definitional
unfolding, which may pass an irreducible constructor's projection
(`eraseBounded_values`); the kernel checks that conversion. -/
private def castFact (equality proof : Lean.Expr) : MetaM Lean.Expr := do
  let some (_, statement, normal) := (← whnfR (← inferType equality)).eq?
    | mkEqMP equality proof
  return mkApp4 (mkConst ``Eq.mp [Level.zero]) statement normal equality
    (← mkExpectedTypeHint proof statement)

/-- Give a goal the fact `name : statement`, proved by `proof`, normalized
once (`giveFact`). -/
private def giveNormal (goal : MVarId) (name : Name) (statement proof : Lean.Expr)
    (rules? : Option BlockRules := none) : TacticM (Option MVarId) := do
  let (normal, equality?) ← normalForm goal statement rules?
  let proof ← goal.withContext do
    match equality? with
    | some equality => castFact equality proof
    | none => pure proof
  giveFact goal name normal proof

/-- Each fact of the block in the normal form of the goal's context, as at
an obligation after the point that gave it: none where one refutes the
goal. -/
private def renormalizeFacts (goal : MVarId) : TacticM (Option MVarId) := do
  let names ← goal.withContext do
    (← markedFacts).mapM fun fact => return (← fact.getDecl).userName
  -- The hypotheses reading a memory's entry the path split on
  -- (`memoryCase?`), named where their names are inaccessible.
  let (reads, cases) ← goal.withContext do
    let mut cases : Array (FVarId × Lean.Expr) := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let some (_, lhs, rhs) := (← instantiateMVars decl.type).eq? | continue
      if ← memoryCase? lhs rhs then cases := cases.push (decl.fvarId, lhs)
    let mut reads := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail || cases.any (·.1 == decl.fvarId) then continue
      unless ← isProp decl.type do continue
      let type ← instantiateMVars decl.type
      if cases.any fun (_, lhs) => (type.find? (· == lhs)).isSome then
        reads := reads.push decl.fvarId
    return (reads, cases)
  let mut goal := goal
  let mut names := names
  if !cases.isEmpty then
    let reads := reads ++ cases.map (·.1)
    for read in reads, index in [0:reads.size] do
      let name ← goal.withContext do return (← read.getDecl).userName
      if name.hasMacroScopes then
        let fresh := Name.mkSimple s!"read_{index}"
        goal ← goal.rename read fresh
        names := names.push fresh
      else unless names.contains name do names := names.push name
  for name in names do
    let some next ← normalizeFact goal (some name) | return none
    goal := next
  -- A read the case refutes leaves an empty existential.
  if !cases.isEmpty then
    let locations ← goal.withContext do
      let lctx ← getLCtx
      return (names.filter fun name => (lctx.findFromUserName? name).isSome).map
        fun name => (⟨(mkIdent name).raw⟩ : TSyntax `term)
    unless locations.isEmpty do
      setGoals [goal]
      evalTactic (← `(tactic| try simp only [exists_false, exists_const, and_false, false_and,
        false_or, or_false, reduceCtorEq] at $locations*))
      match ← getGoals with
      | [] => return none
      | [next] => goal := next
      | _ => pure ()
  return some goal

/-- What the context's implications and equivalences give where their
premises are stated (`statedProof?`) or true in the block's normal form:
their conclusions, as facts (`implied_k`). Only a conclusion that shares a
local with the goal, or any where the goal is `False`. The goal after, none
where a conclusion refutes it. -/
private def impliedFacts (goal : MVarId) : TacticM (Option MVarId) := do
  let rules ← blockRules goal
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  let refuting := target.consumeMData.isConstOf ``False
  let goalLocals := (collectFVars {} target).fvarSet
  let relevant (e : Lean.Expr) : Bool :=
    refuting || (collectFVars {} e).fvarIds.any goalLocals.contains
  let premiseProof (premise : Lean.Expr) : TacticM (Option Lean.Expr) := do
    if let some proof ← goal.withContext (statedProof? premise) then return some proof
    let (normal, equality?) ← normalForm goal premise rules
    let some equality := equality? | return none
    unless normal.isConstOf ``True do return none
    goal.withContext do return some (← mkAppM ``of_eq_true #[equality])
  let mut implied : Array (Lean.Expr × Lean.Expr) := #[]
  let candidates ← goal.withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      if decl.isImplementationDetail then return found
      return found.push (decl.toExpr, (← instantiateMVars decl.type).consumeMData)
  for (hypothesis, type) in candidates do
    if let some (left, right) := type.iff? then
      if relevant right then
        if let some proof ← premiseProof left then
          implied := implied.push (right, ← goal.withContext (mkAppM ``Iff.mp #[hypothesis, proof]))
          continue
      if relevant left then
        if let some proof ← premiseProof right then
          implied := implied.push (left, ← goal.withContext (mkAppM ``Iff.mpr #[hypothesis, proof]))
      continue
    -- An implication of propositions, its premises in order.
    let mut premises := #[]
    let mut conclusion := type
    while conclusion.isArrow do
      premises := premises.push conclusion.bindingDomain!
      conclusion := conclusion.bindingBody!.lowerLooseBVars 1 1
    if premises.isEmpty || !relevant conclusion then continue
    unless ← goal.withContext (premises.allM fun premise => isProp premise) do continue
    if (← goal.withContext (statedProof? conclusion)).isSome then continue
    let mut proofs := #[]
    for premise in premises do
      let some proof ← premiseProof premise | break
      proofs := proofs.push proof
    if proofs.size == premises.size then
      implied := implied.push (conclusion, mkAppN hypothesis proofs)
  if implied.isEmpty then return some goal
  -- Given at once, then each normalized and settled by its name.
  let names := implied.mapIdx fun index _ => Name.mkSimple s!"implied_{index}"
  let (_, given) ← goal.assertHypotheses (implied.mapIdx fun index (statement, proof) =>
    { userName := names[index]!, type := statement, value := proof })
  let mut goal := given
  for name in names do
    let some next ← normalizeFact goal (some name) rules | return none
    let some next ← settleFact next name | return none
    goal := next
  return some goal

/-- The goal as an instance of a quantified fact, each premise of the
instance in the block's normal form, then stated by a hypothesis or decided by
arithmetic: the premise matching a lemma's premises have
(`doc/plan/proof-language.md`, "Closers"). Whether it closes the goal. -/
private partial def blockInstance (goal : MVarId) : TacticM Bool := do
  if ← blockInstanceAt goal then return true
  -- A function value's result the normal form names, read as the
  -- invocation's result where a quantified fact states it (`resultDefinition?`).
  let unfolded ← goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      if decl.isImplementationDetail then return found
      let some x ← resultDefinition? decl | return found
      unless target.containsFVar x do return found
      let (arguments, _, body) ← forallMetaTelescopeReducing (← instantiateMVars decl.type)
      let some (_, invocation, _) := body.eq? | return found
      let motive ← mkLambdaFVars #[mkFVar x] target
      let read := motive.beta #[invocation]
      let next ← mkFreshExprSyntheticOpaqueMVar read
      let equation ← mkCongrArg motive (mkAppN decl.toExpr arguments)
      return found.push (next, ← mkEqMP equation next)
  -- An equation, or a disequation, is an instance either way round.
  let symmetric ← goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    if let some (_, lhs, rhs) := target.eq? then
      let next ← mkFreshExprSyntheticOpaqueMVar (← mkEq rhs lhs)
      return #[(next, ← mkEqSymm next)]
    if target.isAppOfArity ``Not 1 then
      if let some (_, lhs, rhs) := target.appArg!.eq? then
        let next ← mkFreshExprSyntheticOpaqueMVar (mkNot (← mkEq rhs lhs))
        return #[(next, ← mkAppM ``Ne.symm #[next])]
    return #[]
  for (next, proof) in symmetric ++ unfolded do
    let saved ← saveState
    if ← blockInstanceAt next.mvarId! then
      let proof ← instantiateMVars proof
      unless proof.hasExprMVar do
        goal.assign proof
        return true
    -- The names the attempt drew stay drawn: the step caches are keyed by
    -- goal and must not meet a goal of the same name.
    let names ← getNGen
    saved.restore
    setNGen names
  return false
where
  blockInstanceAt (goal : MVarId) : TacticM Bool := do
    let target ← goal.withContext do instantiateMVars (← goal.getType)
    if target.isConstOf ``False then return false
    let candidates ← goal.withContext do
      (← getLCtx).foldlM (init := #[]) fun found decl => do
        if decl.isImplementationDetail then return found
        return if (← whnfR (← instantiateMVars decl.type)).isForall then found.push decl.toExpr
          else found
    for candidate in candidates.reverse do
      let saved ← saveState
      try
        let premises ← withReducible (goal.apply candidate)
        let mut closed := true
        for premise in premises do
          if ← premise.isAssigned then continue
          unless ← goal.withContext (isProp (← premise.getType)) do
            closed := false
            break
          let some normal ← normalizeFact premise none | continue
          setGoals [normal]
          try evalTactic (← `(tactic| first | verex_assumption | omega))
          catch _ => pure ()
          unless (← getUnsolvedGoals).isEmpty do
            closed := false
            break
        if closed then return true
      catch _ => pure ()
      -- The names the attempt drew stay drawn: the step caches are keyed by
      -- goal and must not meet a goal of the same name.
      let names ← getNGen
      saved.restore
      setNGen names
    return false

/-- A conjunction each of whose parts a hypothesis states (`statedProof?`):
its proof. -/
private partial def witnessParts (e : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let e := (← instantiateMVars e).consumeMData
  if let some (left, right) := e.and? then
    let some l ← witnessParts left | return none
    let some r ← witnessParts right | return none
    return some (← mkAppM ``And.intro #[l, r])
  statedProof? e

/-- An existential whose body the context states at one of its locals: the
witness a case split or a destructured fact left, each conjunct of the body
stated by a hypothesis (`statedProof?`). Whether it closes the goal. -/
private def statedWitness (goal : MVarId) : MetaM Bool := goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  let target ← if target.isAppOf ``VerexIR.Proofs.Obligation then whnfD target else whnfR target
  let_expr Exists α body := target | return false
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    unless ← isDefEq (← instantiateMVars decl.type) α do continue
    let instantiated := body.beta #[decl.toExpr]
    if let some proof ← witnessParts instantiated then
      goal.assign (← mkAppOptM ``Exists.intro #[α, body, decl.toExpr, proof])
      return true
  return false

@[tactic blockDecideTactic] def evalBlockDecide : Tactic := fun _ => do
  let goal ← getMainGoal
  if (← closeByStatement goal) || (← statedWitness goal) then
    replaceMainGoal []
    return
  let some goal ← impliedFacts goal | replaceMainGoal []
  replaceMainGoal [goal]
  if ← closeByStatement goal then
    replaceMainGoal []
    return
  let target ← goal.withContext do whnfR (← instantiateMVars (← goal.getType))
  -- The order is a cost: a refutation is mostly arithmetic, a goal reading a
  -- map's positions or values mostly an instance of a quantified fact.
  let readsMap := (target.find? fun term =>
    term.isAppOf ``VerexIR.SpecMap.keyAt || term.isAppOf ``VerexIR.SpecMap.valueAt ||
      term.isAppOf ``VerexIR.SpecMap.rank || term.isAppOf ``VerexIR.SpecMap.hasKey).isSome
  if target.isConstOf ``False then
    evalTactic (← `(tactic| first | verex_negation_conflict | omega | verex_bool_conflict |
      verex_instance))
  else if readsMap then
    evalTactic (← `(tactic| first | verex_negation_conflict | verex_bool_conflict | verex_instance |
      omega))
  else
    try
      evalTactic (← `(tactic| first | verex_negation_conflict | verex_bool_conflict | omega |
        verex_instance))
    catch failure =>
      unless ← blockInstance goal do throw failure
      setGoals []

/-- Settle the equations of names and values a case split leaves in a goal's
context (`settle`), as a fact of such an equation is: a split computes the
hypotheses that test the value it splits, and what they state of the case,
such as a call's result equal to the variant's payload, is substituted over
the context. The goal after, none where it is closed. -/
private def settleCaseEquations (goal : MVarId) : TacticM (Option MVarId) := do
  let mut goal := goal
  let mut settled : Array Lean.Expr := #[]
  repeat
    let candidate ← goal.withContext do
      (← getLCtx).findDeclM? fun decl => do
        if decl.isImplementationDetail then return none
        let statement ← instantiateMVars decl.type
        let some (_, lhs, rhs) := statement.eq? | return none
        unless sideRank lhs ≤ 1 && sideRank rhs ≤ 1 && lhs != rhs do return none
        if settled.contains statement then return none
        return some (decl.fvarId, statement)
    let some (fact, statement) := candidate | break
    settled := settled.push statement
    let some next ← settle goal fact statement | return none
    goal := next
  return some goal

/-- A split's cases: their equations of names settled (`settleCaseEquations`),
then the bounds of the values a case reads. -/
private def prepareCases (cases : Array MVarId) : TacticM (List MVarId) := do
  let settled ← cases.filterMapM settleCaseEquations
  setGoals settled.toList
  evalTactic (← `(tactic| all_goals verex_type_bounds))
  getGoals

/-- Whether a binder of a built-in lemma's theorem is filled other than by a
parameter: the discipline or the order, which a map's owner gives, the
fallback of a vector's elements, the read a clause reads them by and its
injectivity, a type, an instance, or a premise, which the lemma owes. -/
private def lemmaBinderFilled (name : Name) (type : Lean.Expr) : MetaM Bool := do
  if name == `discipline || name == `order || name == `fallback || name == `read ||
      name == `injective || name == `eq || name == `sound then return true
  if (← whnfD type).isSort then return true
  if (← isClass? type).isSome then return true
  isProp type

/-- The built-in lemmas whose conclusion is matched against the obligation
rather than stated at arguments (`frameConclusion`). -/
def obligationLemmas : Array Name := #[``VerexIR.Proofs.Denote.StepLemmas.stored_invariants_frame]

/-- The theorem and the parameters of the built-in lemma `theorem_` is. -/
private def builtinParameters? (theorem_ : Name) : Option (Array LemmaParameter) :=
  (mapLemmas.find? (·.theorem_ == theorem_)).map (·.parameters) <|>
    (builtinLemmas.find? (·.theorem_ == theorem_)).map (·.parameters)

-- The tables checked against their theorems when this module is built: each
-- theorem resolves, each parameter fills a binder of it, and every other
-- binder is the discipline, the order, the fallback, a type, an instance or
-- a premise.
run_meta do
  let entries := mapLemmas.map (fun lemma => (lemma.theorem_, lemma.parameters)) ++
    (builtinLemmas.filter (!obligationLemmas.contains ·.theorem_)).map fun lemma =>
      (lemma.theorem_, lemma.parameters)
  for (theorem_, parameters) in entries do
    let info ← getConstInfo theorem_
    forallTelescopeReducing info.type fun binders _ => do
      let names ← binders.mapM fun binder => return (← binder.fvarId!.getDecl).userName
      for parameter in parameters do
        unless names.contains parameter.binder do
          throwError "built-in lemma {theorem_}: parameter `{parameter.name}` names no binder \
            `{parameter.binder}`"
      for binder in binders, name in names do
        if parameters.any (·.binder == name) then continue
        unless ← lemmaBinderFilled name (← inferType binder) do
          throwError "built-in lemma {theorem_}: binder `{name}` is filled by nothing"

/-- How a clause reads a vector's elements of native type `τ`, carried as
`carrier`, and the proof that the read tells elements apart: an integer by
its value, a Boolean as a proposition, any other value itself. -/
private def elementRead (carrier : Lean.Expr) (τ : NTy) : MetaM (Lean.Expr × Lean.Expr) := do
  withLocalDeclD `element carrier fun element => do
    let integer (width signed : Lean.Expr) : MetaM (Lean.Expr × Lean.Expr) := do
      return (← mkLambdaFVars #[element] (← mkAppM ``VerexIR.SpecInt.val #[element]),
        mkApp2 (mkConst ``VerexIR.Proofs.Denote.StepLemmas.int_read_injective) width signed)
    match τ with
    | .int width signed =>
        integer (mkApp (mkConst ``VerexIR.IntWidth.bits) (mkNatLit width)) (toExpr signed)
    | .num => integer (mkConst ``VerexIR.IntWidth.unbounded) (toExpr true)
    | .bool =>
        return (← mkLambdaFVars #[element]
            (mkApp3 (mkConst ``Eq [Level.one]) (mkConst ``Bool) element (mkConst ``Bool.true)),
          mkConst ``VerexIR.Proofs.Denote.StepLemmas.bool_read_injective)
    | _ =>
        return (← mkLambdaFVars #[element] element,
          mkApp (mkConst ``VerexIR.Proofs.Denote.StepLemmas.self_read_injective) carrier)

/-- The equality a search compares a vector's elements of quoted native type
`τ` by (`NTy.eqb`), and its soundness, where the type holds no reference and
no ghost. -/
private def elementEquality (carriers τ : Lean.Expr) : MetaM (Lean.Expr × Lean.Expr) := do
  let decided (flag : Name) : MetaM Lean.Expr := do
    let statement ← mkEq (mkApp (mkConst flag) τ) (mkConst ``Bool.true)
    unless ← withDefault (isDefEq (mkApp (mkConst flag) τ) (mkConst ``Bool.true)) do
      throwError "a search compares elements holding a reference or a ghost"
    mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true)) statement
  return (mkApp2 (mkConst ``NTy.eqb) carriers τ,
    mkApp4 (mkConst ``NTy.eqb_iff) carriers τ (← decided ``NTy.refFree)
      (← decided ``NTy.ghostFree))

/-- A built-in lemma's instance at its application: its theorem at the
translated arguments, a map as its owner's typed map, a key or a value as the
typed value the map holds it as, the map's discipline as its owner gives it,
an integer, a vector or an element as the clause states it, and a vector's
fallback as a clause reads one. The instance's proof, its statement, and its
premises as goals. -/
def lemmaInstance (goal : MVarId) (step : LoweredStep) (reference : Syntax) (lemmaTheorem : Name) :
    TacticM (Lean.Expr × Lean.Expr × List MVarId) := goal.withContext do
  let context ← stepContext step reference
  let some { kind := .operation (.specification (.lemma lemmaReference _)) _ arguments, .. } :=
      context.ns.expressions[step.expression.index]?
    | throwErrorAt reference "an `apply` names a lemma at its arguments"
  unless (Contract.lemmaOf? step.unit lemmaReference).isSome do
    throwErrorAt reference "the lemma does not resolve"
  let some parameters := builtinParameters? lemmaTheorem
    | throwErrorAt reference "the lemma is not in the tables of built-in lemmas"
  -- The map's owner, whose views read the keys and values.
  let owner? ← (parameters.findIdx? (·.kind == .map)).mapM fun mapIndex => do
    let some mapArgument := arguments[mapIndex]? | throwErrorAt reference "the lemma lacks its map"
    let owner ← Contract.nativeOwner (← context.carriers) (← context.exprType mapArgument)
      (← Contract.translate context mapArgument)
    let some ownerType := (context.ns.expressions[mapArgument.index]?).bind fun expression =>
        context.valueTypeOf? expression.typeId
      | throwErrorAt reference "the lemma's map has no type"
    let some model := Contract.mapModel? step.unit ownerType
      | throwErrorAt reference "the lemma's map is not an intrinsic map"
    return (owner.typed, ← owner.typed.discipline model)
  let typed (view : NativeMaps.TypedView) (argument : VerexIR.ExprId) : MetaM Lean.Expr := do
    let clause ← Contract.translate context argument
    match_expr clause.consumeMData with
    | NTy.unread _ τ _ value => if ← isDefEq τ view.type then return value
    | _ => pure ()
    Contract.TypedTable.typedOf view (← context.exprType argument) clause
  let mut values : Array (Name × Lean.Expr) := #[]
  let mut fallback := none
  let mut read := none
  let mut search := none
  let mut elementType := none
  for parameter in parameters, argument in arguments do
    let value ← match parameter.kind, owner? with
      | .map, some (table, _) => pure table.map
      | .key, some (table, _) => typed table.keys argument
      | .value, some (table, _) => typed table.values argument
      | .int, _ => Contract.translate context argument
      | .element, _ => do
          -- At the vector's element type, an integer read at its width.
          let (value, ε) ← Contract.nativeOperand context argument
          match elementType with
          | some τ => Contract.coerceNative (← context.carriers) ε τ value
          | none => pure value
      | .vector, _ => do
          -- The elements' fallback, as a clause reads a vector.
          if fallback.isNone then
            if let .vector τ ← context.exprType argument then
              let carriers ← context.carriers
              let quoted ← Contract.quoteNTy τ
              fallback := some (← Contract.typedInhabitant carriers quoted)
              read := some (← elementRead (Contract.carrierType carriers quoted) τ)
              search := some (← elementEquality carriers quoted)
              elementType := some τ
          Contract.translate context argument
      | _, none => throwErrorAt reference "a map lemma is applied at no map"
    values := values.push (parameter.binder, value)
  let theorem_ ← mkConstWithFreshMVarLevels lemmaTheorem
  let (holes, _, statement) ← forallMetaTelescopeReducing (← inferType theorem_)
  -- The binders' names, read from the theorem's statement.
  let rec binderNames : Lean.Expr → List Name
    | .forallE name _ body _ => name :: binderNames body
    | _ => []
  let names := binderNames (← inferType theorem_)
  for hole in holes, name in names do
    if let some (_, value) := values.find? (·.1 == name) then
      unless ← isDefEq hole value do
        throwErrorAt reference m!"the lemma's `{name}` does not fit its theorem: {← inferType value} \
          against {← inferType hole}"
  for hole in holes, name in names do
    if name == `fallback || name == `read || name == `injective || name == `eq ||
        name == `sound then
      let value? := if name == `fallback then fallback else if name == `read then read.map (·.1)
        else if name == `injective then read.map (·.2)
        else if name == `eq then search.map (·.1) else search.map (·.2)
      let some value := value? | throwErrorAt reference "the lemma reads no vector"
      unless ← isDefEq hole value do
        throwErrorAt reference m!"the vector's `{name}` does not fit the lemma's theorem"
    else if let some (_, discipline) := owner? then
      if name == `discipline then
        discard <| isDefEq hole discipline
      else if name == `order then
        if discipline.isAppOfArity ``VerexIR.SpecMap.Discipline.ordered 2 then
          discard <| isDefEq hole (discipline.getArg! 1)
  -- Instances by their types, the keys' equality as the map's owner
  -- decides it; the premises are what the lemma owes.
  let mut premises := #[]
  for hole in holes do
    if ← hole.mvarId!.isAssigned then continue
    let type ← instantiateMVars (← inferType hole)
    if (← isClass? type).isSome then
      if let some (table, _) := owner? then
        if type.isAppOfArity ``DecidableEq 1 then
          if ← isDefEq type (← inferType table.keys.decEq) then
            hole.mvarId!.assign table.keys.decEq
            continue
      hole.mvarId!.assign (← synthInstance type)
    else if ← isProp type then
      -- An opaque goal: only its closer proves it, never a unification.
      let premise ← mkFreshExprSyntheticOpaqueMVar type
      hole.mvarId!.assign premise
      premises := premises.push premise.mvarId!
    else throwErrorAt reference m!"the lemma's `{type}` is not determined by its arguments"
  return (← instantiateMVars (mkAppN theorem_ holes), ← instantiateMVars statement, premises.toList)

/-- The proposition a step's expression states at a goal: the lowered
expression translated with its scope's names bound to their values. -/
def translateStep (step : LoweredStep) (reference : Syntax) : MetaM Lean.Expr := do
  Contract.translate (← stepContext step reference) step.expression

/-- The scope names a block's steps read, with the domains and native types
they read them at. -/
def blockScope (file : String) (block : Syntax) :
    MetaM (Array (String × Contract.Domain × NTy)) := do
  let mut names := #[]
  for expression in blockExpressions block do
    let some key := stepKey file expression.expression | continue
    let some step := loweredStep? (← getEnv) key | continue
    let some ns := step.unit.namespaces[0]? | continue
    let base : Contract.Context := {
      unit := step.unit, namespaceId := ⟨0⟩, ns, locals := #[], results := #[] }
    for index in referencedLocals ns step.expression do
      let some localDecl := step.locals[index]? | continue
      if names.any (·.1 == localDecl.name) || expression.binders.any (·.1 == localDecl.name) then
        continue
      let some type := base.valueTypeOf? localDecl.type.typeId | continue
      names := names.push (localDecl.name, Contract.domainOf type,
        ← base.nativeType localDecl.type.typeId)
  return names

/-- A step's lowered expression, as `loweredStepsExtension` records it. -/
private def loweredAt (expression : Syntax) : TacticM LoweredStep := do
  let some key := stepKey (← getFileName) expression
    | throwErrorAt expression "the step has no position"
  let some lowered := loweredStep? (← getEnv) key
    | throwErrorAt expression "the step's expression is not lowered"
  return lowered

/-- At a program point, the walk's hypotheses carry their clauses' markers,
which name the leaves after it; a fact proved there reads them unmarked, in
its own goal, with the bounds of the values in scope, which a leaf's
preparation states. -/
private def readUnmarked : TacticM Unit := do
  if ← atPoint then
    evalTactic (← `(tactic| try simp only [VerexIR.Proofs.Obligation_eq] at *))
    evalTactic (← `(tactic| all_goals verex_type_bounds))

/-- The range of each integer the goal reads that is no local, such as a
field of a resource read from memory: its certificate (`SpecInt.fits`) as
bounds, as a local's type gives them. -/
private def readBounds (goal : MVarId) : TacticM MVarId := goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  let found ← IO.mkRef (#[] : Array Lean.Expr)
  forEachExpr target fun e => do
    if e.isAppOfArity ``VerexIR.SpecInt.val 3 && !e.appArg!.consumeMData.isFVar &&
        !e.hasLooseBVars then
      found.modify fun reads => if reads.contains e then reads else reads.push e
  let reads ← found.get
  if reads.isEmpty then return goal
  let mut goal := goal
  let mut names := #[]
  for read in reads, index in [0:reads.size] do
    let fits ← mkAppM ``VerexIR.SpecInt.fits #[read.appArg!]
    let name := Name.mkSimple s!"read_bounds_{index}"
    let (_, next) ← (← goal.assert name (← inferType fits) fits).intro1P
    goal := next
    names := names.push (mkIdent name)
  setGoals [goal]
  let locations : Array (TSyntax `term) := names.map (⟨·.raw⟩)
  evalTactic (← `(tactic| try simp only [VerexIR.Proofs.Denote.IntegerValueFits_unsigned_succ,
    VerexIR.Proofs.Denote.IntegerValueFits_signed_succ] at $locations*))
  match ← getGoals with
  | [next] => return next
  | _ => return goal

/-- Prove `goal` by `prove`, its hypotheses read unmarked first, which may
close it already: none where it is closed, else why not, where a step inside
`prove` failed. The goals left are those `prove` leaves, or `goal` where it
fails. -/
private def proveReadingWhy (reference : Syntax) (goal : MVarId) (prove : TacticM Unit) :
    TacticM (Option (Option MessageData)) := do
  let goal ← readBounds goal
  setGoals [goal]
  readUnmarked
  if (← getUnsolvedGoals).isEmpty then return none
  try
    withRef reference prove
    return if (← getUnsolvedGoals).isEmpty then none else some none
  catch failure => do setGoals [goal]; return some (some failure.toMessageData)

/-- `proveReadingWhy`: whether the goal is closed. -/
private def proveReading (reference : Syntax) (goal : MVarId) (prove : TacticM Unit) :
    TacticM Bool := do
  return (← proveReadingWhy reference goal prove).isNone

/-- Prove the normal form `owed` of `fact` from `fact` as stated, an
instance of a quantified fact of the context (`blockInstance`), carried by
the normal form's equation. Whether it is proved. -/
private def statedInstance (goal : MVarId) (fact : Lean.Expr) (equality? : Option Lean.Expr)
    (owed : Lean.Expr) : TacticM Bool := do
  let saved ← saveState
  let stated ← goal.withContext (mkFreshExprSyntheticOpaqueMVar fact)
  let (_, inner) ← stated.mvarId!.intros
  unless ← blockInstance inner do saved.restore; return false
  let proof ← goal.withContext do
    match equality? with
    | some equality => mkEqMP equality (← instantiateMVars stated)
    | none => instantiateMVars stated
  unless ← goal.withContext (isDefEq owed proof) do saved.restore; return false
  return true

/-- `fact` normalized as the leaf is, proved by `prove` from the goal's
context: the normal form, its proof, and the equation of the fact to it. -/
private def proveNormal (reference : Syntax) (goal : MVarId) (fact : Lean.Expr)
    (prove : TacticM Unit) (rules : BlockRules) (reasons : Bool := false) :
    TacticM (Lean.Expr × Lean.Expr × Option Lean.Expr) := do
  let (normal, equality?) ← normalForm goal fact rules
  let owed ← goal.withContext (mkFreshExprSyntheticOpaqueMVar normal)
  if normal.isConstOf ``True then owed.mvarId!.assign (mkConst ``True.intro)
  else
    let before ← saveState
    let (_, inner) ← owed.mvarId!.intros
    if (← proveReadingWhy reference inner prove).isSome then
      -- A fact as stated may be an instance of a quantified fact whose
      -- terms the normal form names, such as a function value's result.
      before.restore
      unless ← statedInstance goal fact equality? owed do
        let (_, inner) ← owed.mvarId!.intros
        let why ← proveReadingWhy reference inner prove
        -- A contradiction's block says which of its steps failed.
        let reason := match why, reasons with
          | some (some failure), true => m!"\n{failure}"
          | _, _ => m!""
        withSpecForm <| throwErrorAt reference m!"the step's fact is not proved:\
          {indentD (MessageData.ofGoal ((← getUnsolvedGoals).head?.getD inner))}{reason}"
  return (normal, ← instantiateMVars owed, equality?)

/-- Give the goal the fact `name`: `fact`, normalized as the leaf is, proved
by `prove` from the goal's context. The goal after, none where the fact
refutes it. -/
private def addFact (reference : Syntax) (goal : MVarId) (name : Name) (fact : Lean.Expr)
    (prove : TacticM Unit) (reasons : Bool := false) : TacticM (Option MVarId) := do
  let (normal, proof, _) ← proveNormal reference goal fact prove (← blockRules goal) reasons
  giveFact goal name normal proof


/-- The frame lemma (`lemma_stored_invariants_frame`) matched against the
obligation: the stored invariants at its memory, where one resource changed
whose invariant every value satisfies and the others are kept. The frame
and the stored invariants before are facts of the context; the trivial
invariant is evaluated. The proof and the conclusion. -/
private def frameConclusion (application : Syntax) (closer : TSyntax `tactic) (goal : MVarId) :
    TacticM (Lean.Expr × Lean.Expr) := goal.withContext do
  let rec peel (e : Lean.Expr) (fuel : Nat) : Lean.Expr :=
    match fuel with
    | 0 => e
    | fuel + 1 =>
      let e := e.consumeMData
      if e.isAppOf ``VerexIR.Proofs.Obligation then peel e.appArg! fuel else e
  let target := peel (← instantiateMVars (← goal.getType)) 8
  unless target.isAppOf ``MemoryInvariants do
    throwErrorAt application m!"the frame lemma concludes the stored invariants: the obligation \
      states none:{indentExpr target}"
  let owed ← mkFreshExprSyntheticOpaqueMVar target
  let premises ← owed.mvarId!.apply
    (← mkConstWithFreshMVarLevels ``VerexIR.Proofs.Denote.StepLemmas.stored_invariants_frame)
  -- The frame fixes the modified resource and the memory before, then the
  -- invariants before; the trivial invariant is evaluated last.
  let rank (premise : MVarId) : MetaM Nat := do
    let type ← instantiateMVars (← premise.getType)
    if !(← isProp type) then return 3
    if type.isAppOf ``MemoryInvariants then return 1
    if type.isForall && (type.bindingBody!.find? (·.isAppOf ``Ne)).isSome then return 0
    return 2
  let ranked ← premises.toArray.mapM fun premise => return (← rank premise, premise)
  let ordered := (ranked.qsort (·.1 < ·.1)).filterMap fun (r, premise) =>
    if r == 3 then none else some premise
  for premise in ordered do
    if ← premise.isAssigned then continue
    setGoals [premise]
    try evalTactic (← `(tactic| first
      | assumption
      | (intro _ _; first | assumption | (symm; assumption))
      | (intro; simp only [vir_denote, vir_denote_norm, vir_denote_eval]; done)
      | $closer:tactic))
    catch _ => pure ()
    unless (← getUnsolvedGoals).isEmpty do
      withSpecForm <| throwErrorAt application m!"the frame lemma's premise is not proved:\
        {indentD (MessageData.ofGoal premise)}"
  let proof ← instantiateMVars owed
  if proof.hasExprMVar then
    throwErrorAt application "the frame lemma does not match the obligation"
  return (proof, target)

/-- A lemma application's conclusion at the goal, in its context: the
application translated (`LemmaApplication owes gives`). A map lemma's
conclusion is proved by its theorem, its premises by `closer`; a declared
lemma's premise is proved by `closer` and its theorem gives its conclusion
(`lemmaFacts`). The proof and the conclusion. -/
private def lemmaConclusion (application : Syntax) (closer : TSyntax `tactic) (goal : MVarId)
    (rules : BlockRules) : TacticM (Lean.Expr × Lean.Expr) := do
  let lowered ← loweredAt application
  match lowered.builtin with
  | some ``VerexIR.Proofs.Denote.StepLemmas.stored_invariants_frame =>
      frameConclusion application closer goal
  | some mapLemma =>
      -- The lemma's theorem at the arguments; its premises are owed in the
      -- leaf's form.
      let (proof, statement, premises) ← lemmaInstance goal lowered application mapLemma
      for premise in premises do
        let some premise ← normalizeFact premise none rules | continue
        unless ← proveReading application premise (evalTactic closer) do
          withSpecForm <| throwErrorAt application m!"the lemma's premise is not proved:\
            {indentD (MessageData.ofGoal premise)}"
      return (proof, statement)
  | none =>
      let translated ← goal.withContext do whnfR (← translateStep lowered application)
      unless translated.isAppOfArity ``VerexIR.Proofs.LemmaApplication 2 do
        throwErrorAt application "an `apply` names a lemma at its arguments"
      let premise ← goal.withContext (unfoldLemmaDefinitions (translated.getArg! 0))
      let (_, owed, equality?) ← proveNormal application goal premise (evalTactic closer) rules
      goal.withContext do
        let proof ← match equality? with
          | some equality => mkEqMPR equality owed
          | none => pure owed
        -- The conclusion as stated, its definitions unfolded.
        let (proof, statement) ← lemmaFacts proof translated
        return (← mkExpectedTypeHint proof statement, statement)

/-- Give a goal a lemma's conclusion, normalized once, each part of a
conjunction a fact of its own (`name`, `name_1`, …): a case-wise conclusion
gives one implication per case. -/
private partial def giveConclusion (goal : MVarId) (name : Name) (statement proof : Lean.Expr)
    (rules : BlockRules) : TacticM (Option MVarId) := do
  let (normal, equality?) ← normalForm goal statement rules
  let proof ← goal.withContext do
    match equality? with
    | some equality => castFact equality proof
    | none => pure proof
  let rec give (goal : MVarId) (name : Name) (next : Nat) (statement proof : Lean.Expr) :
      TacticM (Option MVarId) := do
    let_expr And left right := statement | giveFact goal name statement proof
    let some goal ← give goal name (next + 1) left (mkProj ``And 0 proof) | return none
    give goal (Name.mkSimple s!"{name}_{next}") (next + 1) right (mkProj ``And 1 proof)
  give goal name 1 normal proof

/-- `c ==> … ==> L(a)` at the goal, under no binder: the lemma's conclusion
under the guards, its premises owed under them; the proof and the
statement. -/
private def guardedConclusion (formula : Syntax) (closer : TSyntax `tactic) (goal : MVarId)
    (rules : BlockRules) : TacticM (Lean.Expr × Lean.Expr) := do
  let (guards, application) := formulaGuards formula
  if guards.isEmpty then return ← lemmaConclusion application closer goal rules
  let conditions ← guards.mapM fun guard => do
    let condition ← goal.withContext do translateStep (← loweredAt guard) guard
    return (← normalForm goal condition rules).1
  goal.withContext do
    let declarations := conditions.mapIdx fun index condition =>
      (Name.mkSimple s!"guard_{index}", fun (_ : Array Lean.Expr) => pure condition)
    withLocalDeclsD declarations fun held => do
      let within ← mkFreshExprSyntheticOpaqueMVar (← goal.getType)
      let (proof, statement) ← lemmaConclusion application closer within.mvarId! rules
      return (← mkLambdaFVars held (← instantiateMVars proof),
        ← mkForallFVars held (← instantiateMVars statement))

/-- `∀ (x : T; …), c ==> L(a)` at the goal (`doc/plan/proof-language.md`,
"Steps": `apply ∀ xs. ψ` gives the implication under the binders): each
binder a local of its specification type named in the step's scope, the
guarded conclusion proved under them, then closed over them; the proof and
the statement. -/
private def quantifiedConclusion (formula : Syntax) (closer : TSyntax `tactic) (goal : MVarId)
    (rules : BlockRules) : TacticM (Lean.Expr × Lean.Expr) := do
  let some (binders, body) := quantifiedFormula? formula
    | throwErrorAt formula "an `apply` names a lemma at its arguments"
  let (_, application) := formulaGuards body
  let lowered ← loweredAt application
  goal.withContext do
    let context ← stepBaseContext lowered formula
    let types ← binders.mapM fun (name, _) => do
      let some localDecl := lowered.locals.findRev? (·.name == name)
        | throwErrorAt formula m!"the binder `{name}` is not lowered"
      return (name, ← context.binderType localDecl.type.typeId,
        context.valueTypeOf? localDecl.type.typeId)
    -- A binder of a bounded integer type ranges over its type's values.
    let rangeOf (type : Option VerexIR.Ty) (x : Lean.Expr) : MetaM (Option Lean.Expr) := do
      let some (.integer (.bits width) signed) := type | return none
      let (low, high) : Int × Int := if signed then (-(2 ^ (width - 1)), 2 ^ (width - 1) - 1)
        else (0, 2 ^ width - 1)
      return some (← mkAppM ``And #[← mkAppM ``LE.le #[toExpr low, x], ← mkAppM ``LE.le #[x, toExpr high]])
    let rec bind (index : Nat) (bound : Array Lean.Expr) (entries : Array Lean.Expr) :
        TacticM (Lean.Expr × Lean.Expr) := do
      if h : index < types.size then
        let (name, binderType, type) := types[index]
        withLocalDeclD (Name.mkSimple name) binderType fun x => do
        withLocalDeclD (Name.mkSimple s!"{name}_scope")
          (← mkAppOptM ``ScopeEntry #[mkStrLit name, mkRawNatLit (← nextScopeIndex), none, x])
          fun entry => do
          match ← rangeOf type x with
          | some range =>
              withLocalDeclD (Name.mkSimple s!"{name}_range") range fun held =>
                bind (index + 1) ((bound.push x).push held) (entries.push entry)
          | none => bind (index + 1) (bound.push x) (entries.push entry)
      else
        let within ← mkFreshExprSyntheticOpaqueMVar (← goal.getType)
        let (proof, statement) ← guardedConclusion body closer within.mvarId! rules
        -- The scope entries are inert facts: their proofs are `intro`.
        let mut proof ← instantiateMVars proof
        for entry in entries.reverse do
          let type ← inferType entry
          proof := (← mkLambdaFVars #[entry] proof).beta
            #[mkAppN (mkConst ``ScopeEntry.intro type.getAppFn.constLevels!) type.getAppArgs]
        let statement ← instantiateMVars statement
        if entries.any (statement.containsFVar ·.fvarId!) then
          throwErrorAt formula "the conclusion reads a binder's scope entry"
        return (← mkLambdaFVars bound proof, ← mkForallFVars bound statement)
    bind 0 #[] #[]

/-- `apply c ==> … ==> L(a)` at the goal: the lemma's conclusion under the
guards, its premises owed under them (`lemmaConclusion`), becomes the fact
`name`. -/
def applyFormula (formula : Syntax) (closer : TSyntax `tactic) (goal : MVarId) (name : Name) :
    TacticM (Option MVarId) := do
  if (quantifiedFormula? formula).isSome then
    let rules ← blockRules goal
    let (proof, statement) ← quantifiedConclusion formula closer goal rules
    return ← giveConclusion goal name statement proof rules
  let (guards, application) := formulaGuards formula
  -- The premises and the conclusion share the goal's rules, once.
  let rules ← blockRules goal
  if guards.isEmpty then
    let (proof, statement) ← lemmaConclusion application closer goal rules
    return ← giveConclusion goal name statement proof rules
  -- The guards in the leaf's normal form, as the closer reads them.
  let conditions ← guards.mapM fun guard => do
    let condition ← goal.withContext do translateStep (← loweredAt guard) guard
    return (← normalForm goal condition rules).1
  let (proof, statement) ← goal.withContext do
    let declarations := conditions.mapIdx fun index condition =>
      (Name.mkSimple s!"guard_{index}", fun (_ : Array Lean.Expr) => pure condition)
    withLocalDeclsD declarations fun held => do
      -- The goal with the guards held.
      let within ← mkFreshExprSyntheticOpaqueMVar (← goal.getType)
      let (proof, statement) ← lemmaConclusion application closer within.mvarId! rules
      return (← mkLambdaFVars held (← instantiateMVars proof),
        ← mkForallFVars held (← instantiateMVars statement))
  giveConclusion goal name statement proof rules

/-- `apply φ, …` at each goal: each formula's conclusion a fact `fact.k.j`. -/
def applyStep (formulas : Array Syntax) (closer : TSyntax `tactic) (index : String) :
    TacticM Unit := do
  for formula in formulas, position in [0:formulas.size] do
    let name := blockFactName "fact" index position
    let mut remaining := #[]
    for goal in ← getGoals do
      if let some next ← applyFormula formula closer goal name then remaining := remaining.push next
    setGoals remaining.toList

/-- The proof a closer gives a fact's goal. A closer `apply φ, …` gives the
formulas' conclusions (`fact.k.j`), then `decide` closes the fact in the form
they give it. -/
def closerProof (closer? : Option Syntax) (index : String)
    (run? : Option (Syntax → String → TacticM (Array MVarId)) := none) :
    TacticM (TacticM Unit) := do
  let decide ← blockDecide
  match closer? with
  | some closer =>
      if closer.isOfKind ``closerContradiction then
        let some run := run? | throwErrorAt closer "a contradiction closes an `assert`"
        return do
          -- The fact's negation, a fact of the block, then the block's
          -- steps; `decide` finds `False`.
          let [refuting] ← (← getMainGoal).apply (mkConst ``Classical.byContradiction)
            | throwErrorAt closer "the fact is not refuted by contradiction"
          let name := blockFactName "contra" index
          let (negation, refuting) ← refuting.intro name
          let refuting ← markFact refuting negation
          let some refuting ← normalizeFact refuting (some name) | setGoals []
          -- Settled as any fact of the block: a term it names folds into the name.
          let some refuting ← settleFact refuting name | setGoals []
          setGoals [refuting]
          -- A step of the block that fails is reported where it stands.
          let open_ ← try run closer[1] s!"{index}c"
            catch failure =>
              let at_ := match failure with
                | .error ref _ => ref
                | .internal .. => closer
              let position := (← getFileMap).toPosition ((at_.getPos?).getD 0)
              throwErrorAt closer m!"a step of the contradiction fails at line {position.line}, \
                column {position.column + 1}: {failure.toMessageData}"
          if let some goal := open_[0]? then
            withSpecForm <| throwErrorAt closer m!"`decide` finds no contradiction in the block's facts:\
              {indentD (MessageData.ofGoal goal)}"
          setGoals []
      if closer.isOfKind ``closerApply then
        return do
          applyStep closer[1].getSepArgs decide index
          let mut open_ := #[]
          for owed in ← getGoals do
            let some owed ← normalizeFact owed none | continue
            setGoals [owed]
            evalTactic decide
            open_ := open_ ++ (← getUnsolvedGoals).toArray
          setGoals open_.toList
      let tactic ← closerTactic closer
      return evalTactic tactic
  | none => return evalTactic decide

/-- `assert e by c` at each goal: `e` translated and normalized, proved by
`c` from the goal's context, then a hypothesis `fact.k` of the goal. -/
def assertStep (step : Syntax) (index : String)
    (run : Syntax → String → TacticM (Array MVarId)) : TacticM Unit := do
  let lowered ← loweredAt step[1]
  let prove ← closerProof (if step[2].getNumArgs == 2 then some step[2][1] else none) index run
  let name := blockFactName "fact" index
  let mut remaining := #[]
  for goal in ← getGoals do
    let fact ← goal.withContext (translateStep lowered step[1])
    let contradiction := step[2].getNumArgs == 2 && step[2][1].isOfKind ``closerContradiction
    if let some next ← addFact step goal name fact prove contradiction then
      remaining := remaining.push next
  setGoals remaining.toList

/-- A `calc` step's links and conclusion. -/
private def calcChainOf (step : Syntax) : TacticM (Array Syntax × Syntax) :=
  match calcChain step[2] with
  | .ok chain => pure chain
  | .error message => throwErrorAt step[2] message

/-- `calc(e₀ op₁ e₁ …)` at each goal, a chain of `assert`s: each link proved
by the closer, a hypothesis `fact.k_j`, then the conclusion by `decide` from
them, the hypothesis `fact.k`. -/
def calcStep (step : Syntax) (index : String) : TacticM Unit := do
  let (links, conclusion) ← calcChainOf step
  let closer? := if step[4].getNumArgs == 2 then some step[4][1] else none
  if links.size == 1 then
    let lowered ← loweredAt conclusion
    let prove ← closerProof closer? index
    let name := blockFactName "fact" index
    let mut remaining := #[]
    for goal in ← getGoals do
      let fact ← goal.withContext (translateStep lowered conclusion)
      if let some next ← addFact conclusion goal name fact prove then remaining := remaining.push next
    setGoals remaining.toList
    return
  for link in links, position in [0:links.size] do
    let linkIndex := s!"{index}_{position}"
    let lowered ← loweredAt link
    let prove ← closerProof closer? linkIndex
    let mut remaining := #[]
    for goal in ← getGoals do
      let fact ← goal.withContext (translateStep lowered link)
      if let some next ← addFact link goal (blockFactName "fact" linkIndex) fact prove then
        remaining := remaining.push next
    setGoals remaining.toList
  let lowered ← loweredAt conclusion
  let decide ← blockDecide
  let name := blockFactName "fact" index
  let mut remaining := #[]
  for goal in ← getGoals do
    let fact ← goal.withContext (translateStep lowered conclusion)
    if let some next ← addFact step goal name fact (evalTactic decide) then
      remaining := remaining.push next
  setGoals remaining.toList

/-- The most cases a range split makes, as `pragma unroll` caps iterations. -/
def rangeSplitCap : Nat := 32

/-- A literal integer of a range bound. -/
private def rangeBound? (stx : Syntax) : Option Int :=
  match stx.isNatLit? with
  | some value => some value
  | none => (stx.getArgs.findSome? fun child => child.isNatLit?).map Int.ofNat

/-- The literal bounds of a range expression `a..b`. -/
private def rangeBounds (range : Syntax) : TacticM (Int × Int) := do
  let operands := range.getArgs.filter fun child => !child.isAtom
  let some lower := operands[0]? >>= rangeBound?
    | throwErrorAt range "a range split's bounds are literals, `split i in a..b`"
  let some upper := operands[1]? >>= rangeBound?
    | throwErrorAt range "a range split's bounds are literals, `split i in a..b`"
  return (lower, upper)

/-- `split i in a..b` at each goal: the name's integer value in `a..b`, owed
by `decide`, then one case per value, the case a hypothesis `split.k`. -/
def splitRangeStep (step : Syntax) (index : String) : TacticM Unit := do
  let expression := step[1]
  unless expression.isOfKind ``verexLocalExpr do
    throwErrorAt expression "a range split splits a name in scope"
  let (lower, upper) ← rangeBounds step[2][1]
  unless lower < upper && (upper - lower).toNat ≤ rangeSplitCap do
    throwErrorAt step[2] m!"a range split makes between 1 and {rangeSplitCap} cases"
  let name := blockFactName "split" index
  let mut cases := #[]
  for goal in ← getGoals do
    let value ← goal.withContext do
      let some decl := (← getLCtx).findFromUserName? expression[0].getId
        | throwErrorAt expression m!"`{expression[0].getId}` is not in scope at this obligation"
      if ← isDefEq (← whnfR (← inferType decl.toExpr)) (mkConst ``Int) then pure decl.toExpr
      else mkAppM ``VerexIR.SpecInt.val #[decl.toExpr]
    -- The bounds, owed by `decide`.
    let bounds ← goal.withContext do
      mkAppM ``And #[← mkAppM ``LE.le #[toExpr lower, value], ← mkAppM ``LT.lt #[value, toExpr upper]]
    let some bounded ← addFact step goal (blockFactName "split_bounds" index) bounds
        (evalTactic (← blockDecide)) | continue
    -- One case per value, by omega over the bounds.
    let values := (List.range (upper - lower).toNat).map fun offset => lower + Int.ofNat offset
    let equations ← bounded.withContext do values.mapM fun literal => mkEq value (toExpr literal)
    let disjunction := equations.dropLast.foldr (fun equation rest => mkOr equation rest)
      equations.getLast!
    let proof ← bounded.withContext (mkFreshExprSyntheticOpaqueMVar disjunction)
    setGoals [proof.mvarId!]
    evalTactic (← `(tactic| omega))
    let (fvar, split) ← (← bounded.assert name disjunction (← instantiateMVars proof)).intro1P
    -- The disjunction taken apart one value at a time.
    let mut produced := #[]
    let mut current := split
    let mut hypothesis := fvar
    for _ in [0:values.length - 1] do
      let subgoals ← current.cases hypothesis #[{ varNames := [name] }, { varNames := [name] }]
      let #[left, right] := subgoals | throwErrorAt step "a range split's case does not split"
      produced := produced.push left.mvarId
      current := right.mvarId
      hypothesis := right.fields[0]!.fvarId!
    produced := produced.push current
    for case in produced do
      if let some next ← normalizeFact case (some name) then
        if let some next ← settleFact next name then cases := cases.push next
  setGoals (← cases.toList.mapM reduceDefinitions)

/-- The cases a split takes, by the type of its expression. -/
private inductive SplitKind where
  | boolean
  | variant
  /-- An optional value held as a vector (`SemanticOperations.optionalLayout?`). -/
  | optional

/-- The cases of a lowered split expression, by its type: none where it has
no cases. -/
private def splitKind? (lowered : LoweredStep) : Option SplitKind := do
  let ns ← lowered.unit.namespaces[0]?
  let expression ← ns.expressions[lowered.expression.index]?
  match ← lowered.unit.tables.types[(Contract.specTypeId lowered.unit expression.typeId).index]? with
  | .bool => some .boolean
  | .nominal name _ =>
      let handle ← VerexIR.SemanticOperations.resolveNominal? lowered.unit ns name
      let target ← lowered.unit.namespaces[handle.namespaceId.index]?
      let declaration ← target.structs[handle.structId]?
      if !declaration.variants.isEmpty then some .variant
      else if VerexIR.SemanticOperations.optionalLayout? target declaration == some .vector then
        some .optional
      else none
  | _ => none

/-- A case fact that its normal form states as an existential or a
conjunction, taken apart: the witnesses locals, each part a fact of the block,
`name`, `name_1`, …, as a model value a case reads (`∃ a, o = some a ∧ …`) is
named for the facts that read it. The goal and the parts' names. -/
private partial def takeFactApart (goal : MVarId) (name : Name) (next : Nat := 1) :
    TacticM (MVarId × Array Name) := do
  let some fact ← goal.withContext do return (← getLCtx).findFromUserName? name
    | return (goal, #[])
  let type ← goal.withContext do whnfR (← instantiateMVars fact.type)
  if type.isAppOfArity ``Exists 2 then
    let witness ← mkFreshUserName `witness
    let #[case] ← goal.cases fact.fvarId #[{ varNames := [witness, name] }] | return (goal, #[name])
    return ← takeFactApart case.mvarId name next
  if type.isAppOfArity ``And 2 then
    let rest := Name.mkSimple s!"{name}_{next}"
    let #[case] ← goal.cases fact.fvarId #[{ varNames := [name, rest] }] | return (goal, #[name])
    let some restFact ← case.mvarId.withContext do return (← getLCtx).findFromUserName? rest
      | return (case.mvarId, #[name])
    let goal ← markFact case.mvarId restFact.fvarId
    let (goal, restParts) ← takeFactApart goal rest (next + 1)
    let (goal, parts) ← takeFactApart goal name (next + 1)
    return (goal, parts ++ restParts)
  return (goal, #[name])

/-- A case fact taken apart (`takeFactApart`), then, under the bounds of the
values its parts read, each part normalized and settled. The goal after,
none where it is closed. -/
private def destructureFact (goal : MVarId) (name : Name) : TacticM (Option MVarId) := do
  let (goal, parts) ← takeFactApart goal name
  if parts.size ≤ 1 then return some goal
  setGoals [goal]
  evalTactic (← `(tactic| verex_type_bounds))
  let [goal] ← getGoals | return none
  let mut goal := goal
  for part in parts do
    let some next ← normalizeFact goal (some part) | return none
    let some next ← settleFact next part | return none
    goal := next
  return some goal

/-- The hypotheses of a goal that hold a term, which a split of the term
generalizes with the goal, so that each case reads them at its value. -/
private def holding (goal : MVarId) (term : Lean.Expr) : MetaM (Array FVarId) := goal.withContext do
  (← getLCtx).foldlM (init := #[]) fun found decl => do
    if decl.isAuxDecl then return found
    let type ← instantiateMVars decl.type
    return if (type.find? (· == term)).isSome then found.push decl.fvarId else found

/-- Split a goal by the case of an optional value held as a vector
(`optionVector_cases`): the value's equation of each case substituted, a
local's in the leaf, an expression's, generalized, in the fact `name`. -/
private def splitOptional (goal : MVarId) (expression : Syntax) (term : Lean.Expr)
    (name : Name) : TacticM (Array MVarId) := do
  let (value, goal) ← match term with
    | .fvar value => pure (value, goal)
    | _ => do
        let (_, introduced, goal) ← goal.withContext do
          goal.generalizeHyp
            #[{ expr := term, xName? := some (← mkFreshUserName `value), hName? := some name }]
            (← holding goal term)
        pure (introduced[0]!, goal)
  setGoals [goal]
  let value := mkIdent (← goal.withContext do return (← value.getDecl).userName)
  withRef expression (evalTactic (← `(tactic|
    rcases VerexIR.Proofs.Denote.optionVector_cases $value with
      ⟨_, case⟩ | ⟨_, _, case⟩ | ⟨_, _, _, _, case⟩ <;> subst case)))
  evalTactic (← `(tactic| all_goals verex_normalize_context))
  let mut cases := #[]
  for case in ← getGoals do
    if let some next ← normalizeFact case (some name) then
      if let some next ← settleFact next name then cases := cases.push next
  return cases

/-- Split a goal by the variant of a value of an enum type: the value
generalized to a local and split as one (`verex_cases`), the case equation
the fact `name`. -/
private def splitVariant (goal : MVarId) (expression : Syntax) (term : Lean.Expr)
    (name : Name) : TacticM (Array MVarId) := do
  let value ← mkFreshUserName `value
  let (_, _, goal) ← goal.withContext do
    goal.generalizeHyp #[{ expr := term, xName? := some value, hName? := some name }]
      (← holding goal term)
  setGoals [goal]
  withRef expression (evalTactic (← `(tactic| verex_cases $(mkIdent value):ident)))
  evalTactic (← `(tactic| all_goals verex_canonical_families))
  evalTactic (← `(tactic| all_goals verex_normalize_context))
  let mut cases := #[]
  for case in ← getGoals do
    if let some next ← normalizeFact case (some name) then
      if let some next ← settleFact next name then
        if let some next ← destructureFact next name then cases := cases.push next
  -- A case that states what a map's model holds, such as a key's neighbour,
  -- gets the facts the map's positions give (`verex_map_positions`).
  unless (term.find? fun e => match e.getAppFn with
      | .const name _ => (`VerexIR.SpecMap).isPrefixOf name
      | _ => false).isSome do
    return cases
  setGoals cases.toList
  evalTactic (← `(tactic| all_goals verex_map_positions))
  return (← getGoals).toArray

/-- `split e` at each goal: a variant local of the leaf once per variant, its
scope entries reading the variant's value; a value of an enum type once per variant;
a Boolean once true, once false; the case a hypothesis `split.k`. -/
def splitStep (step : Syntax) (index : String) : TacticM Unit := do
  if step[2].getNumArgs > 0 then return ← splitRangeStep step index
  let expression := step[1]
  let name := blockFactName "split" index
  let lowered ← loweredAt expression
  let some kind := splitKind? lowered
    | throwErrorAt expression "a split expression is a Boolean, a value of an enum type, or an \
        optional value"
  let mut cases := #[]
  for goal in ← getGoals do
    -- The value the expression denotes at the leaf: a name's, its scope's.
    let term ← goal.withContext (translateStep lowered expression)
    match kind, term with
    | .optional, _ => cases := cases ++ (← splitOptional goal expression term name)
    | .variant, .fvar value =>
        -- A local's variants, as its preparation splits them (`verex_prepare
        -- cases`).
        setGoals [goal]
        let value := mkIdentFrom expression (← goal.withContext value.getUserName)
        withRef step (evalTactic (← `(tactic| verex_cases $value:ident)))
        evalTactic (← `(tactic| all_goals verex_canonical_families))
        evalTactic (← `(tactic| all_goals verex_normalize_context))
        cases := cases ++ (← getGoals).toArray
    | .variant, _ => cases := cases ++ (← splitVariant goal expression term name)
    | .boolean, proposition =>
        let excluded ← goal.withContext (mkAppM ``Classical.em #[proposition])
        let disjunction ← goal.withContext (inferType excluded)
        let (fvar, split) ← (← goal.assert name disjunction excluded).intro1P
        for subgoal in ← split.cases fvar #[{ varNames := [name] }, { varNames := [name] }] do
          if let some next ← normalizeFact subgoal.mvarId (some name) then
            if let some next ← settleFact next name then cases := cases.push next
  -- A case's new values and equations, such as a variant's payload, enter
  -- the definitions' normal form.
  setGoals (← (← prepareCases cases).mapM reduceDefinitions)

/-- `let V { f, … } := e` at each goal: `e is V` owed by `decide`, the fact
`fact.k`, then `e` split by its variant, the cases but `V` refuted by it; the
case equation is the fact `let.k`, which the fields read through. -/
def letVariantStep (step : Syntax) (index : String) : TacticM Unit := do
  let test := variantTest step
  let testLowered ← loweredAt test
  let valueLowered ← loweredAt step[6]
  let decide ← blockDecide
  let mut cases := #[]
  for goal in ← getGoals do
    let fact ← goal.withContext (translateStep testLowered test)
    let some goal ← addFact step goal (blockFactName "fact" index) fact (evalTactic decide)
      | continue
    let term ← goal.withContext (translateStep valueLowered step[6])
    unless (← goal.withContext (whnfD (← inferType term))).isAppOf ``Sum do
      throwErrorAt step[6] "`let V \{ … } := e` destructures a value of an enum type"
    cases := cases ++ (← splitVariant goal step[6] term (blockFactName "let" index))
  setGoals (← (← prepareCases cases).mapM reduceDefinitions)

/-- A group's conjunction, part by part. -/
private def conjunctionParts (goal : MVarId) (fuel : Nat := 64) : MetaM (List MVarId) := do
  match fuel with
  | 0 => return [goal]
  | fuel + 1 =>
    let target ← goal.withContext do whnfR (← instantiateMVars (← goal.getType))
    unless target.isAppOfArity ``And 2 do return [goal]
    let halves ← goal.apply (mkConst ``And.intro)
    halves.flatMapM (conjunctionParts · fuel)

/-- An obligation's target in the block's normal form: the block's rules,
then the denotation's normal form, whose conditional rules (a quotient's
spelling) decide by the facts the steps gave. The goal after, none where it
is closed. -/
def normalizeTarget (goal : MVarId) : TacticM (Option MVarId) := do
  let rules ← blockRules goal
  let (forward, reversed) := rules.normalRules
  let some next ← simpCached goal .rules forward reversed #[] true | return none
  normalizeFact next none

@[tactic blockCongrTactic] def evalBlockCongr : Tactic := fun _ => do
  let (_, goal) ← (← getMainGoal).intros
  let some goal ← normalizeTarget goal | replaceMainGoal []
  let mut open_ := #[]
  for part in ← conjunctionParts goal do
    let (_, part) ← part.intros
    setGoals [part]
    evalTactic (← `(tactic| lia (splits := 0)))
    open_ := open_ ++ (← getUnsolvedGoals).toArray
  setGoals open_.toList

/-- The steps and the degree the `ring` closer's Gröbner basis computation
is bounded by. -/
def ringSteps : Nat := 10000
def ringMaxDegree : Nat := 16

@[tactic blockRingTactic] def evalBlockRing : Tactic := fun _ => do
  let (_, goal) ← (← getMainGoal).intros
  let some goal ← normalizeTarget goal | replaceMainGoal []
  let steps := Syntax.mkNumLit (toString ringSteps)
  let degree := Syntax.mkNumLit (toString ringMaxDegree)
  let mut open_ := #[]
  for part in ← conjunctionParts goal do
    let (_, part) ← part.intros
    -- The identity alone: the facts in scope are hidden.
    let part ← part.withContext do
      let facts ← (← getLCtx).foldlM (init := #[]) fun found decl => do
        if decl.isImplementationDetail then return found
        return if ← isProp decl.type then found.push decl.fvarId else found
      part.tryClearMany facts
    setGoals [part]
    evalTactic (← `(tactic| grobner (splits := 0) (ematch := 0) (ringSteps := $steps)
      (ringMaxDegree := $degree)))
    open_ := open_ ++ (← getUnsolvedGoals).toArray
  setGoals open_.toList

/-- The heads of what the rules rewrite, from their simp theorems' keys: a
constant, a local, or `none` for any other key, which may match anything. -/
private def ruleHeads (forward reversed : Array FVarId) :
    TacticM (Option (NameSet × FVarIdSet)) := do
  let mut constants : NameSet := {}
  let mut locals : FVarIdSet := {}
  for (rules, inverse) in [(forward, false), (reversed, true)] do
    for rule in rules do
      for theorem_ in ← ruleTheorems rule inverse true do
        match theorem_.keys[0]? with
        | some (DiscrTree.Key.const name _) => constants := constants.insert name
        | some (DiscrTree.Key.fvar x _) => locals := locals.insert x
        | _ => return none
  return some (constants, locals)

/-- Whether a rule of these heads may rewrite a term: it holds one of the
heads, or a constant that reduces at reducible transparency, behind which a
head may stand, as the rules' keys are read (instances are no keys). -/
private def mayBeRewritten (heads : Option (NameSet × FVarIdSet)) (term : Lean.Expr) :
    MetaM Bool := do
  let some (constants, locals) := heads | return true
  if (collectFVars {} term).fvarIds.any locals.contains then return true
  for name in term.getUsedConstants do
    if constants.contains name then return true
    if (← getReducibilityStatus name) == .reducible && !(← isInstance name) then return true
  return false

/-- The leaf's own hypotheses in the block's spelling: the rules the target
is normalized with (`normalizeTarget`), the facts' and the definition folds,
so that a fact and the hypothesis it is about read alike (one spelling inside
a block). -/
def foldLeafHypotheses (goal : MVarId) : TacticM (Option MVarId) := do
  let rules ← blockRules goal
  let forward := rules.facts ++ rules.aliases ++ rules.definitionsForward
  let reversed := rules.definitions ++ rules.aliasesReversed ++ rules.factsReversed
  if forward.isEmpty && reversed.isEmpty then return some goal
  let ruled := forward ++ reversed ++ rules.reads ++ rules.booleans
  -- Only the hypotheses a rule may rewrite (`ruleHeads`): the others are in
  -- the normal form, which the rules' call does not change without a rule.
  let heads ← goal.withContext (ruleHeads forward reversed)
  let targets ← goal.withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      if decl.isImplementationDetail || ruled.contains decl.fvarId then return found
      unless ← isProp decl.type do return found
      return if ← mayBeRewritten heads (← instantiateMVars decl.type) then found.push decl.fvarId
        else found
  if targets.isEmpty then return ← foldFacts goal
  let some next ← simpCached goal .rules forward reversed targets false | return none
  -- The hypotheses the rules changed, in the normal form again: what a
  -- substituted literal simplifies (`r * 0`).
  let original ← goal.withContext getLCtx
  let changed ← next.withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      if decl.isImplementationDetail || original.contains decl.fvarId then return found
      return if ← isProp decl.type then found.push decl.fvarId else found
  let some next ← (if changed.isEmpty then pure (some next)
    else simpCached next .normal #[] #[] changed false) | return none
  -- A part the rules decided leaves its connective (`True ∧ p` is `p`) only
  -- in a changed hypothesis, which the normal form's call above settled.
  foldFacts next
where
  /-- The facts by the definitions alone: a definition the leaf states after
  a fact was given (a case split's) folds it as it folds the leaf's
  hypotheses; facts never rewrite each other here. -/
  foldFacts (goal : MVarId) : TacticM (Option MVarId) := do
    let rules ← blockRules goal
    let defining := rules.aliases ++ rules.definitionsForward
    let definingReversed := rules.definitions ++ rules.aliasesReversed
    let facts := rules.facts ++ rules.factsReversed
    if facts.isEmpty || (defining.isEmpty && definingReversed.isEmpty) then return some goal
    simpCached goal .rules defining definingReversed facts false

/-- Close the goals by `decide` in the block's normal form, their premises
introduced and a conjunction part by part. The goals left open. -/
def closeByDecide : TacticM (Array MVarId) := do
  let normalized ← (← getGoals).filterMapM fun goal => do
    let (_, goal) ← goal.intros
    let some goal ← foldLeafHypotheses goal | return none
    normalizeTarget goal
  setGoals (← normalized.flatMapM (conjunctionParts ·))
  let mut open_ := #[]
  for goal in ← getGoals do
    let (_, normal) ← goal.intros
    setGoals [normal]
    try evalTactic (← blockDecide)
    catch _ => pure ()
    open_ := open_ ++ (← getUnsolvedGoals).toArray
  setGoals open_.toList
  return open_

/-- The reads of a leaf's named maps (`x_read : ofEntries … (writeBack f x) = x`)
in the normal form, as the step's expressions read the maps: one spelling of a
typed read. The goal after, none where it is closed. -/
private def normalizeBridges (goal : MVarId) : TacticM (Option MVarId) := do
  let bridges ← goal.withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      let some (_, lhs, rhs) := (← instantiateMVars decl.type).eq? | return found
      return if rhs.isFVar && lhs.isAppOfArity ``VerexIR.SpecMap.ofEntries 7 then
        found.push decl.userName else found
  let mut goal := goal
  for bridge in bridges do
    let some next ← normalizeFact goal (some bridge) | return none
    goal := next
  return some goal

/-- A block's steps guarded by binders and a condition: a `forall` step, or a
branch of an `if` step (the condition negated in its `else`). -/
private structure GuardedBlock where
  reference : Syntax
  binders : Array (String × Syntax)
  guard : Option Syntax
  negated : Bool := false
  block : Syntax

/-- The guarded blocks of a `forall` or an `if` step, with the positions of
their facts. -/
private def guardedBlocks (step : Syntax) : Array (GuardedBlock × Option Nat) :=
  if step.isOfKind ``stepForall then
    let guard := if step[2].getNumArgs == 2 then some step[2][1] else none
    #[({ reference := step, binders := forallBinders step, guard, block := step[3] }, none)]
  else if step.isOfKind ``stepIf then
    let branch : GuardedBlock := { reference := step, binders := #[], guard := some step[1], block := step[2] }
    if step[3].getNumArgs == 2 then
      #[(branch, some 0), ({ branch with negated := true, block := step[3][1] }, some 1)]
    else #[(branch, some 0)]
  else #[]

mutual

/-- The facts a block asserts, translated in the current context: its
`assert`s, its `calc`s' conclusions, and its `forall` and `if` steps'
facts. A step's `apply`s are its means, not its facts. -/
partial def blockFacts (block : Syntax) : TacticM (Array Lean.Expr) := do
  let mut facts := #[]
  for step in blockSteps block do
    if step.isOfKind ``stepAssert then
      facts := facts.push (← translateStep (← loweredAt step[1]) step[1])
    else if step.isOfKind ``stepCalc then
      let (_, conclusion) ← calcChainOf step
      facts := facts.push (← translateStep (← loweredAt conclusion) conclusion)
    else
      for (guarded, _) in guardedBlocks step do
        if let some (statement, _) ← guardedStatement guarded then facts := facts.push statement
  return facts

/-- The fact a guarded block gives, `∀ xs, m → g → F` with `F` the
conjunction of its facts and `m` a bounded binder's range, and the number
of its hypotheses; none where it asserts nothing. -/
partial def guardedStatement (guarded : GuardedBlock) : TacticM (Option (Lean.Expr × Nat)) := do
  -- The binders' types, from an expression lowered under them.
  let lowered? ← (blockExpressions guarded.block (binders := guarded.binders) |>.toList.head?).mapM
    fun expression => loweredAt expression.expression
  let some lowered := lowered? | return none
  let context ← stepBaseContext lowered guarded.reference
  let types ← guarded.binders.mapM fun (name, _) => do
    let some localDecl := lowered.locals.findRev? (·.name == name)
      | throwErrorAt guarded.reference m!"the binder `{name}` is not lowered"
    let some type := context.valueTypeOf? localDecl.type.typeId
      | throwErrorAt guarded.reference m!"the type of `{name}` is unknown"
    return (name, ← context.binderType localDecl.type.typeId, type)
  let rec bind (index : Nat) (bound : Array Lean.Expr) (ranges : Array Lean.Expr) :
      TacticM (Option (Lean.Expr × Nat)) := do
    if h : index < types.size then
      let (name, binderType, type) := types[index]
      withLocalDeclD (Name.mkSimple name) binderType fun x => do
      withLocalDeclD (Name.mkSimple s!"{name}_scope")
        (← mkAppOptM ``ScopeEntry #[mkStrLit name, mkRawNatLit (← nextScopeIndex), none, x])
        fun _ => do
        let ranges ← match Contract.boundedMembership? type with
          | some membership => pure (ranges.push (← membership x))
          | none => pure ranges
        bind (index + 1) (bound.push x) ranges
    else
      let facts ← blockFacts guarded.block
      if facts.isEmpty then return none
      let mut body := facts.back!
      for fact in facts.pop.reverse do body := mkAnd fact body
      let mut hypotheses := ranges
      if let some guard := guarded.guard then
        let condition ← translateStep (← loweredAt guard) guard
        hypotheses := hypotheses.push (if guarded.negated then mkNot condition else condition)
      for hypothesis in hypotheses.reverse do body ← mkArrow hypothesis body
      return some (← mkForallFVars bound body, hypotheses.size)
  bind 0 #[] #[]

end

/-- Prove a guarded block's fact at a goal: its binders introduced and
scoped, its hypotheses facts of the block (`fact.k.guard_j`), its steps
run, then `decide` closes its facts. The proof. -/
def proveGuarded (goal : MVarId) (guarded : GuardedBlock) (statement : Lean.Expr)
    (hypotheses : Nat) (index : String) (runBlock : Syntax → String → TacticM Unit) :
    TacticM Lean.Expr := do
  let proof ← goal.withContext (mkFreshExprSyntheticOpaqueMVar statement)
  let (xs, inner) ← proof.mvarId!.introNP guarded.binders.size
  let mut inner ← addScopeEntries inner
    (guarded.binders.zip xs |>.map fun ((name, _), x) => (name, mkFVar x))
  let mut refuted := false
  for position in [0:hypotheses] do
    let name := Name.mkSimple s!"fact_{index}_guard_{position}"
    let (_, next) ← inner.intro name
    let some next ← normalizeFact next (some name) | refuted := true; break
    let some next ← settleFact next name | refuted := true; break
    inner := next
  unless refuted do
    setGoals [inner]
    runBlock guarded.block s!"{index}_"
    -- The block's facts as stated, in the normal form its steps gave them.
    setGoals (← (← getGoals).filterMapM fun goal => normalizeFact goal none)
    let open_ ← closeByDecide
    unless open_.isEmpty do
      withSpecForm <| throwErrorAt guarded.reference m!"`decide` does not close the block's facts:\
        {indentD (MessageData.ofGoal open_[0]!)}"
  instantiateMVars proof

/-- A `forall` or an `if` step at each goal: each guarded block's fact
(`fact.k`, an `if`'s branches `fact.k.0` and `fact.k.1`) proved under its
binders and hypotheses, then a hypothesis of the goal. -/
def guardedStep (step : Syntax) (index : String) (runBlock : Syntax → String → TacticM Unit) :
    TacticM Unit := do
  for (guarded, position) in guardedBlocks step do
    let name := blockFactName "fact" index position
    let mut remaining := #[]
    for goal in ← getGoals do
      let some (statement, hypotheses) ← goal.withContext (guardedStatement guarded)
        | remaining := remaining.push goal; continue
      let proof ← proveGuarded goal guarded statement hypotheses
        (s!"{index}{(position.map (s!".{·}")).getD ""}") runBlock
      -- Each part of a conjunction a fact of its own, as a lemma's.
      if let some next ← giveConclusion goal name statement proof (← blockRules goal) then
        remaining := remaining.push next
    setGoals remaining.toList

/-- Prove a proposition a witness owes by `decide` in the block's normal
form, reported at the witness. -/
private def witnessOwes (goal : MVarId) (owed : Lean.Expr) (witness : Syntax) (what : String) :
    TacticM Lean.Expr := do
  let proof ← goal.withContext (mkFreshExprSyntheticOpaqueMVar owed)
  if let some normal ← normalizeFact proof.mvarId! none then
    setGoals [normal]
    try evalTactic (← blockDecide) catch _ => pure ()
    unless (← getUnsolvedGoals).isEmpty do
      withSpecForm <| throwErrorAt witness m!"{what}:{indentD (MessageData.ofGoal normal)}"
  instantiateMVars proof

/-- A witness as a value of its existential binder's type `α`: as it is
where its type is `α`; where `α` is the type of a typed view of the
witness's native type, the typed value the view holds it as
(`TypedTable.typedOf`); where `α` is an integer's carrier, the integer with
its certificate, owed by `decide`; else its native value, a Boolean clause's
Boolean (`nativeOfClause`). -/
private def coerceWitness (goal : MVarId) (α value : Lean.Expr) (lowered : LoweredStep)
    (witness : Syntax) : TacticM Lean.Expr := do
  let (valueType, carrier) ← goal.withContext do return (← inferType value, ← whnfD α)
  if ← goal.withContext (isDefEq valueType α) then return value
  -- An integer at a bounded integer's carrier needs no frame: a clause's
  -- carriers are read only for the views and native values below.
  if carrier.isAppOfArity ``VerexIR.SpecInt 2 && (← goal.withContext (isDefEq valueType (mkConst ``Int))) then
    let (width, signed) := (carrier.getArg! 0, carrier.getArg! 1)
    let fits ← witnessOwes goal (mkApp3 (mkConst ``VerexIR.IntegerValueFits) width signed value) witness
      "the witness is not in its type's range"
    return mkApp4 (mkConst ``VerexIR.SpecInt.mk) width signed value fits
  let (carriers, τ) ← goal.withContext do
    let context ← stepContext lowered witness
    let some expression := context.ns.expressions[lowered.expression.index]?
      | throwErrorAt witness "the witness is not lowered"
    return (← context.carriers, ← context.nativeType expression.typeId)
  let typed? ← goal.withContext do
    let some view ← NativeMaps.typedView? carriers (← Contract.quoteNTy τ) | return none
    unless ← isDefEq view.type α do return none
    some <$> Contract.TypedTable.typedOf view τ value
  if let some typed := typed? then return typed
  let read? ← goal.withContext do
    let read ← try some <$> Contract.nativeOfClause carriers τ value catch _ => pure none
    let some read := read | return none
    return if ← isDefEq (← inferType read) α then some read else none
  if let some read := read? then return read
  throwErrorAt witness m!"the witness is a `{valueType}`, the existential binds a `{α}`"

/-- The conjuncts before a goal's next existential, each owed by `decide`: a
bounded integer binder of a clause states its range as conjuncts beside the
existential of the binder after it (`0 ≤ i ∧ i ≤ m ∧ ∃ j, …`). -/
private partial def peelToExistential (goal : MVarId) (witness : Syntax) : TacticM MVarId := do
  let target ← goal.withContext do whnfR (← instantiateMVars (← goal.getType))
  if target.isAppOf ``Exists then return goal
  let_expr And left right := target | return goal
  -- Only conjuncts in front of an existential: the clause's body is owed
  -- after the witnesses.
  unless (right.find? (·.isAppOf ``Exists)).isSome do return goal
  let owed ← goal.withContext (mkFreshExprSyntheticOpaqueMVar left)
  if let some normal ← normalizeFact owed.mvarId! none then
    setGoals [normal]
    try evalTactic (← blockDecide) catch _ => pure ()
    unless (← getUnsolvedGoals).isEmpty do
      withSpecForm <| throwErrorAt witness m!"the witness's premise is not established:\
        {indentD (MessageData.ofGoal normal)}"
  let rest ← goal.withContext (mkFreshExprSyntheticOpaqueMVar right (← goal.getTag))
  goal.assign (mkApp4 (mkConst ``And.intro) left right owed rest)
  peelToExistential rest.mvarId! witness

/-- Supply `value` for the existential of a goal: its body the goal after, a
bounded integer's range (`∃ fits : m, …`) owed by `decide`. -/
private def supplyWitness (goal : MVarId) (value : Lean.Expr) (lowered : LoweredStep)
    (witness : Syntax) : TacticM MVarId := do
  let (_, goal) ← goal.intros
  let goal ← peelToExistential goal witness
  let (α, _) ← goal.withContext do
    let target ← whnfR (← instantiateMVars (← goal.getType))
    let_expr Exists α predicate := target
      | withSpecForm <| throwErrorAt witness m!"the obligation states no existential to witness:\
          {indentD (MessageData.ofGoal goal)}"
    return (α, predicate)
  let value ← coerceWitness goal α value lowered witness
  goal.withContext do
    let target ← whnfR (← instantiateMVars (← goal.getType))
    let_expr Exists α predicate := target | throwErrorAt witness "the obligation states no existential"
    let exists_ (α predicate value body : Lean.Expr) : Lean.Expr :=
      mkApp4 (mkConst ``Exists.intro target.getAppFn.constLevels!) α predicate value body
    let body := (mkApp predicate value).headBeta
    -- A bounded integer's range, then the clause.
    let bodyType ← whnfR body
    if let some (range, inner) := match_expr bodyType with
        | Exists β p => some (β, p) | _ => none then
      if ← isProp range then
        let fits ← mkFreshExprSyntheticOpaqueMVar range
        let some owed ← normalizeFact fits.mvarId! none
          | throwErrorAt witness "the witness's range refutes the obligation"
        setGoals [owed]
        try evalTactic (← blockDecide) catch _ => pure ()
        unless (← getUnsolvedGoals).isEmpty do
          withSpecForm <| throwErrorAt witness m!"the witness is not in its type's range:\
            {indentD (MessageData.ofGoal owed)}"
        let rest ← mkFreshExprSyntheticOpaqueMVar (mkApp inner fits).headBeta (← goal.getTag)
        goal.assign (exists_ α predicate value
          (mkApp4 (mkConst ``Exists.intro bodyType.getAppFn.constLevels!) range inner fits rest))
        return rest.mvarId!
    let rest ← mkFreshExprSyntheticOpaqueMVar body (← goal.getTag)
    goal.assign (exists_ α predicate value rest)
    return rest.mvarId!

/-- `witness e, …` at each goal: the obligation's existentials instantiated at
the witnesses, in order. A step that changes the target normalizes it once,
as a substitution does. -/
def witnessStep (step : Syntax) : TacticM Unit := do
  let witnesses := step[1].getSepArgs
  let lowered ← witnesses.mapM loweredAt
  let mut remaining := #[]
  for goal in ← getGoals do
    let mut goal := goal
    for witness in witnesses, low in lowered do
      let value ← goal.withContext (translateStep low witness)
      goal ← supplyWitness goal value low witness
    if let some normal ← normalizeFact goal none then remaining := remaining.push normal
  setGoals remaining.toList

/-- The unit a block's expressions are lowered in: the module's copy. -/
private def blockUnit? (block : Syntax) : TacticM (Option ValidatedUnit) := do
  let some expression := (blockExpressions block)[0]? | return none
  return some (← loweredAt expression.expression).unit

/-- Name a leaf's binders of its clause's quantifiers, which its preparation
introduced without their names, by the clause's binder names in order. -/
private def nameBinders (goal : MVarId) (binders : Array String) : MetaM MVarId := do
  let mut goal := goal
  let mut start := 0
  for binder in binders do
    -- The binder introduced under its name, or without it.
    let candidates ← goal.withContext do
      return (← getLCtx).foldl (init := #[]) fun found decl =>
        if !decl.isImplementationDetail && decl.userName.eraseMacroScopes == Name.mkSimple binder
        then found.push decl else found
    let some decl := candidates.find? (·.index ≥ start) | continue
    if ← goal.withContext (isProp decl.type) then continue
    -- The binder's name is bound here, as its scope entry says.
    goal ← goal.rename decl.fvarId (Name.mkSimple binder)
    goal ← addScopeEntries goal #[(binder, mkFVar decl.fvarId)]
    start := decl.index + 1
  return goal

/-- Whether a block reads a map: a name it reads holds one, or a step
applies a map lemma. -/
private def blockReadsMap (block : Syntax) (names : Array (String × Contract.Domain × NTy)) :
    TacticM Bool := do
  if names.any (fun (_, _, τ) => τ matches .table ..) then return true
  for expression in blockExpressions block do
    unless expression.apply do continue
    if let some theorem_ := (← loweredAt expression.expression).builtin then
      if mapLemmas.any (·.theorem_ == theorem_) then return true
  return false

/-- Whether an obligation's statement reads a map: it holds a value of a
map's carrier. -/
private def readsMap (goal : MVarId) : MetaM Bool := goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  let mut found := false
  for fvar in (Lean.collectFVars {} target).fvarIds do
    let type := (← instantiateMVars (← fvar.getType)).consumeMData
    if type.isAppOfArity ``NTy.carrier 2 && (type.getArg! 1).isAppOf ``NTy.table then
      found := true
      break
  return found

/-- An `assert e` of a point step, the `number`-th at its point
(`doc/plan/proof-language.md`, "Points"): `e` is owed as an obligation of its
own, `assert k of p`, which the walk queues as a leaf (`.pointAssert`), and
given to the goal as the fact `name`. The goals after, and the owed ones. -/
private def pointAssertStep (step : Syntax) (point : ProgramPointKey) (number : Nat)
    (name : Name) : TacticM (Array MVarId × Array MVarId) := do
  let lowered ← loweredAt step[1]
  let key : ObligationKey := { kind := "assert", index := number, point }
  let mut remaining := #[]
  let mut owed := #[]
  for goal in ← getGoals do
    let fact ← goal.withContext (translateStep lowered step[1])
    let (normal, _) ← normalForm goal fact
    let proof ← goal.withContext (mkFreshExprSyntheticOpaqueMVar normal key.tag)
    (← targetState).pointOwed.modify (·.insert proof.mvarId! (point, number))
    owed := owed.push proof.mvarId!
    if let some next ← giveFact goal name normal proof then remaining := remaining.push next
  return (remaining, owed)

/-- Run a block's steps on every goal of its scope, its facts named after
`path` and their indices. -/
partial def runSteps (block : Syntax) (path : String := "") : TacticM Unit := do
  for step in blockSteps block, position in [0:(blockSteps block).size] do
    let index := s!"{path}{position}"
    if step.isOfKind ``stepAssert then
      assertStep step index fun block path => do runSteps block path; closeByDecide
    else if step.isOfKind ``stepLet then pure ()
    else if step.isOfKind ``stepLetVariant then letVariantStep step index
    else if step.isOfKind ``stepSplit then splitStep step index
    else if step.isOfKind ``stepApply then
      applyStep step[1].getSepArgs (← blockDecide) index
    else if step.isOfKind ``stepCalc then calcStep step index
    else if step.isOfKind ``stepForall || step.isOfKind ``stepIf then
      guardedStep step index fun block path => runSteps block path
    else if step.isOfKind ``stepAssume then
      throwErrorAt step "`assume` in a proof file is not supported yet: an assumption of an \
        obligation has no place in the target's theorem (`doc/plan/proof-language.md`, \
        \"Semantics and lowering\"); a `spec assume` in the Move proof block is an in-body \
        assumption"
    else if step.isOfKind ``stepWitness then
      unless path.isEmpty do
        throwErrorAt step "a `witness` supplies the obligation's existential: a step of its `on` block"
      witnessStep step
    else if step.isOfKind ``verexLeanStep then
      let script : TSyntax ``Lean.Parser.Tactic.tacticSeq := ⟨step[2]⟩
      evalTactic (← `(tactic| all_goals $script))
    else throwErrorAt step "this step is not supported yet"

/-- A lemma item's steps over its theorem's goal, its premise taken apart:
each parameter's binder entered under the parameter's name, the bounds of
the values in scope stated, the block's steps, then `decide` over the facts
for the conclusion. -/
@[tactic verexLemmaSteps] def evalLemmaSteps : Tactic := fun stx => do
  let pairs := stx[1].getArgs
  let block := stx[3]
  let mut entered := #[]
  for goal in ← getGoals do
    let mut goal := goal
    -- Each parameter under its name; the premise as stated is not a fact.
    let mut entries := #[]
    for pair in pairs do
      let some name := pair[0].isStrLit? | throwErrorAt pair "a parameter is named by a string"
      let decl ← goal.withContext (getLocalDeclFromUserName pair[1].getId)
      goal ← goal.rename decl.fvarId (Name.mkSimple name)
      entries := entries.push (name, decl.toExpr)
    if let some stated ← goal.withContext do
        return ((← getLCtx).findFromUserName? `statedPremise).map (·.fvarId) then
      goal ← goal.tryClear stated
    entered := entered.push (← hideEntries (← addScopeEntries goal entries))
  setGoals entered.toList
  evalTactic (← `(tactic| all_goals verex_type_bounds))
  setGoals (← (← getGoals).mapM reduceDefinitions)
  runSteps block
  let open_ ← closeByDecide
  for goal in open_ do
    withSpecForm <| logErrorAt block m!"`decide` does not close the lemma's conclusion from the \
      block's facts:{indentD (MessageData.ofGoal goal)}"
  setGoals open_.toList

/-- An `on` step's proof of one obligation (internal: `verex_step_route`
takes it): the leaf prepared once, its scope recorded, the block's steps,
then `decide` over the facts. -/
syntax (name := verexStepObligation) "verex_step_obligation" verexProofStep : tactic

/-- The goal and its hypotheses with their `Obligation` markers dropped, each
replaced by its statement, which it is by definition. -/
elab "verex_unmark" : tactic => focusGoal do
  let unmark (e : Lean.Expr) : MetaM Lean.Expr :=
    Core.transform e (post := fun e =>
      return if e.isAppOfArity ``VerexIR.Proofs.Obligation 4 then .done e.appArg! else .continue)
  let mut goal ← getMainGoal
  for decl in ← goal.withContext getLCtx do
    if decl.isImplementationDetail || decl.isLet then continue
    let type ← goal.withContext (instantiateMVars decl.type)
    unless (type.find? (·.isAppOfArity ``VerexIR.Proofs.Obligation 4)).isSome do continue
    goal ← goal.replaceLocalDeclDefEq decl.fvarId (← goal.withContext (unmark type))
  let target ← goal.withContext (instantiateMVars (← goal.getType))
  if (target.find? (·.isAppOfArity ``VerexIR.Proofs.Obligation 4)).isSome then
    goal ← goal.replaceTargetDefEq (← goal.withContext (unmark target))
  replaceMainGoal [goal]

@[tactic verexStepObligation] def evalStepObligation : Tactic := fun stx => do
  let step := stx[1]
  let block := if step.isOfKind ``stepOn then step[3] else step[2]
  let started ← IO.getNumHeartbeats
  -- The values prepared, the maps named (their size bounds and typed reads)
  -- where the block or the obligation reads a map: on a path prepared once
  -- for all its obligations (`verex_step_prepare`), only what the clause adds,
  -- its binders and premises.
  let prepared ← (← getGoals).allM fun goal => pathPrepared goal
  if prepared then
    evalTactic (← `(tactic| verex_paths))
    evalTactic (← `(tactic| all_goals verex_intro_any))
    -- The path was prepared with its markers, which name the leaves; the
    -- leaf reads its clauses unmarked, as a leaf prepared alone does, and a
    -- substitution into them leaves projections of pairs to reduce.
    evalTactic (← `(tactic| all_goals verex_unmark))
    -- The clause's premises read at the caller's family, as the path's.
    evalTactic (← `(tactic| all_goals verex_canonical_families))
    evalTactic (← `(tactic| all_goals (try simp only [VerexIR.Proofs.Denote.whichProjectFamily] at *)))
    evalTactic (← `(tactic| all_goals verex_split_hypotheses))
    evalTactic (← `(tactic| all_goals verex_subst_vars))
    evalTactic (← `(tactic| all_goals verex_reduce_projections))
    evalTactic (← `(tactic| all_goals verex_type_bounds))
    evalTactic (← `(tactic| all_goals verex_finish_prepare))
  else
    evalTactic (← `(tactic| verex_prepare values))
    evalTactic (← `(tactic| all_goals (try simp only [VerexIR.Proofs.Denote.whichProjectFamily] at *)))
  let names ← blockScope (← getFileName) block
  let (preparedAt, stepsAt, open_) ← withStepMode ({ · with blockReads := some (names.map (·.1)) }) do
    let blockReadsMaps ← blockReadsMap block names
    let environment ← getEnv
    let file ← getFileName
    let needs? : Option OnNeeds := (step.getPos?).bind fun position =>
      (onNeedsExtension.getState environment).findSome? fun (key, needs) =>
        if key == (file, position.byteIdx) then some needs else none
    -- A leaf's needs: its `on`'s keys', or, under `on _`, its own clause's.
    let leafNeeds (goal : MVarId) : TacticM (Option ClauseNeeds) := do
      match needs? with
      | some (OnNeeds.keys needs) => return some needs
      | some (OnNeeds.clauses table) =>
          let (key, _) ← leafKeyOf goal
          return (table.find? fun (clause, _) =>
            clause.kind == key.kind && clause.index == key.index && clause.callee == key.callee).map (·.2)
      | none => return none
    let mut prepared := #[]
    for goal in ← getGoals do
      -- At an `ensures` and at the data invariants a `&mut` parameter owes
      -- when its mutation ends, its value written back is its final value.
      let (key, _) ← leafKeyOf goal
      let goal ← if key.kind == "ensures" || key.kind == "data invariant" then bindFinals goal
        else pure goal
      -- An abort condition reads the entry state.
      let goal ← if key.kind == "aborts_if" || key.kind == "aborts_with" then
          match ← goal.withContext do
              return (← scopeEntries).find? (·.1 == "old_" ++ stateScopeName) with
          | some (_, entry) => bindStateEntry goal entry
          | none => pure goal
        else pure goal
      let needs ← leafNeeds goal
      -- The obligation reads a map where its clause does; a key naming no
      -- clause by its statement.
      let reads ← match needs.bind (·.reads) with
        | some reads => pure reads
        | none => readsMap goal
      let mut goals := [goal]
      if blockReadsMaps || reads then
        setGoals [goal]
        evalTactic (← `(tactic| verex_name_maps))
        goals := ← getGoals
      for goal in goals do
        let goal ← nameBinders goal ((needs.map (·.binders)).getD #[])
        let some goal ← normalizeBridges goal | continue
        -- The facts placed at points before the obligation in the normal form
        -- of its leaf, whose names the walk bound after them.
        let some goal ← renormalizeFacts goal | continue
        prepared := prepared.push (← hideEntries goal)
    setGoals prepared.toList
    setGoals (← (← getGoals).mapM reduceDefinitions)
    let preparedAt ← IO.getNumHeartbeats
    runSteps block
    let stepsAt ← IO.getNumHeartbeats
    -- Its premises introduced, `decide` reads it in the block's normal form,
    -- the conjunction of a group normalized once.
    let open_ ← closeByDecide
    return (preparedAt, stepsAt, open_)
  stageLog s!"step block at line {(← getFileMap).toPosition (step.getPos?.getD 0) |>.line}: \
    preparation {(preparedAt - started) / 1000}k, steps {(stepsAt - preparedAt) / 1000}k, \
    decide {((← IO.getNumHeartbeats) - stepsAt) / 1000}k"
  for goal in open_ do
    withSpecForm <| logErrorAt step m!"`decide` does not close the obligation from the block's facts:\
      {indentD (MessageData.ofGoal goal)}"
  setGoals open_.toList

/-- The values of a frame's slots, by declaration index: the components of an
environment literal, `none` for a slot not written. -/
private partial def frameSlots (env : Lean.Expr) : Array (Option Lean.Expr) :=
  match env.consumeMData.getAppFn, env.consumeMData.getAppArgs with
  | .const ``Prod.mk _, #[_, _, slot, rest] =>
      let slot := match slot.consumeMData.getAppFn, slot.consumeMData.getAppArgs with
        | .const ``Option.some _, #[_, value] => some value
        | _, _ => none
      #[slot] ++ frameSlots rest
  | _, _ => #[]

/-- At a program point, the locals the body has bound there, each its scope
entry under its source name, where it has none (`doc/plan/proof-language.md`,
"Obligation addressing": facts after a site name the variables in scope
there): the slots of the environment the walk carries at the point; a
mutable reference's entry holds its current value. The entries stay with the
walk, so the obligations after the point read them. -/
private def bindFrameLocals (goal : MVarId) : TacticM MVarId := goal.withContext do
  let some (names, _) ← scopeTable? | return goal
  let locals ← (← targetState).scopeLocals.get
  let target ← instantiateMVars (← goal.getType)
  let some flow := target.find? fun e =>
      e.isAppOf ``VerexIR.Proofs.Denote.Flow.value || e.isAppOf ``VerexIR.Proofs.Denote.Flow.return_
    | return goal
  let bound := (← scopeEntries).map (·.1)
  -- The slots' native types, from the frame's row: a reference's slot holds
  -- its current value and its prophecy.
  let types := Contract.quotedRowTypes (flow.getArg! 1).consumeMData
  let mut entries := #[]
  for slot in frameSlots flow.appArg!, index in [0:locals.size] do
    let some value := slot | continue
    let some name := locals[index]? | continue
    if bound.contains name || !names.contains name || value.hasLooseBVars then continue
    -- A slot holding a value already in scope is read by that value's name.
    if value.isFVar && (← scopeEntries).any (·.2 == value) then continue
    -- A native tuple ends in the unit, so a pair of one carrier is a
    -- reference's current value and prophecy.
    let type ← whnfR (← inferType value)
    let pairOfOne ← if type.isAppOfArity ``Prod 2 then isDefEq type.appFn!.appArg! type.appArg!
      else pure false
    let reference := (types[index]?).any (·.consumeMData.isAppOf ``NTy.ref) || pairOfOne
    if reference then
      entries := entries.push (name, pairProjections (← mkAppM ``Prod.fst #[value]))
      -- A local reference's prophecy, as `r'`.
      let written := name ++ "'"
      if names.contains written && !bound.contains written then
        entries := entries.push (written, pairProjections (← mkAppM ``Prod.snd #[value]))
    else entries := entries.push (name, value)
  -- Each value under a binder of the local's name with its equation, as a
  -- call's result is: a substitution eliminating the binder keeps the entry
  -- with the value.
  let mut goal := goal
  for (name, value) in entries do
    if value.isFVar then
      goal ← addScopeEntries goal #[(name, value)]
      continue
    let (named, next) ← goal.withContext do
      let type ← inferType value
      let target ← goal.getType
      let binder := Name.mkSimple name
      let motive ← withLocalDeclD binder type fun x => do
        mkForallFVars #[x] (← mkArrow (← mkEq x value) target)
      let next ← mkFreshExprSyntheticOpaqueMVar motive (← goal.getTag)
      goal.assign (mkApp2 next value (← mkEqRefl value))
      let (x, next) ← next.mvarId!.intro binder
      let (_, next) ← next.intro (Name.mkSimple s!"{name}_eq")
      return (x, next)
    goal ← addScopeEntries next #[(name, mkFVar named)]
  return goal

/-- At a program point, the memory the walk carries there is the state of
the steps at the point (`stateScopeName`): the start of the `wp` the goal
continues with. -/
private def bindPointState (goal : MVarId) : TacticM MVarId := do
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  let some wp := target.find? (·.isAppOfArity ``VerexIR.Proofs.wp 7) | return goal
  let state := wp.getArg! 6
  if state.hasLooseBVars then return goal
  bindStateEntry goal state

/-- A point step's facts (internal: the hooks of `verify f by block`'s point
steps): the block's steps run over the goal the walk
reached, whose facts are given and settled at the obligations. -/
syntax (name := verexStepPoint) "verex_step_point" num verexProofStep : tactic

@[tactic verexStepPoint] def evalStepPoint : Tactic := fun stx => do
  -- The asserts at the point before this step's.
  let offset := stx[1].isNatLit?.getD 0
  let step := stx[2]
  let point ← pointOfSyntax step[0] step[1]
  let block := step[2]
  setGoals (← (← getGoals).mapM fun goal => do hideEntries (← bindPointState (← bindFrameLocals goal)))
  withStepMode ({ · with pointFacts := true }) do
    let mut number := offset
    let mut owed := #[]
    for inner in blockSteps block, position in [0:(blockSteps block).size] do
      if inner.isOfKind ``stepAssert then
        number := number + 1
        let (remaining, more) ← pointAssertStep inner point number
          (blockFactName "fact" s!"p{position}")
        setGoals remaining.toList
        owed := owed ++ more
      else
        runSteps (block.setArg 1 (mkNullNode #[inner])) (path := s!"p{position}_")
    -- The owed obligations leave with the goals; the walk queues them.
    setGoals ((← getGoals) ++ owed.toList)

/-- The asserts of a proof's point steps: each step's point, and for each of
its asserts its number among the asserts at that point, in the proof's
order (`assert k of p`). -/
def pointAssertNumbers {m : Type → Type} [Monad m] [MonadError m] (steps : Array Syntax) :
    m (Array (Syntax × ProgramPointKey × Array (Syntax × Nat))) := do
  let mut counts : Array (String × Nat) := #[]
  let mut found := #[]
  for step in steps do
    unless step.isOfKind ``stepAt || step.isOfKind ``stepAfter do continue
    let point ← pointOfSyntax step[0] step[1]
    let mut count := (counts.find? (·.1 == point.text)).map (·.2) |>.getD 0
    let mut asserts := #[]
    for inner in blockSteps step[2] do
      if inner.isOfKind ``stepAssert then
        count := count + 1
        asserts := asserts.push (inner, count)
    counts := (counts.filter (·.1 != point.text)).push (point.text, count)
    found := found.push (step, point, asserts)
  return found

/-- Route the obligations to a block's `on` steps (internal: the lowering of
`verify f by block`), as `verex_on` routes them to its alternatives. -/
syntax (name := verexStepRoute) "verex_step_route" verexProofBlock : tactic

/-- The leaves an `on` of several keys takes, those of one path joined in one
goal (as `verex_group` joins them): prepared once, their steps run once, and
each leaf its part of the conjunction. -/
private def groupLeaves (alternatives : Array RouteAlternative) : TacticM Unit := do
  unless alternatives.any (fun alternative => alternative.items.any (·.size ≥ 2)) do return
  -- Only the goals an `on` of several keys names are taken into paths.
  let grouped (goal : MVarId) : TacticM Bool := do
    let some index ← chosenAlternative alternatives goal | return false
    return alternatives[index]!.items.any (·.size ≥ 2)
  let mut split := #[]
  for goal in ← getGoals do
    if ← grouped goal then split := split ++ (← splitPaths goal) else split := split.push goal
  setGoals split.toList
  let chosen (goal : MVarId) : TacticM (Option Nat) := chosenAlternative alternatives goal
  let keyOf (goal : MVarId) : TacticM (ObligationKey × Name) := leafKeyOf goal
  let decisionsOf ← (← targetState).leafDecisions.get
  let mut remaining := #[]
  let mut classes : Array (Nat × Array (FVarId × Lean.Expr × Option Lean.Expr) × Array MVarId) := #[]
  for goal in ← getGoals do
    let some index ← chosen goal | remaining := remaining.push goal; continue
    unless alternatives[index]!.items.any (·.size ≥ 2) do
      remaining := remaining.push goal
      continue
    let context ← goalContextKey goal
    match classes.findIdx? fun (other, otherContext, _) => other == index && otherContext == context with
    | some position => classes := classes.modify position fun (i, c, members) => (i, c, members.push goal)
    | none => classes := classes.push (index, context, #[goal])
  for (_, _, members) in classes do
    if members.size == 1 then
      remaining := remaining.push members[0]!
      continue
    let first := members[0]!
    let statements ← members.mapM fun goal => goal.withContext do instantiateMVars (← goal.getType)
    let mut statement := statements.back!
    for part in statements.pop.reverse do statement := mkAnd part statement
    let decl ← first.getDecl
    let (key, _) ← keyOf first
    let tag := Name.mkSimple s!"group_{(← first.getTag).components.head?.getD .anonymous}"
    let combined ← mkFreshExprMVarAt decl.lctx decl.localInstances statement .syntheticOpaque tag
    for goal in members, index in [0:members.size] do
      let mut part := combined
      for _ in [0:index] do part := .proj ``And 1 part
      if index + 1 < members.size then part := .proj ``And 0 part
      goal.assign part
    (← targetState).leafKeys.modify (·.insert tag key)
    (← targetState).leafDecisions.modify (·.insert combined.mvarId! (decisionsOf[first]?.getD #[]))
    remaining := remaining.push combined.mvarId!
  setGoals remaining.toList

@[tactic verexStepRoute] def evalStepRoute : Tactic := fun stx => do
  -- Point steps are hooks of the obligations' walk (`lowerProofBlock`).
  let routed := (blockSteps stx[1]).filter fun step =>
    !step.isOfKind ``stepAt && !step.isOfKind ``stepAfter
  let alternatives ← routed.mapM fun step => do
    let steps := Syntax.node (SourceInfo.fromRef step) ``verexStepObligation
      #[mkAtomFrom step "verex_step_obligation", step]
    if step.isOfKind ``stepOn then
      let items ← step[1].getSepArgs.mapM fun key => Sum.inl <$> keyPattern key
      let selector ← if step[2].getNumArgs == 2 then step[2][1].getSepArgs.mapM decisionPattern
        else pure #[]
      return ({ items := some items, selector, reference := step, steps } : RouteAlternative)
    else if step.isOfKind ``stepOnDefault then
      return { items := none, selector := #[], reference := step, steps }
    else throwErrorAt step "at the top of a proof, a step is an `on`, `at` or `after` block or \
      `lean =>`"
  -- A point's `assert e by c` is shorthand for `on assert k of p { assert e by c }`,
  -- `assert e` for its `decide`; a failure reports at the assert.
  let mut implicit := #[]
  for (pointStep, point, asserts) in ← pointAssertNumbers (blockSteps stx[1]) do
    for (inner, number) in asserts do
      let block := pointStep[2].setArg 1 (mkNullNode #[inner])
      let on := Syntax.node (SourceInfo.fromRef inner) ``stepOn
        #[mkAtomFrom inner "on", mkNullNode, mkNullNode, block]
      let steps := Syntax.node (SourceInfo.fromRef inner) ``verexStepObligation
        #[mkAtomFrom inner "verex_step_obligation", on]
      let key : ObligationKey := { kind := "assert", index := number, point }
      let alternative : RouteAlternative :=
        { items := some #[Sum.inl key], selector := #[], reference := inner, steps }
      implicit := implicit.push alternative
  let alternatives := implicit ++ alternatives
  let sinceProof := do return (← IO.getNumHeartbeats) - (← readThe Core.Context).initHeartbeats
  stageLog s!"route: walk and introduction {(← sinceProof) / 1000}k"
  groupLeaves alternatives
  routeLeaves alternatives
  stageLog s!"route: done {(← sinceProof) / 1000}k"

/-- The tactics of a block's steps. A block of one `lean =>` step is its
script; any other routes the obligations to its `on` blocks. -/
def lowerProofBlock [Monad m] [MonadQuotation m] [MonadError m] (block : Syntax) :
    m (TSyntax ``Lean.Parser.Tactic.tacticSeq) := do
  let steps := blockSteps block
  if let #[step] := steps then
    if step.isOfKind ``verexLeanStep then return ⟨step[2]⟩
  let route := Syntax.node (SourceInfo.fromRef block) ``verexStepRoute
    #[mkAtomFrom block "verex_step_route", block]
  -- Each point step a hook of the walk, `at exit` the end of every path,
  -- with the number of the asserts at its point before its own.
  let mut hooks : Array (TSyntax ``verexPointHook) := #[]
  let mut exits : Array (TSyntax `tactic) := #[]
  let numbered ← pointAssertNumbers steps
  for step in steps do
    let offset := ((numbered.find? (·.1 == step)).bind fun (_, _, asserts) =>
      asserts[0]?.map (·.2 - 1)).getD 0
    let facts : TSyntax `tactic := ⟨Syntax.node (SourceInfo.fromRef step) ``verexStepPoint
      #[mkAtomFrom step "verex_step_point", Syntax.mkNumLit (toString offset), step]⟩
    let run ← `(Lean.Parser.Tactic.tacticSeq| $facts:tactic)
    if step.isOfKind ``stepAt then
      if step[1].isOfKind ``pointExit then exits := exits.push facts
      else hooks := hooks.push (← `(verexPointHook| at $(⟨step[1]⟩):verexPoint => $run))
    else if step.isOfKind ``stepAfter then
      hooks := hooks.push (← `(verexPointHook| after $(⟨step[1]⟩):verexSite => $run))
  -- A proof with obligation blocks prepares each completed path once, before
  -- its clauses split, where its blocks take one of them.
  let mut routing : Array (TSyntax `tactic) := #[]
  if steps.any (fun step => step.isOfKind ``stepOn || step.isOfKind ``stepOnDefault) then
    exits := #[← `(tactic| verex_step_prepare)] ++ exits
    let keys : Array (TSyntax `verexKey) := steps.flatMap fun step =>
      if step.isOfKind ``stepOn then step[1].getSepArgs.map (⟨·⟩) else #[]
    routing := #[← if steps.any (·.isOfKind ``stepOnDefault) then
        `(tactic| verex_step_routing all $keys,*)
      else `(tactic| verex_step_routing $keys,*)]
  let obligations ← if exits.isEmpty then `(tactic| verex_obligations $hooks:verexPointHook*)
    else `(tactic| verex_obligations $hooks:verexPointHook* preparing $exits*)
  let tactics : Array (TSyntax `tactic) := routing ++ #[obligations, ⟨route⟩]
  `(Lean.Parser.Tactic.tacticSeq| $tactics*)

end VerexSL.Verify
