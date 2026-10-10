-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Transpiler.Xast
import Transpiler.Names

/-!
# Transitional printer effect view

Leaner marks effectful functions with an `Action` result and has an explicit
frame (`modifies`); Move has neither. The fixed-point analysis now consumes
validated LIR in `Transpiler.LIR.Effects`. This module retains only the
XAST-shaped fact table and render-shape helpers used by the established
printer while that printer is migrated:

- `isAction f`: whether `f` prints with an `Action` result — its body uses an
  `Action`-requiring operation (global storage, a borrow that is not rendered
  as a pure vector update, dereference, reference write, `freeze`, explicit
  `abort`/`assert!`, a vector mutation through a reference) or calls an
  `Action` function; entry functions are always `Action`;
- `writesGlobal f`: whether `f` (transitively) writes global memory —
  `borrow_global_mut`, `move_to`, `move_from`, or a call to a writer — which
  decides the `modifies` rendering.

The syntactic predicate must agree with what `Transpiler.Print` renders: a
borrow counts as pure exactly in the cases the printer turns into a pure value
update (`&mut v` passed to `vector::push_back`, `&v` passed to `vector::length`
/ `is_empty`, `*vector::borrow(&v, i)` as `v.get i`).
-/

namespace Transpiler.Effects

open Transpiler.Xast Transpiler.Names

/-- The package under transpilation: every module by reference. -/
structure Package where
  modules : List Module

namespace Package

def find? (p : Package) (m : ModuleRef) : Option Module :=
  p.modules.find? fun mod => mod.ref == m

def function? (p : Package) (name : QualifiedName) : Option Function := do
  let mod ← p.find? name.module
  mod.functions.find? fun f => f.name == name.name

def struct? (p : Package) (name : QualifiedName) : Option Struct := do
  let mod ← p.find? name.module
  mod.structs.find? fun s => s.name == name.name

/-- Whether a struct carries a data invariant: Leaner certifies its values,
and creating one is an `Action` (`T.certify`), where the invariant is owed. -/
def certified (p : Package) (name : QualifiedName) : Bool :=
  (p.struct? name).any fun s => s.spec.conditions.any fun c => c.kind == .structInvariant

end Package

/-- A `std::vector` call with its operation, if the callee is one. -/
def vectorCall? (op : Operation) : Option VecOp :=
  match op with
  | .moveFunction name =>
    if isPrimitiveModule name.module && name.module.name == "vector" then vecOp name.name
    else none
  | _ => none

/-- Whether an expression is a borrow of a local variable or parameter
(`&x` / `&mut x` on a name), the shape the pure vector renderings accept. -/
def isLocalBorrow (e : Exp) : Bool :=
  match e.node with
  | .call (.borrow _) _ [arg] _ =>
    match arg.node with
    | .«local» _ | .param _ => true
    | _ => false
  | _ => false

/-- The effect facts of one function. -/
structure Facts where
  isAction : Bool := false
  writesGlobal : Bool := false
  deriving BEq, Repr, Inhabited

/-- Effect facts of every function of the package, keyed by qualified name. -/
abbrev Table := List (QualifiedName × Facts)

def Table.get (t : Table) (name : QualifiedName) : Facts :=
  (t.find? fun (n, _) => n == name).map (·.2) |>.getD {}

end Transpiler.Effects
