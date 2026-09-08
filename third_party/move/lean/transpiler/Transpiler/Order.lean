-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Transpiler.Xast
import Transpiler.Names

/-!
# Declaration order

Lean elaborates a `module` block in order, while Move accepts declarations in
any order.  This module computes a dependency-respecting order of a module's
items, keeping Move source order among independent declarations:

1. constants;
2. structs and enums in dependency order (a struct after the structs its
   fields mention);
3. spec functions in dependency order (a spec function after the spec
   functions it calls);
4. functions in call-graph order; a strongly connected component of the
   same-module call graph becomes one `mutual … end` group, whose members are
   recursive;
5. global invariants.

Items of kind 2 and 3 are ordered by a stable topological sort (Kahn's
algorithm seeded in source order); item kind 4 by Tarjan's algorithm over the
condensation, emitting components in dependency order and, among independent
components, by the source position of their first member.
-/

namespace Transpiler.Order

open Transpiler.Xast Transpiler.Names

/-- An item of the ordered module. -/
inductive Item where
  | constant (c : Constant)
  | struct (s : Struct)
  | specFun (f : SpecFun)
  /-- One function, or a mutually recursive group; `recursive` says whether
  the group's members call into the group (a singleton may be self-recursive). -/
  | functions (group : List Function) (recursive : Bool)
  /-- The `spec` of a function, a separate item: it follows its function
  and the spec functions it uses (which may themselves call the function). -/
  | spec (f : Function)
  | invariant (inv : Invariant)
  | specVar (v : SpecVar)
  deriving Inhabited

/-- The key of an item in the dependency order. -/
def Item.key : Item → String
  | .constant c => s!"constant:{c.name}"
  | .struct s => s!"struct:{s.name}"
  | .specFun f => s!"specfun:{f.name}"
  | .functions group _ => s!"fun:{(group.head?.map (·.name)).getD ""}"
  | .spec f => s!"spec:{f.name}"
  | .invariant inv => s!"invariant:{inv.loc.file}:{inv.loc.start}"
  | .specVar v => s!"specvar:{v.name}"

/-- Stable topological order of `items` by `deps`: repeatedly emit the first
item (in input order) whose dependencies are all emitted; a cycle is broken
by emitting the first remaining item. -/
partial def stableTopo {α : Type} [Inhabited α] (items : List α) (key : α → String)
    (deps : α → List String) : List α :=
  let rec go (remaining : List α) (done : List String) (acc : List α) : List α :=
    match remaining with
    | [] => acc.reverse
    | _ =>
      let ready := remaining.find? fun item =>
        (deps item).all fun d => done.contains d || d == key item
      let next := ready.getD remaining.head!
      go (remaining.filter fun item => key item != key next) (key next :: done) (next :: acc)
  go items [] []

/-- The same-module struct names a type mentions. -/
partial def structDeps (current : ModuleRef) : Ty → List String
  | .struct name args =>
    (if name.module == current then [name.name] else []) ++ args.flatMap (structDeps current)
  | .tuple ts => ts.flatMap (structDeps current)
  | .vector t => structDeps current t
  | .function args result _ => structDeps current args ++ structDeps current result
  | .reference _ t => structDeps current t
  | .typeDomain t => structDeps current t
  | .resourceDomain name args =>
    (if name.module == current then [name.name] else []) ++
      (args.getD []).flatMap (structDeps current)
  | _ => []

/-- The same-module functions resp. spec functions an expression calls. -/
partial def calls (current : ModuleRef) (spec : Bool) (e : Exp) : List String :=
  let sub (es : List Exp) : List String := es.flatMap (calls current spec)
  match e.node with
  | .invoke function args => calls current spec function ++ sub args
  | .call op _ args _ =>
    let own := match op with
      | .moveFunction name => if !spec && name.module == current then [name.name] else []
      | .specFunction name _ => if spec && name.module == current then [name.name] else []
      | _ => []
    own ++ sub args
  | .block _ binding body => (binding.map (calls current spec) |>.getD []) ++ calls current spec body
  | .ite c t f => sub [c, t, f]
  | .«match» s arms =>
    calls current spec s ++ arms.flatMap fun arm =>
      (arm.guard.map (calls current spec) |>.getD []) ++ calls current spec arm.body
  | .sequence es => sub es
  | .loop body => calls current spec body
  | .«return» v => calls current spec v
  | .assign _ v => calls current spec v
  | .mutate t v => sub [t, v]
  | .quant _ ranges _ cond body =>
    ranges.flatMap (fun r => calls current spec r.domain) ++
      (cond.map (calls current spec) |>.getD []) ++ calls current spec body
  | _ => []

