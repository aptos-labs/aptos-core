-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# A rendered specification call keeps the instantiation of its model

A specification call's type arguments are inferred from its arguments'
specification types, where the projection of a specification value has already
turned a fixed-width integer into the mathematical one, dropped a reference and
turned a slice into a vector. The instantiation decides the type tag of an
uninterpreted application and the carrier its argument is handed as, so a
rendering spells an instantiation the projection changes and leaves a type
argument the projection keeps — a type parameter, a vector, a nominal type, a
function value — to the ordinary omission policy.

The first section observes the tag and the carrier an instantiation produces.
The second renders units, re-elaborates the rendering, and compares each
specification call of the model with the one at the same place of the rendered
source: the cases below are the shapes these checks cover, not a proof that
rendering is lossless in general. Among them is a call whose binder occurs under
a vector in the signature while its argument carries another width: an
occurrence there is where inference may read a type, not evidence that it reads
the recorded one.
-/

namespace VerexSL.Tests.RenderedInstantiations

open Lean Elab Command VerexIR VerexIR.Validation

/-! ## The tag and the carrier an instantiation produces -/

verex module 0x42::instantiation_carriers where
  opaque spec fun serialize {MoveValue}(v : MoveValue) : Vector<u8>

  fun declared(x : u64) -> u64 := x
  spec declared where
    aborts_if false
    ensures serialize::<u64>(x).length == serialize::<u64>(result).length

  fun mathematical(x : u64) -> u64 := x
  spec mathematical where
    aborts_if false
    ensures serialize(x).length == serialize(result).length

verify 0x42::instantiation_carriers::declared
verify 0x42::instantiation_carriers::mathematical

