-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Validation.Validated

/-!
# Closure templates

What site discovery reads of a namespace, from the namespace alone and in its
own identifiers: the calls and closures of each function body, with their
instantiations, and the closures without instantiation anywhere in the
namespace. A unit's closure sites are the join of its namespaces' templates
(`closureSites`): the references and instantiations mapped through each
namespace's links, the instances reached from the closed functions, and each
site's rows computed after the mapping.
-/

namespace VerexIR.Validation

/-- A call or a closure of a function body, in the namespace's identifiers:
its target and instantiation, open in the enclosing function's type
parameters. -/
inductive TemplateNode where
  | call (callee : QualifiedRef) (instantiations : Array GenericArgument)
  | closure (target : QualifiedRef) (mask : Nat) (instantiations : Array GenericArgument)
  deriving Repr, BEq, Inhabited

/-- The closure templates of a namespace. -/
structure ClosureTemplates where
  /-- Per function, the calls and closures its body runs, in the order the
  body states them. -/
  functions : Array (Array TemplateNode) := #[]
  /-- The closures without instantiation of every expression of the
  namespace, specifications included: target and mask. -/
  literals : Array (QualifiedRef × Nat) := #[]
  deriving Repr, BEq, Inhabited

/-- Whether an expression belongs to a specification, which a body does not
run: its calls and closures are no part of the body's. -/
private def specificationOnly : ExprKind → Bool
  | .spec _ | .quantifier .. | .operation (.specification _) _ _ _ => true
  | _ => false

/-- The calls and closures a body runs, in the order it states them: each
expression before its operands, the operands in order, specifications
skipped. -/
def bodyTemplate (body : Namespace FunctionBody) (root : ExprId) : Array TemplateNode := Id.run do
  let mut nodes := #[]
  let mut pending := #[root]
  let mut visited : Std.HashSet Nat := {}
  while let some id := pending.back? do
    pending := pending.pop
    if visited.contains id.index then continue
    visited := visited.insert id.index
    let some expression := body.expressions[id.index]? | continue
    if specificationOnly expression.kind then continue
    match expression.kind with
    | .operation (.call (.function callee)) instantiations _ _ =>
        nodes := nodes.push (.call callee instantiations)
    | .operation (.call (.closure target mask)) instantiations _ _ =>
        nodes := nodes.push (.closure target mask instantiations)
    | _ => pure ()
    pending := pending ++ (expressionChildren expression.kind).reverse
  return nodes

/-- The closure templates of a namespace, from its body alone. -/
def closureTemplates (body : Namespace FunctionBody) : ClosureTemplates where
  functions := body.functions.map fun declaration =>
    match declaration.body with
    | .structured root => bodyTemplate body root
    | .absent => #[]
  literals := body.expressions.filterMap fun expression =>
    match expression.kind with
    | .operation (.call (.closure target mask)) instantiations _ _ =>
        if instantiations.isEmpty then some (target, mask) else none
    | _ => none

end VerexIR.Validation