/-- Strongly connected components of a graph on `nodes` (in input order),
emitted in dependency order (a component after the components it calls),
ties broken by the input position of the component's first member. -/
partial def sccs (nodes : List String) (edges : String → List String) : List (List String) :=
  -- Tarjan's algorithm; the output of Tarjan is in reverse topological order
  -- of the condensation, i.e. dependencies first, which is what we need.
  let index := fun (n : String) => nodes.idxOf n
  let rec visit (v : String) (st : List String × List (String × Nat × Nat) × Nat × List (List String))
      : List String × List (String × Nat × Nat) × Nat × List (List String) :=
    -- st = (stack, info (node, index, lowlink), counter, components)
    let (stack, info, counter, comps) := st
    let info := (v, counter, counter) :: info
    let stack := v :: stack
    let counter := counter + 1
    let st := (edges v).foldl (fun (stack, info, counter, comps) w =>
      if nodes.contains w then
        match info.find? fun (n, _, _) => n == w with
        | none =>
          let (stack, info, counter, comps) := visit w (stack, info, counter, comps)
          let wl := (info.find? fun (n, _, _) => n == w).map (·.2.2) |>.getD 0
          let info := info.map fun (n, i, l) => if n == v then (n, i, min l wl) else (n, i, l)
          (stack, info, counter, comps)
        | some (_, wi, _) =>
          if stack.contains w then
            let info := info.map fun (n, i, l) => if n == v then (n, i, min l wi) else (n, i, l)
            (stack, info, counter, comps)
          else (stack, info, counter, comps)
      else (stack, info, counter, comps)) (stack, info, counter, comps)
    let (stack, info, counter, comps) := st
    match info.find? fun (n, _, _) => n == v with
    | some (_, vi, vl) =>
      if vi == vl then
        -- pop the component
        let rec pop (stack : List String) (acc : List String) : List String × List String :=
          match stack with
          | [] => (acc, [])
          | w :: rest => if w == v then (w :: acc, rest) else pop rest (w :: acc)
        let (comp, stack) := pop stack []
        let comp := comp.toArray.qsort (fun a b => index a < index b) |>.toList
        (stack, info, counter, comps ++ [comp])
      else (stack, info, counter, comps)
    | none => (stack, info, counter, comps)
  let (_, _, _, comps) := nodes.foldl (fun st v =>
    let (_, info, _, _) := st
    if info.any fun (n, _, _) => n == v then st else visit v st) ([], [], 0, [])
  comps

/-- The ordered items of a module. -/
def order (m : Module) : List Item :=
  let current := m.ref
  let structs := stableTopo m.structs (·.name) fun s =>
    (s.fields.flatMap fun f => structDeps current f.ty) ++
      ((s.variants.getD []).flatMap fun v => v.fields.flatMap fun f => structDeps current f.ty)
  let specFuns := stableTopo m.specFuns (·.name) fun f =>
    f.body.map (calls current true) |>.getD []
  let funNames := m.functions.map (·.name)
  let callees (name : String) : List String :=
    match m.functions.find? (·.name == name) with
    | some f => (f.body.map (calls current false) |>.getD []).eraseDups
    | none => []
  let groups := sccs funNames callees
  let groupOf (name : String) : String :=
    (groups.find? (·.contains name)).bind (·.head?) |>.getD name
  let functionItems := groups.flatMap fun group =>
    let fs := group.filterMap fun n => m.functions.find? (·.name == n)
    let recursive := group.any fun n => (callees n).any group.contains
    Item.functions fs recursive :: fs.map Item.spec
  -- Spec functions may call functions, and a function's `spec` uses spec
  -- functions: one stable order over the items, keeping source order among
  -- independent declarations (a `spec` stays right after its function
  -- unless a spec function it needs comes later).
  let specFunCalls (body : Exp) : List String :=
    (calls current true body).map (s!"specfun:{·}") ++
      (calls current false body).map fun n => s!"fun:{groupOf n}"
  -- A companion spec function (`$f`, printed as `spec fun f`) follows the
  -- Move function `f` it is the specification version of.
  let companionDeps (f : SpecFun) : List String :=
    if f.isMoveFun then
      match companionFunction? f.name with
      | some base => [s!"fun:{groupOf base}"]
      | none => []
    else []
  let itemDeps : Item → List String
    | .specFun f => (f.body.map specFunCalls |>.getD []) ++ companionDeps f
    | .spec f =>
      s!"fun:{groupOf f.name}" ::
        (f.spec.conditions.flatMap fun c => specFunCalls c.exp)
    | _ => []
  let ordered := stableTopo (specFuns.map Item.specFun ++ functionItems) Item.key itemDeps
  m.constants.map Item.constant ++ m.specVars.map Item.specVar ++ structs.map Item.struct ++
    ordered ++ m.invariants.map Item.invariant

end Transpiler.Order