/-- The contract a verified function published, as the term it is. -/
private def contractText (base : Name) : CommandElabM String := do
  let some info := (← getEnv).find? (base ++ `typedContract)
    | throwError "`{base}` published no contract"
  let some value := info.value? | throwError "the contract of `{base}` has no value"
  liftTermElabM do pure (toString (← Meta.ppExpr value))

-- The declared width and the mathematical integer are two applications of the
-- uninterpreted function, differing in the type tag and in the carrier at
-- once: the declared width indexes the application by `NTy.int 64 false` and
-- hands the argument through, while the mathematical integer indexes it by
-- `NTy.num` and rebuilds the argument as an unbounded carrier.
run_cmd do
  let declared ← contractText `«0x42».instantiation_carriers.declared
  unless declared.contains "NTy.int 64 false)]" do
    throwError "a declared width does not index its application:\n{declared}"
  if declared.contains "NTy.num" then
    throwError "a declared width collapsed to the mathematical integer:\n{declared}"
  if declared.contains "val :=" then
    throwError "a declared width rebuilt its carrier:\n{declared}"
  let mathematical ← contractText `«0x42».instantiation_carriers.mathematical
  unless mathematical.contains "NTy.num]" do
    throwError "a mathematical integer does not index its application:\n{mathematical}"
  unless mathematical.contains "{ val := arguments.fst.val" do
    throwError "a mathematical integer does not rebuild its carrier:\n{mathematical}"

/-! ## The round trip -/

/-- A unit compiled from source text, without registering it. The text may
carry the preamble a rendering begins with. -/
private def compileText (text : String) : CommandElabM ValidatedUnit := do
  let env ← getEnv
  let source ← match Print.Layout.namespaceStart text with
    | .ok (command, _, _) => pure command
    | .error message => throwError message
  let stx ← match Parser.runParserCategory env `command source "<rendered>" with
    | .ok stx => pure stx
    | .error message => throwError message
  let (stx, aliases) ← match canonicalMoveCommand env stx with
    | .ok canonical => pure canonical
    | .error (_, message) => throwError message
  let unit ← match compilationUnitOfSyntax stx "<rendered>" with
    | .ok unit => pure { unit with namespaces := unit.namespaces.map ({ · with aliases }) }
    | .error (_, message) => throwError message
  match compile unit with
  | .ok unit => pure unit
  | .error error => throwError "the source did not compile: {repr error}"

/-- The shape of a type of a unit, which is what the two units of a round trip
can be compared at: their type arenas number the same type differently. -/
private partial def typeShape (unit : ValidatedUnit) (typeId : TypeId) (fuel : Nat) : String :=
  if fuel == 0 then "…" else
  let shapes (ids : Array TypeId) : String :=
    String.intercalate ", " (ids.toList.map (typeShape unit · (fuel - 1)))
  match unit.tables.types[typeId.index]? with
  | some (.integer .unbounded true) => "num"
  | some (.integer .unbounded false) => "unbounded unsigned"
  | some (.integer .pointer signed) => if signed then "isize" else "usize"
  | some (.integer (.bits bits) signed) => s!"{if signed then "i" else "u"}{bits}"
  | some (.vector element _) => s!"Vector<{typeShape unit element (fuel - 1)}>"
  | some (.slice element) => s!"Slice<{typeShape unit element (fuel - 1)}>"
  | some (.reference reference) => s!"&{typeShape unit reference.referent (fuel - 1)}"
  | some (.tuple elements) => s!"({shapes elements})"
  | some (.typeParameter index) => s!"#{index}"
  | some (.nominal name arguments) =>
      let shown := (unit.tables.names[name.index]?).map (·.name) |>.getD "?"
      let given := arguments.filterMap fun
        | .typeArg value => some value.typeId
        | _ => none
      if given.isEmpty then shown else s!"{shown}<{shapes given}>"
  | some (.function arguments result _) =>
      s!"Fn({shapes arguments}) -> {typeShape unit result (fuel - 1)}"
  | some other => (repr other).pretty
  | none => "?"

/-- One specification call, as the callee and the instantiation it holds. -/
private def callText (unit : ValidatedUnit) (reference : QualifiedRef)
    (instantiations : Array GenericArgument) : String :=
  let fuel := unit.tables.types.size + 1
  let name := (unit.tables.names[reference.name.index]?).map (·.name) |>.getD "?"
  let given := instantiations.filterMap fun
    | .typeArg value => some value.typeId
    | _ => none
  s!"{name}<{String.intercalate ", " (given.toList.map (typeShape unit · fuel))}>"

/-- The specification calls reachable from an expression, the call before its
own operands, so that a call's place in this sequence is its position in the
expression. -/
private partial def callsIn (unit : ValidatedUnit) (ns : ValidatedNamespace) (fuel : Nat)
    (id : ExprId) : Array String :=
  if fuel == 0 then #[] else
  match ns.expressions[id.index]? with
  | none => #[]
  | some expression =>
      let operands := (expressionChildren expression.kind).flatMap (callsIn unit ns (fuel - 1) ·)
      match expression.kind with
      | .operation (.specification (.functionCall reference _)) instantiations _ =>
          #[callText unit reference instantiations] ++ operands
      | _ => operands

/-- Every specification call of a unit at the place it stands: its enclosing
declaration, the condition or body it belongs to, and its position in that
expression. Comparing the places, and not a multiset of call texts, is what
tells a preserved instantiation from two calls that exchanged theirs. -/
private def specificationCalls (unit : ValidatedUnit) : Array String :=
  unit.namespaces.flatMap fun ns =>
    let ns := ns.inUnitIds
    let fuel := ns.expressions.size + 1
    let placed (owner : String) (roots : Array (String × ExprId)) : Array String :=
      roots.flatMap fun (where_, root) =>
        (callsIn unit ns fuel root).zipIdx.map fun (call, position) =>
          s!"{owner}/{where_}#{position}: {call}"
    let named (id : NameId) : String := (unit.tables.names[id.index]?).map (·.name) |>.getD "?"
    let conditions (contract : VerexIR.FunctionContract) : Array (String × ExprId) :=
      contract.conditions.zipIdx.map fun (condition, index) =>
        (s!"condition {index}", condition.expression)
    ns.functions.flatMap (fun declaration =>
        placed (named declaration.name) (conditions declaration.contract)) ++
      ns.specFunctions.flatMap fun declaration =>
        placed (named declaration.name)
          (conditions declaration.contract ++
            (declaration.body.map (fun root => ("body", root))).toArray)

/-- Render the unit of `source` and re-elaborate the rendering: every
specification call must come back, at the same place, with the instantiation
the model holds. `spelled` are the call texts the rendering must contain,
`implicit` the calls it must spell without a type-argument suffix. -/
private def roundTrip (source : String) (spelled implicit : Array String) :
    CommandElabM Unit := do
  let env ← getEnv
  let unit ← compileText source
  let .ok rendered := Print.render env unit | throwError "the unit did not render"
  let again ← compileText rendered
  let model := specificationCalls unit
  let reelaborated := specificationCalls again
  unless model == reelaborated do
    throwError "rendering changed the specification calls\n  model: {model}\n  \
      rendered: {reelaborated}\n{rendered}"
  -- An implicit call's text holds its opening parenthesis, so a type
  -- argument printed where re-elaboration recovers it fails the check.
  for text in spelled ++ implicit do
    unless rendered.contains text do
      throwError "the rendering does not spell `{text}`:\n{rendered}"

/-- The scalar widths, a truncating and a widening cast, a field read, `old` of
a field, a genuine mathematical sum, a type parameter, a direct tuple, a
vector- and a nominal-bearing argument, a binder under a reference at two
widths, and a binder occurring both directly and through a vector: once with
the argument's own width, once with another one. -/
private def moveSource : String :=
"verex module 0x42::rendered_instantiations where
  pragma verify = false

  opaque spec fun ser {MoveValue}(v : MoveValue) : Vector<u8>
  opaque spec fun both {T}(a : T, b : Vector<T>) : Vector<u8>
  opaque spec fun compare {T}(first : &T, second : &T) : u64

  struct S has Copy, Drop, Store where
    a : u64
    b : u8

  fun widths(x : u8, y : u64, z : u128) -> Unit := ()
  spec widths where
    ensures ser::<u8>(x).length == 1 && ser::<u64>(y).length == 2
      && ser::<u128>(z).length == 3

  fun casts(x : u64) -> Unit := ()
  spec casts where
    ensures ser::<u8>((x as u8)).length == 1 && ser::<u128>((x as u128)).length == 2

  fun fields(s : S, counter : &mut S) -> Unit := ()
  spec fields where
    ensures ser::<u64>(s.a).length == 1 && ser::<u64>(old(counter.a)).length == 2

  fun mathematical(x : u64, y : u64) -> Unit := ()
  spec mathematical where
    ensures ser::<Int>(x + y).length == 1

  fun parameter {T}(v : T) -> Unit := ()
  spec parameter where
    ensures ser::<T>(v).length == 1

  fun tuples(x : u64, y : u8) -> Unit := ()
  spec tuples where
    ensures ser::<(u64, u8)>((x, y)).length == 1

  fun containers(v : Vector<u64>, s : S) -> Unit := ()
  spec containers where
    ensures ser::<Vector<u64> >(v).length == 1 && ser::<S>(s).length == 2

  fun compared(a : u64, b : u64, c : u256, d : u256) -> Unit := ()
  spec compared where
    ensures compare::<u64>(a, b) == 1 && compare::<u256>(c, d) == 2

  fun repeated(x : u64, v : Vector<u64>) -> Unit := ()
  spec repeated where
    ensures both::<u64>(x, v).length == 1

  fun repeated_other_width(x : u64, v : Vector<u64>) -> Unit := ()
  spec repeated_other_width where
    ensures both::<u8>(x, v).length == 1
"

run_cmd do
  roundTrip moveSource
    (spelled := #["ser::<u8>(x)", "ser::<u64>(y)", "ser::<u128>(z)", "ser::<u64>(s.a)",
      "ser::<u64>(old(counter.a))", "ser::<(u64, u8)>((x, y))", "compare::<u64>(a, b)",
      "compare::<u256>(c, d)", "both::<u64>(x, v)", "both::<u8>(x, v)"])
    (implicit := #["ser(x + y)", "ser(v)", "ser(s)"])

/-- A reference instantiation, which the Move profile rejects, and a
function-bearing one, which the projection leaves alone. A slice
instantiation has no case: a by-value type argument must be sized, so neither
profile admits one. -/
private def rustSource : String :=
"verex namespace rendered_instantiations_rust using rust where
  pragma verify = false

  opaque spec fun ser {T}(v : T) : Vector<u8>

  fun borrowed(r : &u64) -> Unit := ()
  spec borrowed where
    ensures ser::<&u64>(r).length == 1

  fun callable(f : Fn(u64) -> u64 has Copy) -> Unit := ()
  spec callable where
    ensures ser::<Fn(u64) -> u64>(f).length == 2
"

run_cmd do
  roundTrip rustSource (spelled := #["ser::<&u64>(r)"]) (implicit := #["ser(f)"])

end VerexSL.Tests.RenderedInstantiations
