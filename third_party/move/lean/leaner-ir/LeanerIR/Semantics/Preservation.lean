-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.BigStep
import LeanerIR.Semantics.FrameInstantiation
import LeanerIR.Validation.StaticTypingSubst

/-!
# Preservation of runtime typing

Evaluation of a checked body from a typed frame and state ends in a typed
frame and state, under loans extending the starting ones, with a control of
the type the checker assigns: a value of the node's type, a `break` value
of its loop's type, returned values of the function's result types
(`designs/static-typing.md`, Phase 4). Natives are assumed to preserve
typing (`NativesTyped`): the runtime provides them and `nativeCall` is
opaque.
-/

namespace LeanerIR

open Validation
open StaticTyping
open SemanticOperations
open BigStep

/-- How a frame reads its instantiation in a context: faithfully at the
types its body requires, or, in a constant's initializer, not at all. -/
def FrameReads (context : Context) (frame : RuntimeFrame) : Prop :=
  match context.required with
  | some required => FrameInstantiation context.ns required frame.typeInstantiation context.env
  | none => frame.typeInstantiation = #[] ∧ context.env = #[]

/-- A frame running a checked context: its locals typed and its
instantiation read faithfully. -/
structure Running (unit : ValidatedUnit) (loans : LoanTypes) (context : Context)
    (frame : RuntimeFrame) : Prop where
  typed : TypedFrame unit loans context.ns context.locals context.env frame
  reads : FrameReads context frame

/-- A value an expression produced: of the expression's type, from an
expression that does not divert. -/
def ValueTyped (unit : ValidatedUnit) (loans : LoanTypes) (context : Context) (id : ExprId)
    (value : RuntimeValue) : Prop :=
  (∀ fuel, diverts context.ns fuel id = false) ∧
    ∃ type, context.exprType id = some type ∧ HasType unit loans value type

/-- Values operands produced, each typed as its operand. -/
def ValuesTyped (unit : ValidatedUnit) (loans : LoanTypes) (context : Context) :
    List ExprId → List RuntimeValue → Prop
  | [], [] => True
  | id :: ids, value :: values =>
      ValueTyped unit loans context id value ∧ ValuesTyped unit loans context ids values
  | _, _ => False

/-- A control an expression ended with, typed as the checker types it. -/
def ControlTyped (unit : ValidatedUnit) (loans : LoanTypes) (context : Context) (id : ExprId) :
    Control → Prop
  | .value value => ValueTyped unit loans context id value
  | .break_ nest value => (∀ fuel, breaksTo context.ns fuel nest id = true) ∧
      ∃ loopType, context.loops[nest]? = some loopType ∧
        HasType unit loans (value.getD .unit) loopType
  | .continue_ _ => True
  | .return_ values => ∃ results, context.results = some results ∧
      HasTypes unit loans values.toList results
  | .throw_ _ _ => True

/-- Where a context sits: its unit, target width, and namespace. -/
structure Placed {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (namespaceId : NamespaceId) (context : Context) : Prop where
  unit_eq : context.unit = unit
  width_eq : context.pointerWidth = executable.targetPointerWidth
  namespace_eq : unit.namespaces[namespaceId.index]? = some context.ns
  table_eq : requiredTypes unit = some context.table

/-- The conclusion every evaluation step establishes: typed frame and state
under loans extending the starting ones. -/
def Preserved (unit : ValidatedUnit) (loans : LoanTypes) (inert : Nat) (context : Context)
    (frame : RuntimeFrame) (state : RuntimeState) (holds : LoanTypes → Prop) : Prop :=
  ∃ loans', loans'.Extends loans ∧ Running unit loans' context frame ∧
    TypedState unit loans' inert state ∧ holds loans'

def ExprPreserves {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (namespaceId : NamespaceId) (frame : RuntimeFrame)
    (state : RuntimeState) (id : ExprId) (frame' : RuntimeFrame) (state' : RuntimeState)
    (control : Control) : Prop :=
  ∀ (loans : LoanTypes) (inert : Nat) (context : Context) (fuel : Nat),
    Placed executable namespaceId context → context.checkTree fuel id = true →
    Running unit loans context frame → TypedState unit loans inert state →
    Preserved unit loans inert context frame' state' fun loans' =>
      ControlTyped unit loans' context id control

def ValuesPreserve {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (namespaceId : NamespaceId) (frame : RuntimeFrame)
    (state : RuntimeState) (ids : List ExprId) (result : ValuesResult) : Prop :=
  ∀ (loans : LoanTypes) (inert : Nat) (context : Context) (fuel : Nat),
    Placed executable namespaceId context → (∀ id ∈ ids, context.checkTree fuel id = true) →
    Running unit loans context frame → TypedState unit loans inert state →
    match result with
    | .values state' frame' values => Preserved unit loans inert context frame' state' fun loans' =>
        ValuesTyped unit loans' context ids values
    | .control state' frame' control => Preserved unit loans inert context frame' state' fun loans' =>
        Abrupt control ∧ ∃ id ∈ ids, ControlTyped unit loans' context id control

def StatementsPreserve {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (namespaceId : NamespaceId) (frame : RuntimeFrame)
    (state : RuntimeState) (statements : List ExprId) (result : StatementsResult) : Prop :=
  ∀ (loans : LoanTypes) (inert : Nat) (context : Context) (fuel : Nat),
    Placed executable namespaceId context → (∀ id ∈ statements, context.checkTree fuel id = true) →
    Running unit loans context frame → TypedState unit loans inert state →
    match result with
    | .done state' frame' => Preserved unit loans inert context frame' state' fun loans' =>
        ∀ id ∈ statements, ∃ value, ValueTyped unit loans' context id value
    | .control state' frame' control => Preserved unit loans inert context frame' state' fun loans' =>
        Abrupt control ∧ ∃ id ∈ statements, ControlTyped unit loans' context id control

def ArmsPreserve {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (namespaceId : NamespaceId) (ns : ValidatedNamespace)
    (frame : RuntimeFrame) (state : RuntimeState) (value : RuntimeValue) (arms : List MatchArm)
    (frame' : RuntimeFrame) (state' : RuntimeState) (control : Control) : Prop :=
  ∀ (loans : LoanTypes) (inert : Nat) (context : Context) (fuel : Nat) (scrutineeType resultType : SemTy),
    Placed executable namespaceId context → context.ns = ns →
    (∀ arm ∈ arms, context.patternTyped arm.pattern scrutineeType = true ∧
      arm.guard.all (fun guard => context.flows guard .bool && context.checkTree fuel guard) ∧
      context.flows arm.body resultType = true ∧ context.checkTree fuel arm.body = true) →
    HasType unit loans value scrutineeType →
    Running unit loans context frame → TypedState unit loans inert state →
    Preserved unit loans inert context frame' state' fun loans' => (∃ arm ∈ arms,
      ControlTyped unit loans' context arm.body control ∨
        ∃ guard, arm.guard = some guard ∧ ControlTyped unit loans' context guard control ∧
          Abrupt control) ∨
      -- No arm matched: the profile's mismatch throw.
      ∃ kind arguments, control = .throw_ kind arguments

/-- A function call from a typed state with arguments of the function's
parameter types ends in a typed state, returning values of its result
types. -/
def FunctionPreserves (unit : ValidatedUnit) (handle : FunctionHandle)
    (instantiation : Array (TypeId × TypeId)) (state : RuntimeState)
    (arguments : Array RuntimeValue) (state' : RuntimeState) (outcome : Outcome) : Prop :=
  ∀ (loans : LoanTypes) (inert : Nat) (ns : ValidatedNamespace)
    (declaration : FunctionDecl FunctionBody)
    (semantic : Array SemArg) (parameters results : List SemTy),
    unit.namespaces[handle.namespaceId.index]? = some ns →
    ns.functions[handle.functionId.index]? = some declaration →
    FrameInstantiation ns (requiredAt unit handle) instantiation
      (frameEnv declaration.signature.generics semantic) →
    signatureTypes? ns declaration (frameEnv declaration.signature.generics semantic) =
      some (parameters, results) →
    HasTypes unit loans arguments.toList parameters →
    TypedState unit loans inert state →
    ∃ loans', loans'.Extends loans ∧ TypedState unit loans' inert state' ∧
      ∀ values, outcome = .returned values → HasTypes unit loans' values.toList results

/-! ## Support -/

variable {inert : Nat}

theorem ValueTyped.weaken {unit : ValidatedUnit} {smaller larger : LoanTypes}
    {context : Context} {id : ExprId} {value : RuntimeValue} (extends_ : larger.Extends smaller)
    (typed : ValueTyped unit smaller context id value) : ValueTyped unit larger context id value :=
  ⟨typed.1, let ⟨type, type_eq, value_typed⟩ := typed.2; ⟨type, type_eq, value_typed.weaken extends_⟩⟩

theorem ValuesTyped.weaken {unit : ValidatedUnit} {smaller larger : LoanTypes}
    {context : Context} (extends_ : larger.Extends smaller) :
    ∀ {ids : List ExprId} {values : List RuntimeValue},
      ValuesTyped unit smaller context ids values → ValuesTyped unit larger context ids values
  | [], [], _ => trivial
  | _ :: _, _ :: _, ⟨head, tail⟩ => ⟨head.weaken extends_, ValuesTyped.weaken extends_ tail⟩

theorem ControlTyped.weaken {unit : ValidatedUnit} {smaller larger : LoanTypes}
    {context : Context} {id : ExprId} (extends_ : larger.Extends smaller) :
    ∀ {control : Control}, ControlTyped unit smaller context id control →
      ControlTyped unit larger context id control
  | .value _, typed => ValueTyped.weaken extends_ typed
  | .break_ _ _, ⟨breaks, loopType, loopType_eq, value_typed⟩ =>
      ⟨breaks, loopType, loopType_eq, value_typed.weaken extends_⟩
  | .continue_ _, typed => typed
  | .return_ _, ⟨results, results_eq, values_typed⟩ =>
      ⟨results, results_eq, values_typed.weaken extends_⟩
  | .throw_ _ _, typed => typed

theorem Running.weaken {unit : ValidatedUnit} {smaller larger : LoanTypes} {context : Context}
    {frame : RuntimeFrame} (extends_ : larger.Extends smaller)
    (running : Running unit smaller context frame) : Running unit larger context frame :=
  ⟨running.typed.weaken extends_, running.reads⟩

/-- A frame with the same instantiation reads it as faithfully. -/
theorem Running.of_typed {unit : ValidatedUnit} {loans loans' : LoanTypes} {context : Context}
    {frame frame' : RuntimeFrame} (running : Running unit loans context frame)
    (typed : TypedFrame unit loans' context.ns context.locals context.env frame')
    (instantiation_eq : frame'.typeInstantiation = frame.typeInstantiation) :
    Running unit loans' context frame' :=
  ⟨typed, by unfold FrameReads; rw [instantiation_eq]; exact running.reads⟩

/-- A node a checked tree runs is checked, and so is each child it runs, one
level deeper. -/
theorem checkTree_step {context : Context} {fuel : Nat} {id : ExprId}
    (checked : context.checkTree fuel id = true) :
    ∃ fuel', fuel = fuel' + 1 ∧ context.checkNode id = true ∧
      ∀ child ∈ context.children id, child.1.checkTree fuel' child.2 = true := by
  cases fuel with
  | zero => simp [Context.checkTree] at checked
  | succ fuel =>
      simp only [Context.checkTree, Bool.and_eq_true, List.all_eq_true] at checked
      exact ⟨fuel, rfl, checked.1, fun child member => checked.2 child member⟩

/-- Whether a node runs its children in its own context: all but a loop
(its body inside the loop) and the nodes that run nothing. -/
def runsPlainly : ExprKind → Bool
  | .loop .. | .spec _ | .quantifier .. => false
  | _ => true

/-- The children of a node that runs them plainly run in its context. -/
theorem child_checkTree {context : Context} {fuel : Nat} {id child : ExprId}
    {expression : Expr} (expression_eq : context.ns.expressions[id.index]? = some expression)
    (plain : runsPlainly expression.kind = true)
    (checked : ∀ child ∈ context.children id, child.1.checkTree fuel child.2 = true)
    (member : child ∈ expressionChildren expression.kind) : context.checkTree fuel child = true := by
  apply checked (context, child)
  unfold Context.children
  rw [expression_eq]
  obtain ⟨loc, typeId, kind⟩ := expression
  cases kind <;> simp only [runsPlainly, Bool.false_eq_true] at plain <;>
    simp only [List.mem_map] <;> exact ⟨child, by simpa using member, rfl⟩

/-! ## Frames keep their instantiation -/

theorem fillVisibleHole_instantiation (frame : RuntimeFrame) (state : RuntimeState) (loan : Nat)
    (replacement : RuntimeValue) :
    (fillVisibleHole frame state loan replacement).1.typeInstantiation =
      frame.typeInstantiation := by
  unfold fillVisibleHole
  repeat' split
  all_goals rfl

theorem applyWriteBack_instantiation (frame : RuntimeFrame) (state : RuntimeState) (loan : Nat)
    (current : RuntimeValue) :
    (applyWriteBack frame state loan current).1.typeInstantiation = frame.typeInstantiation := by
  unfold applyWriteBack
  rcases fill_eq : fillVisibleHole frame state loan current with ⟨frame', state', found⟩
  have := fillVisibleHole_instantiation frame state loan current
  rw [fill_eq] at this
  cases found <;> exact this

theorem clearBorrowValue_instantiation (frame : RuntimeFrame) (state : RuntimeState)
    (loan : Nat) :
    (clearBorrowValue frame state loan).1.typeInstantiation = frame.typeInstantiation := by
  unfold clearBorrowValue
  dsimp only
  repeat' split
  all_goals rfl

theorem settleLoans_instantiation (ended : Array LoanId) (frame : RuntimeFrame)
    (state : RuntimeState) :
    (settleLoans ended frame state).1.typeInstantiation = frame.typeInstantiation := by
  unfold settleLoans
  refine Array.foldr_preserves
    (fun (pair : RuntimeFrame × RuntimeState) =>
      pair.1.typeInstantiation = frame.typeInstantiation) _ _ _ rfl
    fun lexical ⟨frame', state'⟩ same => ?_
  dsimp only at same ⊢
  split
  · exact same
  · split
    · rw [applyWriteBack_instantiation, clearBorrowValue_instantiation]
      exact same
    · exact same

theorem settleAfter_instantiation (ended : Array LoanId) (control : Control)
    (frame : RuntimeFrame) (state : RuntimeState) :
    (settleAfter ended control frame state).1.typeInstantiation = frame.typeInstantiation := by
  unfold settleAfter
  split
  · exact settleLoans_instantiation ended frame state
  · rfl

theorem writeRuntimePlace?_instantiation {frame frame' : RuntimeFrame}
    {state state' : RuntimeState} {place : RuntimePlace} {value : RuntimeValue}
    (write : writeRuntimePlace? frame state place value = some (frame', state')) :
    frame'.typeInstantiation = frame.typeInstantiation := by
  have root : ∀ stored, writeRoot? frame state place.root stored = some (frame', state') →
      frame'.typeInstantiation = frame.typeInstantiation := by
    intro stored stored_eq
    unfold writeRoot? at stored_eq
    split at stored_eq
    · split at stored_eq
      · cases stored_eq
      · simp only [Option.some.injEq, Prod.mk.injEq] at stored_eq
        rw [← stored_eq.1]
    · simp only [Option.bind_eq_bind, Option.bind_eq_some_iff, Option.some.injEq,
        Prod.mk.injEq] at stored_eq
      obtain ⟨_, -, rfl, -⟩ := stored_eq
      rfl
  unfold writeRuntimePlace? at write
  split at write
  · cases write
  split at write
  · exact root _ write
  · simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at write
    obtain ⟨_, -, _, -, write⟩ := write
    exact root _ write

theorem applyPendingFrom_instantiation (inherited : Array (Nat × RuntimeValue))
    (frame : RuntimeFrame) (state : RuntimeState) :
    (applyPendingFrom inherited frame state).1.typeInstantiation = frame.typeInstantiation := by
  unfold applyPendingFrom
  refine Array.foldl_preserves
    (fun (pair : RuntimeFrame × RuntimeState) =>
      pair.1.typeInstantiation = frame.typeInstantiation) _ _ _ rfl
    fun ⟨frame', state'⟩ ⟨loan, current⟩ _ same => ?_
  dsimp only at same ⊢
  unfold applyPendingWriteBack
  split
  · rename_i resolved resolved_eq
    simp only [fillLocalLoanHole?, Option.bind_eq_bind, Option.bind_eq_some_iff] at resolved_eq
    obtain ⟨_, -, _, -, _, -, ⟨written, writtenState⟩, write, resolved_eq⟩ := resolved_eq
    simp only [Option.some.injEq] at resolved_eq
    subst resolved_eq
    exact (writeRuntimePlace?_instantiation write).trans same
  · repeat' split
    all_goals exact same

theorem registerReturnedLoan_locals (lexical : Option Nat) (results : Array RuntimeValue)
    (frame : RuntimeFrame) : (registerReturnedLoan lexical results frame).locals = frame.locals := by
  unfold registerReturnedLoan
  split
  · rfl
  · dsimp only
    split <;> rfl

theorem registerReturnedLoan_instantiation (lexical : Option Nat) (results : Array RuntimeValue)
    (frame : RuntimeFrame) :
    (registerReturnedLoan lexical results frame).typeInstantiation = frame.typeInstantiation := by
  unfold registerReturnedLoan
  split
  · rfl
  · dsimp only
    split <;> rfl

/-- A context placed at a namespace reads that namespace. -/
theorem Placed.ns_eq {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {namespaceId : NamespaceId} {context : Context}
    {ns : ValidatedNamespace} (placed : Placed executable namespaceId context)
    (namespace_eq : unit.namespaces[namespaceId.index]? = some ns) : context.ns = ns := by
  have := placed.namespace_eq
  rw [namespace_eq, Option.some.injEq] at this
  exact this.symm

theorem Placed.width {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {namespaceId : NamespaceId} {context : Context}
    (placed : Placed executable namespaceId context) :
    context.pointerWidth = targetPointerWidth? unit :=
  placed.width_eq.trans executable.width_eq

/-- A checked node's own type. -/
theorem checkNode_type {context : Context} {id : ExprId} {expression : Expr}
    (checked : context.checkNode id = true)
    (expression_eq : context.ns.expressions[id.index]? = some expression) :
    ∃ type, context.typeOf expression.typeId = some type := by
  unfold Context.checkNode at checked
  rw [expression_eq] at checked
  simp only at checked
  cases type_eq : context.typeOf expression.typeId with
  | none => rw [type_eq] at checked; cases checked
  | some type => exact ⟨type, rfl⟩

theorem exprType_of {context : Context} {id : ExprId} {expression : Expr} {type : SemTy}
    (expression_eq : context.ns.expressions[id.index]? = some expression)
    (type_eq : context.typeOf expression.typeId = some type) : context.exprType id = some type := by
  simp [Context.exprType, expression_eq, type_eq]

/-- A node of a kind that produces a value without control transfer does
not divert. -/
theorem diverts_false_of {ns : ValidatedNamespace} {id : ExprId} {expression : Expr}
    (expression_eq : ns.expressions[id.index]? = some expression)
    (quiet : match expression.kind with
      | .value .. | .constant _ | .localVar _ | .spec _ => True
      | _ => False) : ∀ fuel, diverts ns fuel id = false := by
  intro fuel
  cases fuel with
  | zero => rfl
  | succ fuel =>
      unfold diverts
      rw [expression_eq]
      obtain ⟨loc, typeId, kind⟩ := expression
      cases kind <;> simp_all

/-- No value has a type `never` is under shared references. -/
theorem ValueTyped.not_stops {unit : ValidatedUnit} {loans : LoanTypes} {context : Context}
    {id : ExprId} {value : RuntimeValue} (typed : ValueTyped unit loans context id value) :
    context.stops id = false := by
  obtain ⟨quiet, type, type_eq, value_typed⟩ := typed
  simp only [Context.stops, type_eq, quiet, Bool.or_false, beq_eq_false_iff_ne, ne_eq,
    Option.some.injEq]
  rintro rfl
  exact value_typed.inhabited (by simp [SemTy.unshared])

/-- A value flowing where a type goes has that type. -/
theorem ValueTyped.flows {unit : ValidatedUnit} {loans : LoanTypes} {context : Context}
    {id : ExprId} {value : RuntimeValue} {type : SemTy}
    (typed : ValueTyped unit loans context id value) (flowing : context.flows id type = true) :
    HasType unit loans value type := by
  simp only [Context.flows, typed.not_stops, Bool.or_false, beq_iff_eq] at flowing
  obtain ⟨_, type', type_eq, value_typed⟩ := typed
  rw [type_eq, Option.some.injEq] at flowing
  exact flowing ▸ value_typed

/-- An abrupt control of a child a node runs plainly is its control. -/
theorem ControlTyped.parent {unit : ValidatedUnit} {loans : LoanTypes} {context : Context}
    {id child : ExprId} {expression : Expr} {control : Control}
    (expression_eq : context.ns.expressions[id.index]? = some expression)
    (plain : runsPlainly expression.kind = true)
    (member : child ∈ expressionChildren expression.kind)
    (typed : ControlTyped unit loans context child control) (abrupt : Abrupt control) :
    ControlTyped unit loans context id control := by
  cases abrupt with
  | break_ nest value =>
      obtain ⟨breaks, loopType, loopType_eq, value_typed⟩ := typed
      refine ⟨fun fuel => ?_, loopType, loopType_eq, value_typed⟩
      cases fuel with
      | zero => rfl
      | succ fuel =>
          unfold breaksTo
          rw [expression_eq]
          obtain ⟨loc, typeId, kind⟩ := expression
          cases kind <;> simp only [runsPlainly, Bool.false_eq_true] at plain <;>
            simp only [expressionChildren] at member <;>
            first
              | simp only [Bool.or_eq_true, beq_iff_eq, Option.any_eq_true]
                right
                simp only [Option.toArray, Array.mem_def] at member
                cases ‹Option ExprId› <;> simp_all
              | exact Array.any_eq_true'.mpr ⟨child, member, breaks fuel⟩
  | continue_ nest => trivial
  | return_ values => exact typed
  | throw_ kind arguments => trivial

/-- A checked unit places each namespace at its index, reading the unit's
types and names, with each function and constant checked. -/
theorem checkUnit_namespace {unit : ValidatedUnit} {pointerWidth : Option Nat}
    (checked : checkUnit unit pointerWidth = true) {index : Nat} {ns : ValidatedNamespace}
    (ns_eq : unit.namespaces[index]? = some ns) :
    ns.identity.index = index ∧ ns.tables.types = unit.tables.types ∧
      ns.tables.names = unit.tables.names ∧ ns.structs.all (variantsDistinct ns) = true ∧
      ∃ table, requiredTypes unit = some table ∧
        (∀ functionIndex declaration, ns.functions[functionIndex]? = some declaration →
          checkFunction unit pointerWidth ns table (index, functionIndex) declaration = true) ∧
        ∀ declaration ∈ ns.constants, checkConstant unit pointerWidth ns table declaration = true := by
  unfold checkUnit at checked
  split at checked
  · cases checked
  rename_i table table_eq
  have member : (ns, index) ∈ unit.namespaces.zipIdx := by
    simp [Array.mem_zipIdx_iff_getElem?, ns_eq]
  have holds := Array.all_eq_true'.mp checked _ member
  simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq, Array.all_eq_true'] at holds
  obtain ⟨⟨⟨⟨⟨identity, types⟩, names⟩, distinct⟩, functions⟩, constants⟩ := holds
  refine ⟨identity, types, names, Array.all_eq_true'.mpr distinct, table, table_eq,
    fun functionIndex declaration declaration_eq => ?_, constants⟩
  exact functions (declaration, functionIndex)
    (by simp [Array.mem_zipIdx_iff_getElem?, declaration_eq])

/-- A placed context's namespace sits at its own identity. -/
theorem Placed.identity {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {namespaceId : NamespaceId} {context : Context}
    (placed : Placed executable namespaceId context) :
    unit.namespaces[context.ns.identity.index]? = some context.ns := by
  have := (checkUnit_namespace executable.typed placed.namespace_eq).1
  rw [this]
  exact placed.namespace_eq

/-- Typed values flowing where the types of a row go have those types. -/
theorem ValuesTyped.flows {unit : ValidatedUnit} {loans : LoanTypes} {context : Context} :
    ∀ {ids : List ExprId} {values : List RuntimeValue} {types : List SemTy},
      ValuesTyped unit loans context ids values → ids.length = types.length →
      (ids.zip types).all (fun pair => context.flows pair.1 pair.2) = true →
      HasTypes unit loans values types
  | [], [], [], _, _, _ => .nil
  | _ :: _, _ :: _, _ :: _, ⟨head, tail⟩, length, all => by
      simp only [List.zip_cons_cons, List.all_cons, Bool.and_eq_true] at all
      exact .cons (head.flows all.1) (ValuesTyped.flows tail (by simpa using length) all.2)
  | [], _ :: _, _, typed, _, _ | _ :: _, [], _, typed, _, _ => by cases typed
  | [], [], _ :: _, _, length, _ => by cases length
  | _ :: _, _ :: _, [], _, length, _ => by cases length

/-- Operands that produced values do not divert. -/
theorem ValuesTyped.quiet {unit : ValidatedUnit} {loans : LoanTypes} {context : Context} :
    ∀ {ids : List ExprId} {values : List RuntimeValue},
      ValuesTyped unit loans context ids values →
        ∀ id ∈ ids, ∀ fuel, diverts context.ns fuel id = false
  | [], _, _, _, member => by cases member
  | _ :: _, _ :: _, ⟨head, tail⟩, _, member => by
      cases member with
      | head => exact head.1
      | tail _ member => exact ValuesTyped.quiet tail _ member
  | _ :: _, [], typed, _, _ => by cases typed

/-- An operation whose operands produced values does not divert. -/
theorem ValuesTyped.operation_quiet {unit : ValidatedUnit} {loans : LoanTypes} {context : Context}
    {id : ExprId} {expression : Expr} {operation : Operation}
    {instantiations : Array GenericArgument} {operands : Array ExprId} {surface : Option SurfaceSyntax}
    {values : List RuntimeValue}
    (expression_eq : context.ns.expressions[id.index]? = some expression)
    (kind_eq : expression.kind = .operation operation instantiations operands surface)
    (typed : ValuesTyped unit loans context operands.toList values) :
    ∀ fuel, diverts context.ns fuel id = false := by
  intro fuel
  cases fuel with
  | zero => rfl
  | succ fuel =>
      unfold diverts
      rw [expression_eq]
      simp only [kind_eq, Array.any_eq_false]
      intro operand member
      simp [typed.quiet operands[operand] (by simp) fuel]

/-- A `break` leaves the loops its own nesting says. -/
theorem breaksTo_break {ns : ValidatedNamespace} {id : ExprId} {expression : Expr}
    {nest : Nat} {value : Option ExprId}
    (expression_eq : ns.expressions[id.index]? = some expression)
    (kind_eq : expression.kind = .break_ nest value) : ∀ fuel, breaksTo ns fuel nest id = true := by
  intro fuel
  cases fuel with
  | zero => rfl
  | succ fuel =>
      unfold breaksTo
      rw [expression_eq]
      simp [kind_eq]

/-- A control is a value or abrupt. -/
theorem Control.value_or_abrupt : ∀ control : Control,
    (∃ result, control = .value result) ∨ Abrupt control
  | .value result => .inl ⟨result, rfl⟩
  | .break_ nest result => .inr (.break_ nest result)
  | .continue_ nest => .inr (.continue_ nest)
  | .return_ values => .inr (.return_ values)
  | .throw_ kind arguments => .inr (.throw_ kind arguments)

theorem bindPatternFuel_instantiation {unit : ValidatedUnit} {ns : ValidatedNamespace} :
    ∀ (fuel : Nat) {frame frame' : RuntimeFrame} {pattern : PatternId} {value : RuntimeValue},
      bindPatternFuel unit ns frame fuel pattern value = some frame' →
      frame'.typeInstantiation = frame.typeInstantiation
  | 0, _, _, _, _, bind => by simp [bindPatternFuel] at bind
  | fuel + 1, frame, frame', pattern, value, bind => by
      have row : ∀ (patterns : List PatternId) (values : List RuntimeValue) (frame frame' : RuntimeFrame),
          bindPatternRow (fun pattern value frame =>
            bindPatternFuel unit ns frame fuel pattern value) patterns values frame = some frame' →
          frame'.typeInstantiation = frame.typeInstantiation := by
        intro patterns
        induction patterns with
        | nil =>
            intro values frame frame' bind
            cases values with
            | nil => simp only [bindPatternRow, Option.some.injEq] at bind; rw [bind]
            | cons _ _ => simp [bindPatternRow] at bind
        | cons pattern patterns ih =>
            intro values frame frame' bind
            cases values with
            | nil => simp [bindPatternRow] at bind
            | cons value values =>
                simp only [bindPatternRow, Option.bind_eq_bind, Option.bind_eq_some_iff] at bind
                obtain ⟨bound, bound_eq, bind⟩ := bind
                exact (ih values bound frame' bind).trans
                  (bindPatternFuel_instantiation fuel bound_eq)
      unfold bindPatternFuel at bind
      simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at bind
      obtain ⟨⟨loc, typeId, kind⟩, -, bind⟩ := bind
      cases kind with
      | wildcard => simp only [pure, Option.some.injEq] at bind; rw [bind]
      | «variable» localId =>
          simp only at bind
          split at bind
          · simp only [pure, Option.some.injEq] at bind; rw [← bind]
          · cases bind
      | tuple elements =>
          cases value with
          | tuple values =>
              simp only at bind
              split at bind
              · cases bind
              · exact row _ _ _ _ bind
          | _ => simp at bind
      | constructor name instantiations variant fields =>
          cases value with
          | nominal source actualVariant values =>
              simp only [Option.bind_eq_some_iff] at bind
              obtain ⟨_, -, bind⟩ := bind
              split at bind
              · cases bind
              · exact row _ _ _ _ bind
          | _ => simp at bind
      | literal literal =>
          simp only [Option.bind_eq_some_iff] at bind
          obtain ⟨_, -, bind⟩ := bind
          split at bind
          · simp only [pure, Option.some.injEq] at bind; rw [bind]
          · cases bind
      | range lower upper inclusive =>
          have unchanged : frame' = frame := by
            revert bind
            repeat' split
            all_goals simp_all [Option.bind, pure]
          rw [unchanged]

theorem bindPattern_instantiation {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {frame frame' : RuntimeFrame} {pattern : PatternId} {value : RuntimeValue}
    (bind : bindPattern unit ns frame pattern value = some frame') :
    frame'.typeInstantiation = frame.typeInstantiation :=
  bindPatternFuel_instantiation _ bind

/-- A pattern binding typed as the checker types it keeps the frame
running. -/
theorem Running.bind {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {namespaceId : NamespaceId} {loans : LoanTypes}
    {context : Context} {frame frame' : RuntimeFrame} {pattern : PatternId}
    {value : RuntimeValue} {type : SemTy} (placed : Placed executable namespaceId context)
    (running : Running unit loans context frame)
    (typed : context.patternTyped pattern type = true)
    (value_typed : HasType unit loans value type)
    (bind : bindPattern unit context.ns frame pattern value = some frame') :
    Running unit loans context frame' :=
  running.of_typed (bindPatternFuel_typed placed.unit_eq _ _ running.typed typed value_typed bind)
    (bindPattern_instantiation bind)

/-- A place the checker types resolves, in a running frame, to a typed
place. -/
theorem Running.place {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {namespaceId : NamespaceId} {loans : LoanTypes}
    {context : Context} {frame : RuntimeFrame} {state : RuntimeState} {place : PlaceId}
    {type : SemTy} {resolved : RuntimePlace} (placed : Placed executable namespaceId context)
    (running : Running unit loans context frame)
    (typed : context.placeTypeOf place = some type)
    (resolve : resolvePlace? unit context.ns frame state place = some resolved) :
    ∃ variant, PlaceTyped unit loans context frame resolved type variant := by
  simp only [Context.placeTypeOf, Functor.map, Option.map_eq_some_iff] at typed
  obtain ⟨⟨type', variant⟩, placeType_eq, rfl⟩ := typed
  exact ⟨variant, resolvePlace_typed placed.unit_eq placed.identity running.typed _ _
    placeType_eq resolve⟩

private theorem match_children_fold (arms : List MatchArm) :
    ∀ (init : Array ExprId),
      (∀ id ∈ init, id ∈ arms.foldl (fun ids arm => match arm.guard with
        | some guard => (ids.push guard).push arm.body
        | none => ids.push arm.body) init) ∧
      ∀ arm ∈ arms, arm.body ∈ arms.foldl (fun ids arm => match arm.guard with
          | some guard => (ids.push guard).push arm.body
          | none => ids.push arm.body) init ∧
        ∀ guard, arm.guard = some guard → guard ∈ arms.foldl (fun ids arm => match arm.guard with
          | some guard => (ids.push guard).push arm.body
          | none => ids.push arm.body) init := by
  induction arms with
  | nil => intro init; simp
  | cons arm rest ih =>
      intro init
      obtain ⟨keeps, holds⟩ := ih (match arm.guard with
        | some guard => (init.push guard).push arm.body
        | none => init.push arm.body)
      simp only [List.foldl_cons]
      refine ⟨fun id member => keeps id (by split <;> simp [member]), fun arm' member => ?_⟩
      simp only [List.mem_cons] at member
      rcases member with rfl | member
      · refine ⟨keeps _ (by split <;> simp), fun guard guard_eq => keeps _ ?_⟩
        rw [guard_eq]
        simp
      · exact holds arm' member

/-- A match runs its scrutinee, guards, and bodies as children. -/
theorem match_children {scrutinee : ExprId} {arms : Array MatchArm} :
    scrutinee ∈ expressionChildren (.match_ scrutinee arms) ∧
      ∀ arm ∈ arms, arm.body ∈ expressionChildren (.match_ scrutinee arms) ∧
        ∀ guard, arm.guard = some guard → guard ∈ expressionChildren (.match_ scrutinee arms) := by
  simp only [expressionChildren]
  rw [← Array.foldl_toList]
  obtain ⟨keeps, holds⟩ := match_children_fold arms.toList #[scrutinee]
  exact ⟨keeps scrutinee (by simp), fun arm member => holds arm (by simpa using member)⟩

theorem Placed.loops {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {namespaceId : NamespaceId} {context : Context}
    (placed : Placed executable namespaceId context) (loops : List SemTy) :
    Placed executable namespaceId { context with loops } :=
  ⟨placed.unit_eq, placed.width_eq, placed.namespace_eq, placed.table_eq⟩

/-- A frame runs a context whatever loops surround it. -/
theorem Running.loops {unit : ValidatedUnit} {loans : LoanTypes} {context : Context}
    {frame : RuntimeFrame} {loops : List SemTy} :
    Running unit loans { context with loops } frame ↔ Running unit loans context frame :=
  ⟨fun running => ⟨running.typed, running.reads⟩, fun running => ⟨running.typed, running.reads⟩⟩

/-- A loop runs its body inside the loop. -/
theorem loop_child_checkTree {context : Context} {fuel : Nat} {id : ExprId}
    {expression : Expr} {label : Option String} {body : ExprId} {type : SemTy}
    (expression_eq : context.ns.expressions[id.index]? = some expression)
    (kind_eq : expression.kind = .loop label body)
    (type_eq : context.typeOf expression.typeId = some type)
    (checked : ∀ child ∈ context.children id, child.1.checkTree fuel child.2 = true) :
    ({ context with loops := type :: context.loops } : Context).checkTree fuel body = true := by
  apply checked ({ context with loops := type :: context.loops }, body)
  unfold Context.children
  simp [expression_eq, kind_eq, type_eq]

/-- A function the runtime resolves, the checker's lookup finds, at the
handle's position. -/
theorem functionTarget_of_resolve {unit : ValidatedUnit} {sourceNamespace : NamespaceId}
    {ns : ValidatedNamespace} {reference : QualifiedRef} {handle : FunctionHandle}
    (ns_eq : unit.namespaces[sourceNamespace.index]? = some ns)
    (resolve : resolveFunction? unit sourceNamespace reference = some handle) :
    ∃ targetNs declaration,
      functionTarget? unit ns reference = some (targetNs, declaration) ∧
      unit.namespaces[handle.namespaceId.index]? = some targetNs ∧
      targetNs.functions[handle.functionId.index]? = some declaration ∧
      functionIndex? unit ns reference = some (handle.namespaceId.index, handle.functionId.index) := by
  simp only [resolveFunction?, ns_eq, Option.bind_eq_bind, Option.bind_some] at resolve
  obtain ⟨qualified, qualified_eq, resolve⟩ := Option.bind_eq_some_iff.mp resolve
  split at resolve
  · cases resolve
  rename_i same
  simp only [bne_iff_ne, ne_eq, Decidable.not_not] at same
  obtain ⟨functionId, functionId_eq, resolve⟩ := Option.bind_eq_some_iff.mp resolve
  obtain ⟨targetNs, targetNs_eq, resolve⟩ := Option.bind_eq_some_iff.mp resolve
  obtain ⟨declaration, declaration_eq, resolve⟩ := Option.bind_eq_some_iff.mp resolve
  simp only [pure, Option.some.injEq] at resolve
  subst resolve
  refine ⟨targetNs, declaration, ?_, targetNs_eq, declaration_eq, ?_⟩
  · simp [functionTarget?, qualified_eq, same, functionId_eq, targetNs_eq, declaration_eq]
  · simp [functionIndex?, qualified_eq, same, functionId_eq, targetNs_eq, declaration_eq]

/-- Kinds agree: each generic argument has its binder's kind. -/
theorem kindsAgree_spec {generics : Array GenericBinder} {instantiations : Array GenericArgument}
    (agree : Context.kindsAgree generics instantiations = true) :
    ∀ (index : Nat) (binder : GenericBinder) (argument : GenericArgument),
      generics[index]? = some binder → instantiations[index]? = some argument →
        (binder.kind = .typeArg ↔ ∃ value, argument = .typeArg value) := by
  intro index binder argument binder_eq argument_eq
  unfold Context.kindsAgree at agree
  have member : (binder, argument) ∈ generics.toList.zip instantiations.toList := by
    rw [List.mem_iff_getElem?]
    exact ⟨index, by simp [List.getElem?_zip_eq_some, binder_eq, argument_eq]⟩
  have holds := List.all_eq_true.mp agree _ member
  simp only [beq_iff_eq] at holds
  obtain ⟨name, kind, abilities, predicates, type, loc⟩ := binder
  cases kind <;> cases argument <;> simp at holds ⊢

/-- At kind-agreeing arguments, a frame environment holds exactly the
arguments' types. -/
theorem frameEnv_agree {ns : ValidatedNamespace} {env : Array SemArg}
    {generics : Array GenericBinder} {instantiations : Array GenericArgument}
    {arguments : Array SemArg}
    (resolved : resolveArguments ns env instantiations = some arguments)
    (agree : Context.kindsAgree generics instantiations = true)
    (arity : instantiations.size = generics.size) (index : Nat) (type : SemTy) :
    arguments[index]? = some (.type type) ↔
      (frameEnv generics arguments)[index]? = some (.type type) := by
  have size_eq := Array.mapM_size resolved
  have kinds := kindsAgree_spec agree
  constructor
  · intro argument_eq
    have bound : index < instantiations.size := by
      rw [← size_eq]; exact (Array.getElem?_eq_some_iff.mp argument_eq).1
    obtain ⟨instantiation, instantiation_eq⟩ : ∃ x, instantiations[index]? = some x :=
      ⟨_, Array.getElem?_eq_getElem bound⟩
    obtain ⟨binder, binder_eq⟩ : ∃ b, generics[index]? = some b :=
      ⟨_, Array.getElem?_eq_getElem (arity ▸ bound)⟩
    obtain ⟨y, y_eq, mapped⟩ := Array.mapM_getElem? resolved instantiation_eq
    rw [argument_eq, Option.some.injEq] at y_eq
    subst y_eq
    have kind : binder.kind = .typeArg := by
      refine (kinds index binder instantiation binder_eq instantiation_eq).mpr ?_
      cases instantiation <;> simp_all
    exact frameEnv_type binder_eq kind argument_eq
  · intro frame_eq
    have bound : index < generics.size := by
      have := (Array.getElem?_eq_some_iff.mp frame_eq).1
      simpa [frameEnv, staticEnv] using this
    obtain ⟨binder, binder_eq⟩ : ∃ b, generics[index]? = some b :=
      ⟨_, Array.getElem?_eq_getElem bound⟩
    have kind : binder.kind = .typeArg := by
      simp only [frameEnv, staticEnv, Array.getElem?_map, Array.getElem?_mapIdx, binder_eq,
        Option.map_some, Option.some.injEq] at frame_eq
      revert frame_eq
      cases binder.kind <;> simp [SemArg.subst]
    obtain ⟨instantiation, instantiation_eq⟩ : ∃ x, instantiations[index]? = some x :=
      ⟨_, Array.getElem?_eq_getElem (arity ▸ bound)⟩
    obtain ⟨value, rfl⟩ := (kinds index binder instantiation binder_eq instantiation_eq).mp kind
    obtain ⟨y, y_eq, mapped⟩ := Array.mapM_getElem? resolved instantiation_eq
    simp only [Functor.map, Option.map_eq_some_iff] at mapped
    obtain ⟨resolvedType, -, rfl⟩ := mapped
    rw [frameEnv_type binder_eq kind y_eq, Option.some.injEq, SemArg.type.injEq] at frame_eq
    rw [y_eq, frame_eq]

/-- A target's signature at kind-agreeing arguments reads as at its frame's
environment. -/
theorem signatureTypes?_frameEnv {ns targetNs : ValidatedNamespace} {env : Array SemArg}
    {declaration : FunctionDecl FunctionBody} {instantiations : Array GenericArgument}
    {arguments : Array SemArg}
    (resolved : resolveArguments ns env instantiations = some arguments)
    (agree : Context.kindsAgree declaration.signature.generics instantiations = true)
    (arity : instantiations.size = declaration.signature.generics.size) :
    signatureTypes? targetNs declaration (frameEnv declaration.signature.generics arguments) =
      signatureTypes? targetNs declaration arguments := by
  simp only [signatureTypes?, resolveIn,
    SemTy.resolveFuel_congr (frameEnv_agree resolved agree arity)]

theorem typeArguments_mem {instantiations : Array GenericArgument} {index : Nat}
    {value : TypeUse} (argument_eq : instantiations[index]? = some (.typeArg value)) :
    value.typeId ∈ typeArguments instantiations := by
  unfold typeArguments
  exact Array.mem_filterMap.mpr ⟨_, Array.mem_of_getElem? argument_eq, rfl⟩

/-- A call or closure from a running frame creates a faithful frame. -/
theorem FrameReads.call {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {namespaceId : NamespaceId} {context : Context}
    {frame : RuntimeFrame} {reference : QualifiedRef} {handle : FunctionHandle}
    {targetNs : ValidatedNamespace} {generics : Array GenericBinder}
    {instantiations : Array GenericArgument} {arguments : Array SemArg}
    (placed : Placed executable namespaceId context) (reads : FrameReads context frame)
    (edge : context.edgeClosed reference instantiations = true)
    (index_eq : functionIndex? context.unit context.ns reference =
      some (handle.namespaceId.index, handle.functionId.index))
    (target_eq : unit.namespaces[handle.namespaceId.index]? = some targetNs)
    (resolved : resolveArguments context.ns context.env instantiations = some arguments)
    (agree : Context.kindsAgree generics instantiations = true)
    (arity : instantiations.size = generics.size) :
    FrameInstantiation targetNs (requiredAt unit handle)
      (callTypeInstantiation unit handle frame.typeInstantiation instantiations)
      (frameEnv generics arguments) := by
  unfold Context.edgeClosed at edge
  rw [index_eq, placed.unit_eq] at edge
  simp only [target_eq, Bool.and_eq_true, Array.all_eq_true'] at edge
  obtain ⟨consulted, closed⟩ := edge
  have required_eq : requiredAt unit handle =
      context.table.at (handle.namespaceId.index, handle.functionId.index) := by
    simp [requiredAt, placed.table_eq]
  rw [required_eq]
  have namespace_eq := (checkUnit_namespace executable.typed placed.namespace_eq)
  have target := checkUnit_namespace executable.typed target_eq
  have types_eq : context.ns.tables.types = targetNs.tables.types :=
    namespace_eq.2.1.trans target.2.1.symm
  have names_eq : context.ns.tables.names = targetNs.tables.names :=
    namespace_eq.2.2.1.trans target.2.2.1.symm
  have instances : ∀ typeId ∈ context.table.at (handle.namespaceId.index, handle.functionId.index),
      ∃ instance_, instantiatePlaceFieldType? targetNs instantiations typeId = some instance_ ∧
        context.consults instance_ = true ∧
        mentionsLifetimeParameter context.ns (context.ns.tables.types.size + 1) instance_ =
          false := by
    intro typeId member
    have holds := closed typeId member
    split at holds
    · rename_i instance_ instance_eq
      simp only [Bool.and_eq_true, Bool.not_eq_true'] at holds
      exact ⟨instance_, instance_eq, holds⟩
    · cases holds
  unfold FrameReads at reads
  split at reads
  · rename_i required required_eq'
    refine FrameInstantiation.call (unit := unit) reads types_eq names_eq target_eq
      (fun index value argument_eq => ?_) (fun typeId member => ?_) resolved
      (kindsAgree_spec agree) arity
    · have := consulted _ (typeArguments_mem argument_eq)
      simpa [Context.consults, required_eq'] using this
    · obtain ⟨instance_, instance_eq, consults, fixed⟩ := instances typeId member
      simp only [Context.consults, required_eq', Array.contains_iff_mem] at consults
      exact ⟨instance_, instance_eq, by simpa using consults, fixed⟩
  · rename_i none_eq
    obtain ⟨instantiation_eq, env_eq⟩ := reads
    rw [env_eq] at resolved
    rw [instantiation_eq]
    let required := typeArguments instantiations ++
      (context.table.at (handle.namespaceId.index, handle.functionId.index)).filterMap
        (instantiatePlaceFieldType? targetNs instantiations)
    have closedFrame : FrameInstantiation context.ns required #[] #[] :=
      .inl ⟨rfl, fun _ _ _ resolved => resolved⟩
    refine FrameInstantiation.call (unit := unit) closedFrame types_eq names_eq target_eq
      (fun index value argument_eq => Array.mem_append_left _ (typeArguments_mem argument_eq))
      (fun typeId member => ?_) resolved (kindsAgree_spec agree) arity
    obtain ⟨instance_, instance_eq, -, fixed⟩ := instances typeId member
    exact ⟨instance_, instance_eq,
      Array.mem_append_right _ (Array.mem_filterMap.mpr ⟨typeId, member, instance_eq⟩), fixed⟩

/-- An operation whose operands ended abruptly ends with their control. -/
theorem operands_control {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId} {ns : ValidatedNamespace}
    {expression : Expr} {operation : Operation} {instantiations : Array GenericArgument}
    {arguments : Array ExprId} {surface : Option SurfaceSyntax} {finalState : RuntimeState}
    {finalFrame : RuntimeFrame} {control : Control}
    (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation operation instantiations arguments surface)
    (ih : ValuesPreserve executable namespaceId frame state arguments.toList
      (.control finalState finalFrame control)) :
    ExprPreserves executable namespaceId frame state exprId finalFrame finalState control := by
  intro loans inert context fuel placed checked running stateTyped
  obtain ⟨fuel', rfl, -, children⟩ := checkTree_step checked
  cases placed.ns_eq namespace_eq
  obtain ⟨loans', extends', running', state', abrupt, id, member, typed⟩ :=
    ih loans inert context fuel' placed (fun id member => child_checkTree expression_eq
      (by simp [kind_eq, runsPlainly]) children (by simpa [kind_eq, expressionChildren] using member))
      running stateTyped
  exact ⟨loans', extends', running', state', typed.parent expression_eq
    (by simp [kind_eq, runsPlainly]) (by simpa [kind_eq, expressionChildren] using member) abrupt⟩

/-- A struct the checker's lookup finds not, the runtime resolves not. -/
theorem resolveStruct_none {unit : ValidatedUnit} {sourceNamespace : NamespaceId}
    {ns : ValidatedNamespace} {reference : QualifiedRef}
    (ns_eq : unit.namespaces[sourceNamespace.index]? = some ns)
    (missing : declarationTarget? unit ns reference = none) :
    resolveStruct? unit sourceNamespace reference = none := by
  cases resolve : resolveStruct? unit sourceNamespace reference with
  | none => rfl
  | some handle =>
      obtain ⟨_, _, found, -⟩ := declarationTarget_of_resolve ns_eq resolve
      rw [missing] at found
      cases found

theorem signatureTypes?_length {targetNs : ValidatedNamespace}
    {declaration : FunctionDecl FunctionBody} {arguments : Array SemArg}
    {parameters results : List SemTy}
    (signature_eq : signatureTypes? targetNs declaration arguments = some (parameters, results)) :
    parameters.length = declaration.signature.parameters.size := by
  simp only [signatureTypes?, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.some.injEq,
    Prod.mk.injEq] at signature_eq
  obtain ⟨parameters', parameters_eq, results', -, rfl, rfl⟩ := signature_eq
  simpa using List.mapM_length parameters_eq

theorem signatureTypes?_results_length {targetNs : ValidatedNamespace}
    {declaration : FunctionDecl FunctionBody} {arguments : Array SemArg}
    {parameters results : List SemTy}
    (signature_eq : signatureTypes? targetNs declaration arguments = some (parameters, results)) :
    results.length = declaration.signature.results.size := by
  simp only [signatureTypes?, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.some.injEq,
    Prod.mk.injEq] at signature_eq
  obtain ⟨parameters', -, results', results_eq, rfl, rfl⟩ := signature_eq
  simpa using List.mapM_length results_eq

theorem initialFrame?_instantiation {declaration : FunctionDecl FunctionBody}
    {arguments : Array RuntimeValue} {instantiation : Array (TypeId × TypeId)}
    {frame : RuntimeFrame} (init : initialFrame? declaration arguments instantiation = some frame) :
    frame.typeInstantiation = instantiation := by
  unfold initialFrame? at init
  split at init
  · cases init
  split at init
  · cases init
  simp only [Option.some.injEq] at init
  subst init
  rfl

theorem ClosureMask.extract_zero_true {α : Type} :
    ∀ values : List α, ClosureMask.extract 0 true values = []
  | [] => rfl
  | _ :: values => by simp [ClosureMask.extract, ClosureMask.extract_zero_true values]

theorem ClosureMask.extract_zero_false {α : Type} :
    ∀ values : List α, ClosureMask.extract 0 false values = values
  | [] => rfl
  | _ :: values => by simp [ClosureMask.extract, ClosureMask.extract_zero_false values]

/-- Captures and supplied arguments typed at a mask's two parts compose to
arguments of the whole row. -/
theorem HasTypes.compose_go {unit : ValidatedUnit} {loans : LoanTypes} :
    ∀ {all : List SemTy} {fuel mask : Nat} {captures supplied composed : List RuntimeValue},
      mask < 2 ^ all.length →
      HasTypes unit loans captures (ClosureMask.extract mask true all) →
      HasTypes unit loans supplied (ClosureMask.extract mask false all) →
      ClosureMask.compose.go fuel mask captures supplied = some composed →
      HasTypes unit loans composed all := by
  intro all
  induction all with
  | nil =>
      intro fuel mask captures supplied composed bound capturesTyped suppliedTyped go_eq
      have : mask = 0 := by simpa using bound
      subst this
      simp only [ClosureMask.extract] at capturesTyped suppliedTyped
      cases capturesTyped
      cases suppliedTyped
      simp only [ClosureMask.compose.go, Option.some.injEq] at go_eq
      subst go_eq
      exact .nil
  | cons type types ih =>
      intro fuel mask captures supplied composed bound capturesTyped suppliedTyped go_eq
      cases mask with
      | zero =>
          rw [ClosureMask.extract_zero_true] at capturesTyped
          rw [ClosureMask.extract_zero_false] at suppliedTyped
          cases capturesTyped
          simp only [ClosureMask.compose.go, Option.some.injEq] at go_eq
          subst go_eq
          exact suppliedTyped
      | succ mask =>
          cases fuel with
          | zero => simp [ClosureMask.compose.go] at go_eq
          | succ fuel =>
              have bound' : (mask + 1) / 2 < 2 ^ types.length := by
                simp only [List.length_cons, Nat.pow_succ] at bound
                omega
              simp only [ClosureMask.compose.go] at go_eq
              simp only [ClosureMask.extract] at capturesTyped suppliedTyped
              split at go_eq
              · rename_i odd
                simp only [odd] at capturesTyped suppliedTyped
                simp only [beq_self_eq_true, if_true, Bool.true_beq, Bool.false_eq_true,
                  if_false] at capturesTyped suppliedTyped
                cases capturesTyped with
                | cons head rest =>
                    simp only [Functor.map, Option.map_eq_some_iff] at go_eq
                    obtain ⟨tail, tail_eq, rfl⟩ := go_eq
                    exact .cons head (ih bound' rest suppliedTyped tail_eq)
              · rename_i even
                have even' : ((mask + 1) % 2 == 1) = false := by simpa using even
                simp only [even'] at capturesTyped suppliedTyped
                simp only [Bool.false_beq, Bool.not_true, Bool.false_eq_true, if_false,
                  beq_self_eq_true, if_true] at capturesTyped suppliedTyped
                cases suppliedTyped with
                | cons head rest =>
                    simp only [Functor.map, Option.map_eq_some_iff] at go_eq
                    obtain ⟨tail, tail_eq, rfl⟩ := go_eq
                    exact .cons head (ih bound' capturesTyped rest tail_eq)

theorem HasTypes.compose {unit : ValidatedUnit} {loans : LoanTypes} {all : List SemTy}
    {mask : Nat} {captures supplied composed : List RuntimeValue}
    (bound : mask < 2 ^ all.length)
    (capturesTyped : HasTypes unit loans captures (ClosureMask.extract mask true all))
    (suppliedTyped : HasTypes unit loans supplied (ClosureMask.extract mask false all))
    (compose_eq : ClosureMask.compose mask captures supplied = some composed) :
    HasTypes unit loans composed all :=
  HasTypes.compose_go bound capturesTyped suppliedTyped compose_eq

/-- Returning from a callee: the caller's frame takes the callee's
write-backs, typed under the loans the callee ended with. -/
theorem Running.returned {unit : ValidatedUnit} {loans loans' : LoanTypes} {context : Context}
    {frame : RuntimeFrame} {state state' : RuntimeState}
    (running : Running unit loans context frame) (stateTyped : TypedState unit loans inert state)
    (extends_ : loans'.Extends loans) (stateTyped' : TypedState unit loans' inert state') :
    Running unit loans' context (applyPendingFrom state.pending frame state').1 ∧
      TypedState unit loans' inert (applyPendingFrom state.pending frame state').2 := by
  have applied := applyPendingFrom_typed (running.typed.weaken extends_) stateTyped'
    (inherited := state.pending) stateTyped.inert_le fun entry member => by
      obtain ⟨type, loan_eq, typed⟩ := stateTyped.pending entry member
      exact ⟨type, extends_ _ _ loan_eq, typed.weaken extends_⟩
  exact ⟨running.of_typed applied.1 (applyPendingFrom_instantiation _ _ _), applied.2⟩

/-- A checked call from a running frame on typed operands returns to a
running frame and a typed state, its results packed at the node's type. -/
theorem Running.call {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {namespaceId : NamespaceId} {loans : LoanTypes}
    {context : Context} {frame : RuntimeFrame} {state finalState : RuntimeState}
    {reference : QualifiedRef} {instantiations : Array GenericArgument} {operands : Array ExprId}
    {values : List RuntimeValue} {handle : FunctionHandle} {outcome : Outcome} {type : SemTy}
    (placed : Placed executable namespaceId context) (running : Running unit loans context frame)
    (stateTyped : TypedState unit loans inert state)
    (valuesTyped : ValuesTyped unit loans context operands.toList values)
    (checked : context.checkCall type (.function reference) instantiations operands = true)
    (resolve_eq : resolveFunction? unit namespaceId reference = some handle)
    (callee : FunctionPreserves unit handle
      (callTypeInstantiation unit handle frame.typeInstantiation instantiations)
      state values.toArray finalState outcome) :
    Preserved unit loans inert context (applyPendingFrom state.pending frame finalState).1
      (applyPendingFrom state.pending frame finalState).2 fun loans' =>
        ∀ results, outcome = .returned results →
          HasType unit loans' (packResults results) type := by
  obtain ⟨targetNs, declaration, target_eq, targetNs_eq, declaration_eq, index_eq⟩ :=
    functionTarget_of_resolve placed.namespace_eq resolve_eq
  rw [← placed.unit_eq] at target_eq index_eq
  simp only [Context.checkCall, target_eq] at checked
  cases resolved : resolveArguments context.ns context.env instantiations with
  | none => simp [resolved] at checked
  | some semantic =>
    simp only [resolved, Bool.and_eq_true, beq_iff_eq] at checked
    obtain ⟨⟨⟨arity, agree⟩, edge⟩, signature⟩ := checked
    split at signature
    · rename_i parameters resultTypes signature_eq
      simp only [Bool.and_eq_true, beq_iff_eq] at signature
      obtain ⟨⟨size_eq, flows⟩, packed⟩ := signature
      have arity' := (Array.mapM_size resolved).symm.trans arity
      have faithful := FrameReads.call placed running.reads edge index_eq targetNs_eq resolved
        agree arity'
      obtain ⟨loans', extends', stateTyped', results_typed⟩ :=
        callee loans inert targetNs declaration semantic parameters resultTypes targetNs_eq
          declaration_eq faithful
          (by rw [signatureTypes?_frameEnv resolved agree arity']; exact signature_eq)
          (by simpa using valuesTyped.flows (by simpa using size_eq) flows) stateTyped
      obtain ⟨running', stateTyped''⟩ := running.returned stateTyped extends' stateTyped'
      exact ⟨loans', extends', running', stateTyped'', fun results outcome_eq =>
        packResults_typed (results_typed results outcome_eq) packed⟩
    · cases signature

/-- A checked closure built from a running frame on typed captures is of
the node's type. -/
theorem Running.closure {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {namespaceId : NamespaceId} {loans : LoanTypes}
    {context : Context} {frame : RuntimeFrame} {reference : QualifiedRef} {mask : Nat}
    {instantiations : Array GenericArgument} {captures : Array ExprId}
    {values : List RuntimeValue} {handle : FunctionHandle} {type : SemTy}
    (placed : Placed executable namespaceId context) (running : Running unit loans context frame)
    (valuesTyped : ValuesTyped unit loans context captures.toList values)
    (checked : context.checkCall type (.closure reference mask) instantiations captures = true)
    (resolve_eq : resolveFunction? unit namespaceId reference = some handle) :
    HasType unit loans (.closure handle mask
      (callTypeInstantiation unit handle frame.typeInstantiation instantiations)
      values.toArray) type := by
  obtain ⟨targetNs, declaration, target_eq, targetNs_eq, declaration_eq, index_eq⟩ :=
    functionTarget_of_resolve placed.namespace_eq resolve_eq
  rw [← placed.unit_eq] at target_eq index_eq
  simp only [Context.checkCall, target_eq] at checked
  cases resolved : resolveArguments context.ns context.env instantiations with
  | none => simp [resolved] at checked
  | some semantic =>
    cases type
    case function openTypes resultType =>
      simp only [resolved, Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at checked
      obtain ⟨⟨⟨⟨arity, agree⟩, edge⟩, mask_bound⟩, signature⟩ := checked
      split at signature
      · rename_i parameters resultTypes signature_eq
        simp only [Bool.and_eq_true, beq_iff_eq] at signature
        obtain ⟨⟨⟨size_eq, flows⟩, open_eq⟩, packed⟩ := signature
        have arity' := (Array.mapM_size resolved).symm.trans arity
        exact .closure handle mask _ values.toArray openTypes resultType targetNs declaration
          semantic parameters resultTypes targetNs_eq declaration_eq
          (by rw [signatureTypes?_frameEnv resolved agree arity']; exact signature_eq)
          (FrameReads.call placed running.reads edge index_eq targetNs_eq resolved agree arity')
          mask_bound (by simpa using valuesTyped.flows (by simpa using size_eq) flows) open_eq
          packed
      · cases signature
    all_goals simp [resolved] at checked

/-- Invoking a typed closure on arguments of its open parameter types
returns to a running frame and a typed state, its results packed at the
closure's result type. -/
theorem Running.invoke {unit : ValidatedUnit} {loans : LoanTypes} {context : Context}
    {frame : RuntimeFrame} {state finalState : RuntimeState}
    {handle : FunctionHandle} {mask : Nat} {typeInstantiation : Array (TypeId × TypeId)}
    {captures : Array RuntimeValue} {values composed : List RuntimeValue}
    {parameters : List SemTy} {resultType : SemTy} {outcome : Outcome}
    (running : Running unit loans context frame)
    (stateTyped : TypedState unit loans inert state)
    (closureTyped : HasType unit loans (.closure handle mask typeInstantiation captures)
      (.function parameters resultType))
    (valuesTyped : HasTypes unit loans values parameters)
    (compose_eq : ClosureMask.compose mask captures.toList values = some composed)
    (callee : FunctionPreserves unit handle typeInstantiation state composed.toArray finalState
      outcome) :
    Preserved unit loans inert context (applyPendingFrom state.pending frame finalState).1
      (applyPendingFrom state.pending frame finalState).2 fun loans' =>
        ∀ results, outcome = .returned results →
          HasType unit loans' (packResults results) resultType := by
  cases closureTyped with
  | closure _ _ _ _ _ _ targetNs declaration arguments allParameters results namespace_eq
      declaration_eq signature_eq faithful mask_bound captures_typed parameters_eq packed =>
    subst parameters_eq
    have composedTyped := HasTypes.compose
      (by rw [signatureTypes?_length signature_eq]; exact mask_bound) captures_typed valuesTyped
      compose_eq
    obtain ⟨loans', extends', stateTyped', results_typed⟩ :=
      callee loans inert targetNs declaration arguments allParameters results namespace_eq
        declaration_eq faithful signature_eq (by simpa using composedTyped) stateTyped
    obtain ⟨running', stateTyped''⟩ := running.returned stateTyped extends' stateTyped'
    exact ⟨loans', extends', running', stateTyped'', fun results outcome_eq =>
      packResults_typed (results_typed results outcome_eq) packed⟩

/-- A checked invocation from a running frame on typed operands. -/
theorem Running.invokeChecked {unit : ValidatedUnit} {loans : LoanTypes} {context : Context}
    {frame : RuntimeFrame} {state finalState : RuntimeState}
    {instantiations : Array GenericArgument} {arguments : Array ExprId}
    {handle : FunctionHandle} {mask : Nat} {typeInstantiation : Array (TypeId × TypeId)}
    {captures : Array RuntimeValue} {values composed : List RuntimeValue} {type : SemTy}
    {outcome : Outcome}
    (running : Running unit loans context frame)
    (stateTyped : TypedState unit loans inert state)
    (valuesTyped : ValuesTyped unit loans context arguments.toList
      (.closure handle mask typeInstantiation captures :: values))
    (checked : context.checkCall type .invoke instantiations arguments = true)
    (compose_eq : ClosureMask.compose mask captures.toList values = some composed)
    (callee : FunctionPreserves unit handle typeInstantiation state composed.toArray finalState
      outcome) :
    Preserved unit loans inert context (applyPendingFrom state.pending frame finalState).1
      (applyPendingFrom state.pending frame finalState).2 fun loans' =>
        ∀ results, outcome = .returned results →
          HasType unit loans' (packResults results) type := by
  simp only [Context.checkCall] at checked
  cases list_eq : arguments.toList with
  | nil => rw [list_eq] at valuesTyped; simp [ValuesTyped] at valuesTyped
  | cons calleeId supplied =>
    rw [list_eq] at valuesTyped checked
    obtain ⟨⟨-, closureType, closureType_eq, closureTyped⟩, suppliedTyped⟩ := valuesTyped
    simp only [closureType_eq] at checked
    cases closureType
    case function parameters resultType =>
      simp only [Bool.and_eq_true, beq_iff_eq] at checked
      obtain ⟨⟨length_eq, flows⟩, rfl⟩ := checked
      exact running.invoke stateTyped closureTyped (suppliedTyped.flows length_eq flows)
        compose_eq callee
    all_goals simp at checked

/-- Operands that produced values are typed at the types the checker reads
for them. -/
theorem ValuesTyped.types {unit : ValidatedUnit} {loans : LoanTypes} {context : Context} :
    ∀ {ids : List ExprId} {values : List RuntimeValue} {types : List SemTy},
      ValuesTyped unit loans context ids values → ids.mapM context.exprType = some types →
        HasTypes unit loans values types
  | [], [], types, _, mapped => by
      simp only [List.mapM_nil, Option.pure_def, Option.some.injEq] at mapped
      subst mapped
      exact .nil
  | _ :: _, _ :: _, types, ⟨head, tail⟩, mapped => by
      simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff,
        Option.pure_def, Option.some.injEq] at mapped
      obtain ⟨type, type_eq, rest, rest_eq, rfl⟩ := mapped
      obtain ⟨-, type', type'_eq, typed⟩ := head
      rw [type'_eq, Option.some.injEq] at type_eq
      subst type_eq
      exact .cons typed (ValuesTyped.types tail rest_eq)
  | [], _ :: _, _, typed, _ | _ :: _, [], _, typed, _ => by cases typed

/-- A frame reads a type it consults as the type the context resolves. -/
theorem FrameReads.resolves {context : Context} {frame : RuntimeFrame} {typeId : TypeId}
    {type : SemTy} (reads : FrameReads context frame) (consults : context.consults typeId = true)
    (typed : context.typeOf typeId = some type) :
    Resolves context.ns.tables #[] (instantiatedTypeId frame.typeInstantiation typeId) type := by
  have resolved : Resolves context.ns.tables context.env typeId type := ⟨_, typed⟩
  unfold FrameReads at reads
  unfold Context.consults at consults
  split at reads
  · rename_i required required_eq
    rw [required_eq, Array.contains_iff_mem] at consults
    exact reads.read consults resolved
  · obtain ⟨instantiation_eq, env_eq⟩ := reads
    rw [instantiation_eq, instantiatedTypeId_empty]
    rw [env_eq] at resolved
    exact resolved

theorem borrowRuntimePlace?_instantiation {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {site : ExprId} {referenceType : ReferenceType} {kind : BorrowKind}
    {frame frame' : RuntimeFrame} {state state' : RuntimeState} {place : RuntimePlace}
    {value : RuntimeValue}
    (borrow : borrowRuntimePlace? unit ns site referenceType kind frame state place =
      some (frame', state', value)) :
    frame'.typeInstantiation = frame.typeInstantiation := by
  cases kind with
  | profile _ => simp [borrowRuntimePlace?] at borrow
  | immutable =>
    simp only [borrowRuntimePlace?, Option.bind_eq_bind, Option.bind_some] at borrow
    split at borrow
    · cases borrow
    obtain ⟨read, -, borrow⟩ := Option.bind_eq_some_iff.mp borrow
    simp only [Option.some.injEq, Prod.mk.injEq] at borrow
    rw [← borrow.1]
  | mutable =>
    simp only [borrowRuntimePlace?, Option.bind_eq_bind, Option.bind_some] at borrow
    split at borrow
    · cases borrow
    obtain ⟨read, -, borrow⟩ := Option.bind_eq_some_iff.mp borrow
    obtain ⟨lexical, -, borrow⟩ := Option.bind_eq_some_iff.mp borrow
    obtain ⟨⟨written, writtenState⟩, write, borrow⟩ := Option.bind_eq_some_iff.mp borrow
    have kept := writeRuntimePlace?_instantiation write
    simp only [Option.some.injEq, Prod.mk.injEq] at borrow
    rw [← borrow.1]
    exact kept

theorem evaluateGlobalOperation?_instantiation {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId} {kind : GlobalKind} {resource : TypeUse}
    {arguments : Array RuntimeValue} {frame frame' : RuntimeFrame} {state state' : RuntimeState}
    {value : RuntimeValue}
    (eval : evaluateGlobalOperation? unit ns resultType site kind #[.typeArg resource] arguments
      frame state = some (.value frame' state' value)) :
    frame'.typeInstantiation = frame.typeInstantiation := by
  rw [evaluateGlobalOperation?_typeArg] at eval
  cases kind with
  | contains | take | publish =>
      dsimp only at eval
      split at eval
      · simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at eval
        obtain ⟨_, -, eval⟩ := eval
        (try split at eval) <;>
          simp only [Option.some.injEq, GlobalOperationResult.value.injEq, reduceCtorEq] at eval <;>
          rw [← eval.1]
      · cases eval
  | borrow borrowKind =>
      dsimp only at eval
      split at eval
      · simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at eval
        obtain ⟨_, -, eval⟩ := eval
        split at eval
        · simp at eval
        · obtain ⟨node, -, eval⟩ := Option.bind_eq_some_iff.mp eval
          split at eval
          · obtain ⟨⟨borrowFrame, borrowState, borrowed⟩, borrow, eval⟩ :=
              Option.bind_eq_some_iff.mp eval
            simp only [Option.some.injEq, GlobalOperationResult.value.injEq] at eval
            rw [← eval.1]
            exact borrowRuntimePlace?_instantiation borrow
          · cases eval
      · cases eval

theorem ValuesTyped.single {unit : ValidatedUnit} {loans : LoanTypes} {context : Context}
    {ids : List ExprId} {values : List RuntimeValue} {id : ExprId}
    (typed : ValuesTyped unit loans context ids values) (ids_eq : ids = [id]) :
    ∃ value, values = [value] ∧ ValueTyped unit loans context id value := by
  subst ids_eq
  match values, typed with
  | [value], ⟨head, _⟩ => exact ⟨value, rfl, head⟩

theorem ValuesTyped.pair {unit : ValidatedUnit} {loans : LoanTypes} {context : Context}
    {ids : List ExprId} {values : List RuntimeValue} {first second : ExprId}
    (typed : ValuesTyped unit loans context ids values) (ids_eq : ids = [first, second]) :
    ∃ x y, values = [x, y] ∧ ValueTyped unit loans context first x ∧
      ValueTyped unit loans context second y := by
  subst ids_eq
  match values, typed with
  | [x, y], ⟨head, second, _⟩ => exact ⟨x, y, rfl, head, second⟩

/-- A checked global operation from a running frame on typed operands. -/
theorem Running.global {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {namespaceId : NamespaceId} {loans : LoanTypes}
    {context : Context} {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {resultType : TypeId} {kind : GlobalKind} {instantiations : Array GenericArgument}
    {arguments : Array ExprId} {values : List RuntimeValue} {type : SemTy}
    {result : GlobalOperationResult}
    (placed : Placed executable namespaceId context) (running : Running unit loans context frame)
    (stateTyped : TypedState unit loans inert state)
    (valuesTyped : ValuesTyped unit loans context arguments.toList values)
    (checked : context.checkOperation type (.global kind) instantiations arguments = true)
    (evaluate_eq : evaluateGlobalOperation? unit context.ns resultType exprId kind
      instantiations values.toArray frame state = some result) :
    GlobalResultTyped unit loans inert context.ns context.locals context.env frame state type
      result ∧ ∃ resource, instantiations = #[.typeArg resource] := by
  simp only [Context.checkOperation] at checked
  split at checked
  · rename_i resource instantiations_eq
    have array_eq : instantiations = #[.typeArg resource] := by
      apply Array.toList_inj.mp
      simpa using instantiations_eq
    subst array_eq
    refine ⟨?_, resource, rfl⟩
    simp only [Bool.and_eq_true] at checked
    obtain ⟨consults, checked⟩ := checked
    split at checked
    · rename_i resourceType resourceType_eq
      have resolves := running.reads.resolves consults resourceType_eq
      have ns_eq := placed.identity
      split at checked
      · simp only [beq_iff_eq] at checked
        subst checked
        exact evaluateGlobal_contains_typed running.typed stateTyped evaluate_eq
      · split at checked
        · rename_i referenceKind kind_eq
          simp only [beq_iff_eq] at checked
          subst checked
          exact evaluateGlobal_borrow_typed ns_eq resolves kind_eq running.typed stateTyped
            evaluate_eq
        · cases checked
      · simp only [beq_iff_eq] at checked
        subst checked
        exact evaluateGlobal_take_typed ns_eq resolves running.typed stateTyped evaluate_eq
      · rename_i key value operands_eq
        simp only [Bool.and_eq_true] at checked
        obtain ⟨flows, packed⟩ := checked
        refine evaluateGlobal_publish_typed ns_eq resolves running.typed stateTyped
          (fun key' value' values_eq => ?_) packed evaluate_eq
        obtain ⟨keyValue, storedValue, rfl, -, valueTyped⟩ := valuesTyped.pair operands_eq
        simp only [List.cons.injEq, and_true] at values_eq
        rw [← values_eq.2]
        exact valueTyped.flows flows
      · cases checked
    · cases checked
  · cases checked

/-- A constant the runtime resolves, the checker's lookup finds. -/
theorem constantTarget_of_resolve {unit : ValidatedUnit} {sourceNamespace : NamespaceId}
    {ns : ValidatedNamespace} {reference : QualifiedRef} {handle : ConstantHandle}
    (ns_eq : unit.namespaces[sourceNamespace.index]? = some ns)
    (resolve : resolveConstant? unit sourceNamespace reference = some handle) :
    ∃ targetNs declaration,
      constantTarget? unit ns reference = some (targetNs, declaration) ∧
      unit.namespaces[handle.namespaceId.index]? = some targetNs ∧
      targetNs.constants[handle.constantId]? = some declaration := by
  simp only [resolveConstant?, ns_eq, Option.bind_eq_bind, Option.bind_some] at resolve
  obtain ⟨qualified, qualified_eq, resolve⟩ := Option.bind_eq_some_iff.mp resolve
  split at resolve
  · cases resolve
  rename_i same
  simp only [bne_iff_ne, ne_eq, Decidable.not_not] at same
  obtain ⟨constantId, constantId_eq, resolve⟩ := Option.bind_eq_some_iff.mp resolve
  obtain ⟨targetNs, targetNs_eq, resolve⟩ := Option.bind_eq_some_iff.mp resolve
  obtain ⟨declaration, declaration_eq, resolve⟩ := Option.bind_eq_some_iff.mp resolve
  simp only [pure, Option.some.injEq] at resolve
  subst resolve
  refine ⟨targetNs, declaration, ?_, targetNs_eq, declaration_eq⟩
  simp [constantTarget?, qualified_eq, same, constantId_eq, targetNs_eq, declaration_eq]

/-- A constant's initializer runs checked in its constant context, from an
empty frame. -/
theorem constant_initializer {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {handle : ConstantHandle}
    {targetNs : ValidatedNamespace} {declaration : ConstantDecl} {loans : LoanTypes}
    {state finalState : RuntimeState} {targetFrame : RuntimeFrame} {control : Control}
    (target_namespace_eq : unit.namespaces[handle.namespaceId.index]? = some targetNs)
    (declaration_eq : targetNs.constants[handle.constantId]? = some declaration)
    (ih : ExprPreserves executable handle.namespaceId { locals := #[] } state declaration.value
      targetFrame finalState control)
    (stateTyped : TypedState unit loans inert state) :
    ∃ table type, resolveIn targetNs #[] declaration.type.typeId = some type ∧
      (constantContext unit executable.targetPointerWidth targetNs table).flows
        declaration.value type = true ∧
      Preserved unit loans inert (constantContext unit executable.targetPointerWidth targetNs table)
        targetFrame finalState fun loans' => ControlTyped unit loans'
          (constantContext unit executable.targetPointerWidth targetNs table) declaration.value
          control := by
  obtain ⟨-, -, -, -, table, table_eq, -, constants⟩ :=
    checkUnit_namespace executable.typed target_namespace_eq
  have checked := constants declaration (Array.mem_of_getElem? declaration_eq)
  unfold checkConstant at checked
  split at checked
  · rename_i type type_eq
    simp only [Bool.and_eq_true] at checked
    refine ⟨table, type, type_eq, checked.1, ih loans inert _ _ ⟨rfl, rfl, target_namespace_eq, table_eq⟩
      checked.2 ⟨⟨rfl, fun _ _ _ declaration_eq _ => by simp [constantContext] at declaration_eq⟩,
        rfl, rfl⟩
      stateTyped⟩
  · cases checked

theorem sharedOperandAt_eq {ns : ValidatedNamespace} {site : ExprId} {expression : Expr}
    {operation : Operation} {instantiations : Array GenericArgument} {operands : Array ExprId}
    {surface : Option SurfaceSyntax} {reference : ExprId}
    (expression_eq : ns.expressions[site.index]? = some expression)
    (kind_eq : expression.kind = .operation operation instantiations operands surface)
    (operands_eq : operands.toList = [reference]) :
    sharedOperandAt ns site = Context.sharedNode ns reference := by
  obtain ⟨loc, typeId, kind⟩ := expression
  simp only at kind_eq
  subst kind_eq
  have first : operands[0]? = some reference := by
    rw [← Array.getElem?_toList, operands_eq]
    rfl
  simp only [sharedOperandAt, expression_eq, sharedOperand, first, Context.sharedNode,
    isSharedReferenceType]
  split <;> rename_i found <;> simp only [found]
  split <;> rename_i node <;> simp only [node]

theorem HasType.mutable_current {unit : ValidatedUnit} {loans : LoanTypes} {loan : Nat}
    {current : RuntimeValue} {referent : SemTy}
    (typed : HasType unit loans (.borrow loan current) (.reference .mutable referent)) :
    HasType unit loans current referent := by
  cases typed with
  | borrow _ _ _ _ current_typed => exact current_typed

theorem updateBorrowValue?_instantiation {frame frame' : RuntimeFrame}
    {state state' : RuntimeState} {loan : Nat} {replacement : RuntimeValue}
    (update : updateBorrowValue? frame state loan replacement = some (frame', state')) :
    frame'.typeInstantiation = frame.typeInstantiation := by
  unfold updateBorrowValue? at update
  split at update
  · rename_i updated local_eq
    simp only [Option.some.injEq] at update
    subst update
    simp only [updateLocalBorrowValue?, Option.bind_eq_bind, Option.bind_eq_some_iff] at local_eq
    obtain ⟨_, -, _, -, _, -, write⟩ := local_eq
    exact writeRuntimePlace?_instantiation write
  · dsimp only at update
    split at update
    · simp only [Option.map_eq_some_iff] at update
      obtain ⟨_, -, update⟩ := update
      simp only [Prod.mk.injEq] at update
      rw [← update.1]
    · split at update
      · simp only [Option.bind_eq_some_iff, Option.map_eq_some_iff] at update
        obtain ⟨_, -, _, -, update⟩ := update
        simp only [Prod.mk.injEq] at update
        rw [← update.1]
      · cases update

theorem mutateBorrow?_instantiation {arguments : Array RuntimeValue} {frame frame' : RuntimeFrame}
    {state state' : RuntimeState} {result : RuntimeValue}
    (mutate : mutateBorrow? arguments frame state = some (frame', state', result)) :
    frame'.typeInstantiation = frame.typeInstantiation := by
  unfold mutateBorrow? at mutate
  split at mutate
  · split at mutate
    · rename_i updatedFrame updatedState update_eq
      simp only [Option.some.injEq, Prod.mk.injEq] at mutate
      obtain ⟨rfl, rfl, rfl⟩ := mutate
      exact updateBorrowValue?_instantiation update_eq
    · simp only [Option.some.injEq, Prod.mk.injEq] at mutate
      obtain ⟨rfl, rfl, rfl⟩ := mutate
      exact applyWriteBack_instantiation _ _ _ _
  · cases mutate

/-- A struct the checker resolves has distinct variants. -/
theorem structTarget_distinct {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {ns targetNs : ValidatedNamespace}
    {reference : QualifiedRef} {declaration : StructDecl} {spelled : QualifiedName}
    (target : structTarget? unit ns reference = some (targetNs, declaration, spelled)) :
    variantsDistinct targetNs declaration = true := by
  simp only [structTarget?, Option.bind_eq_bind, Option.bind_eq_some_iff] at target
  obtain ⟨⟨targetNs', declaration'⟩, declared, _, -, _, -, target⟩ := target
  simp only [Option.some.injEq, Prod.mk.injEq] at target
  obtain ⟨rfl, rfl, -⟩ := target
  simp only [declarationTarget?, Option.bind_eq_bind, Option.bind_eq_some_iff] at declared
  obtain ⟨qualified, -, declared⟩ := declared
  split at declared
  · cases declared
  obtain ⟨typeId, -, declared⟩ := Option.bind_eq_some_iff.mp declared
  obtain ⟨targetNs'', targetNs_eq, declared⟩ := Option.bind_eq_some_iff.mp declared
  obtain ⟨declaration'', declaration_eq, declared⟩ := Option.bind_eq_some_iff.mp declared
  simp only [Option.some.injEq, Prod.mk.injEq] at declared
  obtain ⟨rfl, rfl⟩ := declared
  have distinct := (checkUnit_namespace executable.typed targetNs_eq).2.2.2.1
  exact Array.all_eq_true'.mp distinct _ (Array.mem_of_getElem? declaration_eq)

theorem TypedFrame.clear {unit : ValidatedUnit} {loans : LoanTypes} {ns : ValidatedNamespace}
    {locals : Array LocalDecl} {env : Array SemArg} {frame : RuntimeFrame}
    (typed : TypedFrame unit loans ns locals env frame) (index : Nat) :
    TypedFrame unit loans ns locals env { frame with locals := frame.locals.set! index none } := by
  obtain ⟨size_eq, locals_typed⟩ := typed
  refine ⟨by simp [size_eq], fun index' declaration value declaration_eq value_eq => ?_⟩
  simp only [Array.set!_eq_setIfInBounds, Array.getElem?_setIfInBounds] at value_eq
  split at value_eq
  · split at value_eq <;> simp at value_eq
  · exact locals_typed index' declaration value declaration_eq value_eq

/-- A context's place lookup types a resolved place under every loans. -/
theorem Placed.place {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {namespaceId : NamespaceId} {context : Context}
    {frame : RuntimeFrame} {state : RuntimeState} {place : PlaceId} {type : SemTy}
    {resolved : RuntimePlace} (placed : Placed executable namespaceId context)
    (typed : context.placeTypeOf place = some type)
    (resolve : resolvePlace? unit context.ns frame state place = some resolved) :
    ∃ variant, ∀ loans, TypedFrame unit loans context.ns context.locals context.env frame →
      PlaceTyped unit loans context frame resolved type variant := by
  simp only [Context.placeTypeOf, Functor.map, Option.map_eq_some_iff] at typed
  obtain ⟨⟨type', variant⟩, placeType_eq, rfl⟩ := typed
  exact ⟨variant, fun loans frameTyped => resolvePlace_typed placed.unit_eq placed.identity
    frameTyped _ _ placeType_eq resolve⟩

/-- A checked place, reference, or data operation from a running frame on
typed operands produces a value of the node's type in a running frame and a
typed state. -/
theorem Running.placeOperation {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {namespaceId : NamespaceId}
    {loans : LoanTypes}
    {context : Context} {frame finalFrame : RuntimeFrame} {state finalState : RuntimeState}
    {exprId : ExprId} {expression : Expr} {operation : Operation}
    {instantiations : Array GenericArgument} {operands : Array ExprId}
    {surface : Option SurfaceSyntax} {values : List RuntimeValue} {type : SemTy}
    {value : RuntimeValue}
    (placed : Placed executable namespaceId context) (running : Running unit loans context frame)
    (stateTyped : TypedState unit loans inert state)
    (expression_eq : context.ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation operation instantiations operands surface)
    (type_eq : context.typeOf expression.typeId = some type)
    (valuesTyped : ValuesTyped unit loans context operands.toList values)
    (checked : context.checkOperation type operation instantiations operands = true)
    (evaluate_eq : evaluatePlaceOperation? unit context.ns expression.typeId exprId operation
      values.toArray frame state = some (finalFrame, finalState, value)) :
    Preserved unit loans inert context finalFrame finalState fun loans' =>
      HasType unit loans' value type := by
  cases operation with
  | copy place | read place =>
      simp only [Context.checkOperation, Bool.and_eq_true, beq_iff_eq] at checked
      simp only [evaluatePlaceOperation?, Option.bind_eq_bind] at evaluate_eq
      split at evaluate_eq
      · cases evaluate_eq
      obtain ⟨resolved, resolve_eq, evaluate_eq⟩ := Option.bind_eq_some_iff.mp evaluate_eq
      obtain ⟨read, read_eq, evaluate_eq⟩ := Option.bind_eq_some_iff.mp evaluate_eq
      simp only [Option.some.injEq, Prod.mk.injEq] at evaluate_eq
      obtain ⟨rfl, rfl, rfl⟩ := evaluate_eq
      obtain ⟨variant, placeTyped⟩ := running.place placed checked.2 resolve_eq
      exact ⟨loans, .refl _, running, stateTyped, placeTyped.read read_eq⟩
  | move place =>
      simp only [Context.checkOperation, Bool.and_eq_true, beq_iff_eq] at checked
      simp only [evaluatePlaceOperation?, Option.bind_eq_bind] at evaluate_eq
      split at evaluate_eq
      · cases evaluate_eq
      obtain ⟨resolved, resolve_eq, evaluate_eq⟩ := Option.bind_eq_some_iff.mp evaluate_eq
      obtain ⟨read, read_eq, evaluate_eq⟩ := Option.bind_eq_some_iff.mp evaluate_eq
      obtain ⟨variant, placeTyped⟩ := running.place placed checked.2 resolve_eq
      have value_typed := placeTyped.read read_eq
      split at evaluate_eq
      · rename_i localId root_eq
        split at evaluate_eq
        · simp only [Option.some.injEq, Prod.mk.injEq] at evaluate_eq
          obtain ⟨rfl, rfl, rfl⟩ := evaluate_eq
          exact ⟨loans, .refl _, running.of_typed (running.typed.clear _) rfl, stateTyped,
            value_typed⟩
        · simp only [Option.some.injEq, Prod.mk.injEq] at evaluate_eq
          obtain ⟨rfl, rfl, rfl⟩ := evaluate_eq
          exact ⟨loans, .refl _, running, stateTyped, value_typed⟩
      · cases evaluate_eq
  | borrow kind place =>
      simp only [Context.checkOperation, Bool.and_eq_true] at checked
      obtain ⟨-, checked⟩ := checked
      simp only [evaluatePlaceOperation?, Option.bind_eq_bind] at evaluate_eq
      split at evaluate_eq
      · cases evaluate_eq
      obtain ⟨node, node_eq, evaluate_eq⟩ := Option.bind_eq_some_iff.mp evaluate_eq
      split at evaluate_eq
      · rename_i referenceType
        obtain ⟨resolved, resolve_eq, borrow_eq⟩ := Option.bind_eq_some_iff.mp evaluate_eq
        split at checked
        · rename_i referenceKind referent kind_eq placeType_eq
          simp only [beq_iff_eq] at checked
          subst checked
          obtain ⟨variant, placeTyped⟩ := placed.place placeType_eq resolve_eq
          have kept := borrowRuntimePlace?_instantiation borrow_eq
          cases kind with
          | immutable =>
              simp only [borrowedKind, Option.some.injEq] at kind_eq
              subst kind_eq
              obtain ⟨rfl, rfl, typed⟩ :=
                borrowRuntimePlace?_shared_typed (placeTyped loans running.typed) borrow_eq
              exact ⟨loans, .refl _, running, stateTyped, typed⟩
          | mutable =>
              simp only [borrowedKind, Option.some.injEq] at kind_eq
              subst kind_eq
              obtain ⟨frameTyped, stateTyped', typed, -⟩ :=
                borrowRuntimePlace?_mutable_typed running.typed stateTyped placeTyped borrow_eq
              exact ⟨_, stateTyped.mint.1, running.of_typed frameTyped kept, stateTyped', typed⟩
          | profile _ => simp [borrowedKind] at kind_eq
        · cases checked
      · cases evaluate_eq
  | write place =>
      simp only [Context.checkOperation] at checked
      split at checked
      · rename_i operand placeType operands_eq placeType_eq
        simp only [Bool.and_eq_true] at checked
        obtain ⟨flows, packed⟩ := checked
        obtain ⟨argument, rfl, argumentTyped⟩ := valuesTyped.single operands_eq
        rw [evaluatePlaceOperation?_write] at evaluate_eq
        obtain ⟨resolved, resolve_eq, evaluate_eq⟩ := Option.bind_eq_some_iff.mp evaluate_eq
        obtain ⟨⟨written, writtenState⟩, write_eq, evaluate_eq⟩ :=
          Option.bind_eq_some_iff.mp evaluate_eq
        simp only [Option.some.injEq, Prod.mk.injEq] at evaluate_eq
        obtain ⟨rfl, rfl, rfl⟩ := evaluate_eq
        obtain ⟨variant, placeTyped⟩ := running.place placed placeType_eq resolve_eq
        obtain ⟨frameTyped, rfl⟩ := placeTyped.write running.typed (argumentTyped.flows flows)
          write_eq
        exact ⟨loans, .refl _, running.of_typed frameTyped (writeRuntimePlace?_instantiation write_eq),
          stateTyped, packResults_typed (values := #[]) .nil packed⟩
      · cases checked
  | drop place =>
      simp only [Context.checkOperation, Bool.and_eq_true] at checked
      obtain ⟨⟨-, -⟩, packed⟩ := checked
      simp only [evaluatePlaceOperation?, Option.bind_eq_bind] at evaluate_eq
      split at evaluate_eq
      · cases evaluate_eq
      obtain ⟨resolved, resolve_eq, evaluate_eq⟩ := Option.bind_eq_some_iff.mp evaluate_eq
      split at evaluate_eq
      · split at evaluate_eq
        · simp only [Option.some.injEq, Prod.mk.injEq] at evaluate_eq
          obtain ⟨rfl, rfl, rfl⟩ := evaluate_eq
          exact ⟨loans, .refl _, running.of_typed (running.typed.clear _) rfl, stateTyped,
            packResults_typed (values := #[]) .nil packed⟩
        · simp only [Option.some.injEq, Prod.mk.injEq] at evaluate_eq
          obtain ⟨rfl, rfl, rfl⟩ := evaluate_eq
          exact ⟨loans, .refl _, running, stateTyped, packResults_typed (values := #[]) .nil packed⟩
      · cases evaluate_eq
  | reference referenceOperation =>
      cases referenceOperation
      case dereference =>
          simp only [Context.checkOperation] at checked
          cases operands_eq : operands.toList with
          | nil => rw [operands_eq] at checked; simp at checked
          | cons reference rest =>
            cases rest with
            | cons _ _ => rw [operands_eq] at checked; simp at checked
            | nil =>
              rw [operands_eq] at checked
              simp only at checked
              obtain ⟨argument, rfl, -, argumentType, argumentType_eq, typed⟩ :=
                valuesTyped.single operands_eq
              rw [argumentType_eq] at checked
              cases argumentType
              case reference kind referent =>
                simp only [Bool.and_eq_true, beq_iff_eq] at checked
                obtain ⟨rfl, agree⟩ := checked
                rw [evaluatePlaceOperation?_dereference,
                  sharedOperandAt_eq expression_eq kind_eq operands_eq] at evaluate_eq
                split at evaluate_eq
                · rename_i shared
                  simp only [Option.some.injEq, Prod.mk.injEq] at evaluate_eq
                  obtain ⟨rfl, rfl, rfl⟩ := evaluate_eq
                  rw [shared] at agree
                  cases kind with
                  | shared => exact ⟨loans, .refl _, running, stateTyped, typed.shared_referent⟩
                  | mutable => exact absurd agree (by decide)
                · rename_i notShared
                  rw [Bool.not_eq_true] at notShared
                  rw [notShared] at agree
                  cases kind with
                  | shared => exact absurd agree (by decide)
                  | mutable =>
                    cases argument
                    case borrow loan current =>
                      simp only [dereferenceBorrow?, Option.some.injEq,
                        Prod.mk.injEq] at evaluate_eq
                      obtain ⟨rfl, rfl, rfl⟩ := evaluate_eq
                      exact ⟨loans, .refl _, running, stateTyped, typed.mutable_current⟩
                    all_goals simp [dereferenceBorrow?] at evaluate_eq
              all_goals simp at checked
      case freeze explicit =>
          simp only [Context.checkOperation] at checked
          cases operands_eq : operands.toList with
          | nil => rw [operands_eq] at checked; simp at checked
          | cons reference rest =>
            cases rest with
            | cons _ _ => rw [operands_eq] at checked; simp at checked
            | nil =>
              rw [operands_eq] at checked
              simp only at checked
              obtain ⟨argument, rfl, -, argumentType, argumentType_eq, typed⟩ :=
                valuesTyped.single operands_eq
              rw [argumentType_eq] at checked
              cases argumentType
              case reference kind referent =>
                simp only [Bool.and_eq_true, beq_iff_eq] at checked
                obtain ⟨rfl, agree⟩ := checked
                rw [evaluatePlaceOperation?_freeze,
                  sharedOperandAt_eq expression_eq kind_eq operands_eq] at evaluate_eq
                split at evaluate_eq
                · rename_i shared
                  simp only [Option.some.injEq, Prod.mk.injEq] at evaluate_eq
                  obtain ⟨rfl, rfl, rfl⟩ := evaluate_eq
                  rw [shared] at agree
                  cases kind with
                  | shared => exact ⟨loans, .refl _, running, stateTyped, typed⟩
                  | mutable => exact absurd agree (by decide)
                · rename_i notShared
                  rw [Bool.not_eq_true] at notShared
                  rw [notShared] at agree
                  cases kind with
                  | shared => exact absurd agree (by decide)
                  | mutable =>
                    obtain ⟨node, -, evaluate_eq⟩ := Option.bind_eq_some_iff.mp evaluate_eq
                    split at evaluate_eq
                    · rename_i resultReference
                      rw [freezeBorrow?_single] at evaluate_eq
                      cases argument
                      case borrow loan current =>
                        simp only at evaluate_eq
                        split at evaluate_eq
                        · cases evaluate_eq
                        · simp only [Option.some.injEq, Prod.mk.injEq] at evaluate_eq
                          obtain ⟨rfl, rfl, rfl⟩ := evaluate_eq
                          exact ⟨loans, .refl _, running, stateTyped,
                            .shared _ _ typed.mutable_current⟩
                      all_goals simp at evaluate_eq
                    · cases evaluate_eq
              all_goals simp at checked
      case mutate =>
          simp only [Context.checkOperation] at checked
          cases operands_eq : operands.toList with
          | nil => rw [operands_eq] at checked; simp at checked
          | cons reference rest =>
            cases rest with
            | nil => rw [operands_eq] at checked; simp at checked
            | cons operand rest =>
              cases rest with
              | cons _ _ => rw [operands_eq] at checked; simp at checked
              | nil =>
                rw [operands_eq] at checked
                simp only at checked
                obtain ⟨target, argument, rfl, ⟨-, targetType, targetType_eq, targetTyped⟩,
                  argumentTyped⟩ := valuesTyped.pair operands_eq
                rw [targetType_eq] at checked
                cases targetType
                case reference kind referent =>
                  cases kind
                  case mutable =>
                    simp only [Bool.and_eq_true] at checked
                    obtain ⟨flows, packed⟩ := checked
                    rw [evaluatePlaceOperation?_mutate] at evaluate_eq
                    obtain ⟨frameTyped, stateTyped', typed⟩ := mutateBorrow?_typed running.typed
                      stateTyped targetTyped (argumentTyped.flows flows) packed evaluate_eq
                    exact ⟨loans, .refl _, running.of_typed frameTyped
                      (mutateBorrow?_instantiation evaluate_eq), stateTyped', typed⟩
                  all_goals simp at checked
                all_goals simp at checked
      case borrow kind =>
          rw [show evaluatePlaceOperation? unit context.ns expression.typeId exprId
            (.reference (.borrow kind)) values.toArray frame state = none from rfl] at evaluate_eq
          cases evaluate_eq
  | data dataOperation =>
      rw [evaluatePlaceOperation?_data] at evaluate_eq
      obtain ⟨result, eval_eq, evaluate_eq⟩ := Option.bind_eq_some_iff.mp evaluate_eq
      simp only [Option.some.injEq, Prod.mk.injEq] at evaluate_eq
      obtain ⟨rfl, rfl, rfl⟩ := evaluate_eq
      refine ⟨loans, .refl _, running, stateTyped, ?_⟩
      simp only [Context.checkOperation] at checked
      cases dataOperation
      case select reference field =>
          simp only [Context.checkData, placed.unit_eq] at checked
          cases target : structTarget? unit context.ns reference with
          | none => simp [target] at checked
          | some found =>
            obtain ⟨targetNs, declaration, spelled⟩ := found
            cases operands_eq : operands.toList with
            | nil => simp [target, operands_eq] at checked
            | cons operand rest =>
              cases rest with
              | cons _ _ => simp [target, operands_eq] at checked
              | nil =>
                simp only [target, operands_eq] at checked
                obtain ⟨argument, rfl, -, operandType, operandType_eq, typed⟩ :=
                  valuesTyped.single operands_eq
                rw [operandType_eq] at checked
                simp only at checked
                split at checked
                · rename_i name arguments operand_eq
                  simp only [Bool.and_eq_true, beq_iff_eq] at checked
                  obtain ⟨rfl, checked⟩ := checked
                  split at checked
                  · rename_i types types_eq
                    simp only [Bool.and_eq_true] at checked
                    exact select_typed placed.identity target operand_eq types_eq checked.2 typed
                      eval_eq
                  · cases checked
                · cases checked
      case selectVariants reference fields =>
          simp only [Context.checkData, placed.unit_eq] at checked
          cases target : structTarget? unit context.ns reference with
          | none => simp [target] at checked
          | some found =>
            obtain ⟨targetNs, declaration, spelled⟩ := found
            cases operands_eq : operands.toList with
            | nil => simp [target, operands_eq] at checked
            | cons operand rest =>
              cases rest with
              | cons _ _ => simp [target, operands_eq] at checked
              | nil =>
                simp only [target, operands_eq] at checked
                obtain ⟨argument, rfl, -, operandType, operandType_eq, typed⟩ :=
                  valuesTyped.single operands_eq
                rw [operandType_eq] at checked
                simp only at checked
                split at checked
                · rename_i name arguments operand_eq
                  simp only [Bool.and_eq_true, beq_iff_eq] at checked
                  obtain ⟨rfl, checked⟩ := checked
                  exact selectVariants_typed placed.identity target
                    (structTarget_distinct (executable := executable) target)
                    operand_eq checked typed eval_eq
                · cases checked
      case testVariants reference variants =>
          simp only [Context.checkData, beq_iff_eq] at checked
          subst checked
          exact testVariants_typed eval_eq
      case discriminant reference =>
          simp only [Context.checkData, Bool.and_eq_true] at checked
          rw [placed.width, placed.unit_eq] at checked
          exact discriminant_typed placed.identity checked.2 eval_eq
      case updateField reference field =>
          simp only [Context.checkData, placed.unit_eq] at checked
          cases target : structTarget? unit context.ns reference with
          | none => simp [target] at checked
          | some found =>
            obtain ⟨targetNs, declaration, spelled⟩ := found
            cases operands_eq : operands.toList with
            | nil => simp [target, operands_eq] at checked
            | cons operand rest =>
              cases rest with
              | nil => simp [target, operands_eq] at checked
              | cons replacement rest =>
                cases rest with
                | cons _ _ => simp [target, operands_eq] at checked
                | nil =>
                  simp only [target, operands_eq] at checked
                  obtain ⟨argument, replacementValue, rfl, ⟨-, operandType, operandType_eq, typed⟩,
                    replacementTyped⟩ := valuesTyped.pair operands_eq
                  rw [operandType_eq] at checked
                  cases operandType
                  case nominal name arguments =>
                    simp only [Bool.and_eq_true, beq_iff_eq] at checked
                    obtain ⟨⟨rfl, rfl⟩, checked⟩ := checked
                    split at checked
                    · rename_i types types_eq
                      exact updateField_typed placed.identity target types_eq typed
                        (fun type member => replacementTyped.flows
                          (List.all_eq_true.mp checked type member)) eval_eq
                    · cases checked
                  all_goals simp at checked
  | call _ | global _ | primitive _ | specification _ | assert | profile _ _ => cases evaluate_eq

/-- Natives preserve typing: assumed, since the runtime provides them. -/
def NativesTyped {unit : ValidatedUnit} (executable : ExecutableUnit unit) : Prop :=
  ∀ handle instantiation state arguments state' outcome,
    nativeCall executable handle instantiation state arguments = some (state', outcome) →
    FunctionPreserves unit handle instantiation state arguments state' outcome

theorem preservation {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    (natives : NativesTyped executable)
    {handle : FunctionHandle} {instantiation : Array (TypeId × TypeId)} {state : RuntimeState}
    {arguments : Array RuntimeValue} {state' : RuntimeState} {outcome : Outcome}
    (step : EvalFunction executable handle instantiation state arguments state' outcome) :
    FunctionPreserves unit handle instantiation state arguments state' outcome := by
  apply EvalFunction.rec
    (motive_1 := fun handle instantiation state arguments state' outcome _ =>
      FunctionPreserves unit handle instantiation state arguments state' outcome)
    (motive_2 := fun namespaceId frame state id frame' state' control _ =>
      ExprPreserves executable namespaceId frame state id frame' state' control)
    (motive_3 := fun namespaceId frame state id frame' state' control _ =>
      ExprPreserves executable namespaceId frame state id frame' state' control)
    (motive_4 := fun namespaceId frame state ids result _ =>
      ValuesPreserve executable namespaceId frame state ids result)
    (motive_5 := fun namespaceId frame state statements result _ =>
      StatementsPreserve executable namespaceId frame state statements result)
    (motive_6 := fun namespaceId ns frame state value arms frame' state' control _ =>
      ArmsPreserve executable namespaceId ns frame state value arms frame' state' control)
    (t := step)
  case node =>
    intro namespaceId frame state exprId startFrame startState nodeFrame nodeState control
      finalFrame finalState before_eq node_step after_eq ih
    intro loans inert context fuel placed checked running stateTyped
    have settled := settleLoans_typed
      (ended := (loanDeathsAt unit namespaceId exprId).before) running.typed stateTyped
    have kept := settleLoans_instantiation (loanDeathsAt unit namespaceId exprId).before
      frame state
    rw [before_eq] at settled kept
    obtain ⟨loans', extends', nodeRunning, nodeState', typed⟩ :=
      ih loans inert context fuel placed checked (running.of_typed settled.1 kept) settled.2
    have after := settleAfter_typed
      (ended := (loanDeathsAt unit namespaceId exprId).after) (control := control)
      nodeRunning.typed nodeState'
    have kept' := settleAfter_instantiation (loanDeathsAt unit namespaceId exprId).after
      control nodeFrame nodeState
    rw [after_eq] at after kept'
    exact ⟨loans', extends', nodeRunning.of_typed after.1 kept', after.2, typed⟩
  case value =>
    intro namespaceId frame state exprId ns expression literal source runtimeValue
      namespace_eq expression_eq kind_eq value_eq
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, -⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp only [type_eq, kind_eq] at nodeChecked
    refine ⟨loans, .refl _, running, stateTyped, diverts_false_of expression_eq (by simp [kind_eq]),
      type, exprType_of expression_eq type_eq, ?_⟩
    rw [placed.width] at nodeChecked
    exact constValue?_typed nodeChecked value_eq
  case localVar =>
    intro namespaceId frame state exprId ns expression localId runtimeValue
      namespace_eq expression_eq kind_eq local_eq
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, -⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp only [type_eq, kind_eq, beq_iff_eq] at nodeChecked
    exact ⟨loans, .refl _, running, stateTyped, diverts_false_of expression_eq (by simp [kind_eq]),
      type, exprType_of expression_eq type_eq, running.typed.read nodeChecked local_eq⟩
  case spec =>
    intro namespaceId frame state exprId ns expression block namespace_eq expression_eq kind_eq
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, -⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp only [type_eq, kind_eq] at nodeChecked
    exact ⟨loans, .refl _, running, stateTyped, diverts_false_of expression_eq (by simp [kind_eq]),
      type, exprType_of expression_eq type_eq, packResults_typed (values := #[]) .nil nodeChecked⟩
  case continue_ =>
    intro namespaceId frame state exprId ns expression nest namespace_eq expression_eq kind_eq
    intro loans inert context fuel placed checked running stateTyped
    exact ⟨loans, .refl _, running, stateTyped, trivial⟩
  case nil =>
    intro namespaceId frame state
    intro loans inert context fuel placed checked running stateTyped
    exact ⟨loans, .refl _, running, stateTyped, trivial⟩
  case headControl =>
    intro namespaceId frame state expression expressions finalFrame finalState control
      head_step abrupt ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨loans', extends', running', state', typed⟩ :=
      ih loans inert context fuel placed (checked expression (by simp)) running stateTyped
    exact ⟨loans', extends', running', state', abrupt, expression, by simp, typed⟩
  case tailValues =>
    intro namespaceId frame state expression expressions headFrame headState value finalFrame
      finalState values head_step tail_step ih_head ih_tail
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨loans₁, extends₁, running₁, state₁, head⟩ :=
      ih_head loans inert context fuel placed (checked expression (by simp)) running stateTyped
    obtain ⟨loans₂, extends₂, running₂, state₂, tail⟩ :=
      ih_tail loans₁ inert context fuel placed (fun id member => checked id (by simp [member]))
        running₁ state₁
    exact ⟨loans₂, extends₂.trans extends₁, running₂, state₂,
      ValueTyped.weaken extends₂ head, tail⟩
  case tailControl =>
    intro namespaceId frame state expression expressions headFrame headState value finalFrame
      finalState control head_step tail_step ih_head ih_tail
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨loans₁, extends₁, running₁, state₁, -⟩ :=
      ih_head loans inert context fuel placed (checked expression (by simp)) running stateTyped
    obtain ⟨loans₂, extends₂, running₂, state₂, abrupt, id, member, typed⟩ :=
      ih_tail loans₁ inert context fuel placed (fun id member => checked id (by simp [member]))
        running₁ state₁
    exact ⟨loans₂, extends₂.trans extends₁, running₂, state₂, abrupt, id, by simp [member],
      typed⟩
  case nil =>
    intro namespaceId frame state
    intro loans inert context fuel placed checked running stateTyped
    exact ⟨loans, .refl _, running, stateTyped, fun _ member => by simp at member⟩
  case headControl =>
    intro namespaceId frame state statement statements finalFrame finalState control
      head_step abrupt ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨loans', extends', running', state', typed⟩ :=
      ih loans inert context fuel placed (checked statement (by simp)) running stateTyped
    exact ⟨loans', extends', running', state', abrupt, statement, by simp, typed⟩
  case cons =>
    intro namespaceId frame state statement statements headFrame headState value result
      head_step tail_step ih_head ih_tail
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨loans₁, extends₁, running₁, state₁, head⟩ :=
      ih_head loans inert context fuel placed (checked statement (by simp)) running stateTyped
    have tail := ih_tail loans₁ inert context fuel placed (fun id member => checked id (by simp [member]))
      running₁ state₁
    cases result with
    | done state' frame' =>
        obtain ⟨loans₂, extends₂, running₂, state₂, rest⟩ := tail
        refine ⟨loans₂, extends₂.trans extends₁, running₂, state₂, fun id member => ?_⟩
        simp only [List.mem_cons] at member
        rcases member with rfl | member
        · exact ⟨value, ValueTyped.weaken extends₂ head⟩
        · exact rest id member
    | control state' frame' control =>
        obtain ⟨loans₂, extends₂, running₂, state₂, abrupt, id, member, typed⟩ := tail
        exact ⟨loans₂, extends₂.trans extends₁, running₂, state₂, abrupt, id, by simp [member],
          typed⟩
  case breakNone =>
    intro namespaceId frame state exprId ns expression nest namespace_eq expression_eq kind_eq
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, -⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp only [type_eq, kind_eq] at nodeChecked
    split at nodeChecked
    · rename_i loopType loopType_eq
      exact ⟨loans, .refl _, running, stateTyped, breaksTo_break expression_eq kind_eq, loopType,
        loopType_eq, packResults_typed (values := #[]) .nil nodeChecked⟩
    · cases nodeChecked
  case breakValue =>
    intro namespaceId frame state exprId ns expression nest child finalFrame finalState
      runtimeValue namespace_eq expression_eq kind_eq child_step ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    have childChecked := child_checkTree expression_eq (by simp [kind_eq, runsPlainly]) children
      (child := child) (by simp [kind_eq, expressionChildren])
    obtain ⟨loans', extends', running', state', childTyped⟩ :=
      ih loans inert context fuel' placed childChecked running stateTyped
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp only [type_eq, kind_eq] at nodeChecked
    split at nodeChecked
    · rename_i loopType loopType_eq
      exact ⟨loans', extends', running', state', breaksTo_break expression_eq kind_eq, loopType,
        loopType_eq, childTyped.flows nodeChecked⟩
    · cases nodeChecked
  case breakControl =>
    intro namespaceId frame state exprId ns expression nest child finalFrame finalState control
      namespace_eq expression_eq kind_eq child_step abrupt ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    have childChecked := child_checkTree expression_eq (by simp [kind_eq, runsPlainly]) children
      (child := child) (by simp [kind_eq, expressionChildren])
    obtain ⟨loans', extends', running', state', childTyped⟩ :=
      ih loans inert context fuel' placed childChecked running stateTyped
    exact ⟨loans', extends', running', state', childTyped.parent expression_eq
      (by simp [kind_eq, runsPlainly]) (by simp [kind_eq, expressionChildren]) abrupt⟩
  case returnValues =>
    intro namespaceId frame state exprId ns expression values finalFrame finalState
      runtimeValues namespace_eq expression_eq kind_eq values_step ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans', extends', running', state', valuesTyped⟩ :=
      ih loans inert context fuel' placed (fun id member => child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simpa [kind_eq, expressionChildren] using member))
        running stateTyped
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp only [type_eq, kind_eq] at nodeChecked
    split at nodeChecked
    · rename_i results results_eq
      simp only [Bool.and_eq_true, beq_iff_eq] at nodeChecked
      exact ⟨loans', extends', running', state', results, results_eq,
        by simpa using valuesTyped.flows (by simpa using nodeChecked.1) nodeChecked.2⟩
    · cases nodeChecked
  case returnControl =>
    intro namespaceId frame state exprId ns expression values finalFrame finalState control
      namespace_eq expression_eq kind_eq values_step ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨loans', extends', running', state', abrupt, id, member, typed⟩ :=
      ih loans inert context fuel' placed (fun id member => child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simpa [kind_eq, expressionChildren] using member))
        running stateTyped
    exact ⟨loans', extends', running', state', typed.parent expression_eq
      (by simp [kind_eq, runsPlainly]) (by simpa [kind_eq, expressionChildren] using member) abrupt⟩
  case throwValues =>
    intro namespaceId frame state exprId ns expression kind arguments finalFrame finalState
      runtimeValues namespace_eq expression_eq kind_eq values_step ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨loans', extends', running', state', -⟩ :=
      ih loans inert context fuel' placed (fun id member => child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simpa [kind_eq, expressionChildren] using member))
        running stateTyped
    exact ⟨loans', extends', running', state', trivial⟩
  case throwControl =>
    intro namespaceId frame state exprId ns expression kind arguments finalFrame finalState control
      namespace_eq expression_eq kind_eq values_step ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨loans', extends', running', state', abrupt, id, member, typed⟩ :=
      ih loans inert context fuel' placed (fun id member => child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simpa [kind_eq, expressionChildren] using member))
        running stateTyped
    exact ⟨loans', extends', running', state', typed.parent expression_eq
      (by simp [kind_eq, runsPlainly]) (by simpa [kind_eq, expressionChildren] using member) abrupt⟩
  case blockControl =>
    intro namespaceId frame state exprId ns expression statements result finalState finalFrame
      control namespace_eq expression_eq kind_eq steps ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    have member_child : ∀ id ∈ statements.toList, id ∈ expressionChildren expression.kind := by
      intro id member
      rw [kind_eq]
      cases result <;> simp_all [expressionChildren]
    obtain ⟨loans', extends', running', state', abrupt, id, member, typed⟩ :=
      ih loans inert context fuel' placed (fun id member => child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (member_child id member)) running stateTyped
    exact ⟨loans', extends', running', state', typed.parent expression_eq
      (by simp [kind_eq, runsPlainly]) (member_child id member) abrupt⟩
  case blockUnit =>
    intro namespaceId frame state exprId ns expression statements finalState finalFrame
      namespace_eq expression_eq kind_eq steps ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans', extends', running', state', statementsTyped⟩ :=
      ih loans inert context fuel' placed (fun id member => child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simpa [kind_eq, expressionChildren] using member))
        running stateTyped
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp only [type_eq, kind_eq, Bool.or_eq_true, Array.any_eq_true'] at nodeChecked
    refine ⟨loans', extends', running', state', fun fuel => ?_, type,
      exprType_of expression_eq type_eq, ?_⟩
    · cases fuel with
      | zero => rfl
      | succ fuel =>
          unfold diverts
          rw [expression_eq]
          simp only [kind_eq, Option.any_none, Bool.or_false, Array.any_eq_false']
          intro statement member
          obtain ⟨_, typed⟩ := statementsTyped statement (by simpa using member)
          simp [typed.1 fuel]
    · rcases nodeChecked with packed | ⟨statement, member, stopped⟩
      · exact packResults_typed (values := #[]) .nil packed
      · obtain ⟨_, typed⟩ := statementsTyped statement (by simpa using member)
        rw [typed.not_stops] at stopped
        cases stopped
  case blockResult =>
    intro namespaceId frame state exprId ns expression statements result statementState
      statementFrame finalState finalFrame control namespace_eq expression_eq kind_eq steps
      result_step ih_steps ih_result
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans₁, extends₁, running₁, state₁, statementsTyped⟩ :=
      ih_steps loans inert context fuel' placed (fun id member => child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children
        (by simp only [kind_eq, expressionChildren, Array.mem_push]; exact .inl (by simpa using member)))
        running stateTyped
    obtain ⟨loans₂, extends₂, running₂, state₂, resultTyped⟩ :=
      ih_result loans₁ inert context fuel' placed (child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simp [kind_eq, expressionChildren]))
        running₁ state₁
    refine ⟨loans₂, extends₂.trans extends₁, running₂, state₂, ?_⟩
    rcases Control.value_or_abrupt control with ⟨value, rfl⟩ | abrupt
    · unfold Context.checkNode at nodeChecked
      rw [expression_eq] at nodeChecked
      simp only [type_eq, kind_eq] at nodeChecked
      refine ⟨fun fuel => ?_, type, exprType_of expression_eq type_eq,
        resultTyped.flows nodeChecked⟩
      cases fuel with
      | zero => rfl
      | succ fuel =>
          unfold diverts
          rw [expression_eq]
          simp only [kind_eq, Option.any_some, Bool.or_eq_false_iff, Array.any_eq_false']
          refine ⟨fun statement member => ?_, resultTyped.1 fuel⟩
          obtain ⟨_, typed⟩ := statementsTyped statement (by simpa using member)
          simp [typed.1 fuel]
    · exact resultTyped.parent expression_eq (by simp [kind_eq, runsPlainly])
        (by simp [kind_eq, expressionChildren]) abrupt
  case letNoValue =>
    intro namespaceId frame state exprId ns expression pattern body finalFrame finalState control
      namespace_eq expression_eq kind_eq body_step ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans', extends', running', state', bodyTyped⟩ :=
      ih loans inert context fuel' placed (child_checkTree expression_eq (by simp [kind_eq, runsPlainly])
        children (by simp [kind_eq, expressionChildren])) running stateTyped
    refine ⟨loans', extends', running', state', ?_⟩
    rcases Control.value_or_abrupt control with ⟨value, rfl⟩ | abrupt
    · unfold Context.checkNode at nodeChecked
      rw [expression_eq] at nodeChecked
      simp only [type_eq, kind_eq, Bool.true_and] at nodeChecked
      refine ⟨fun fuel => ?_, type, exprType_of expression_eq type_eq,
        bodyTyped.flows nodeChecked⟩
      cases fuel with
      | zero => rfl
      | succ fuel =>
          unfold diverts
          rw [expression_eq]
          simp [kind_eq, bodyTyped.1 fuel]
    · exact bodyTyped.parent expression_eq (by simp [kind_eq, runsPlainly])
        (by simp [kind_eq, expressionChildren]) abrupt
  case letValueControl =>
    intro namespaceId frame state exprId ns expression pattern initializer body finalFrame
      finalState control namespace_eq expression_eq kind_eq initializer_step abrupt ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨loans', extends', running', state', typed⟩ :=
      ih loans inert context fuel' placed (child_checkTree expression_eq (by simp [kind_eq, runsPlainly])
        children (by simp [kind_eq, expressionChildren])) running stateTyped
    exact ⟨loans', extends', running', state', typed.parent expression_eq
      (by simp [kind_eq, runsPlainly]) (by simp [kind_eq, expressionChildren]) abrupt⟩
  case letMismatch =>
    intro namespaceId frame state exprId ns expression pattern initializer body initializedFrame
      initializedState runtimeValue kind arguments namespace_eq expression_eq kind_eq
      initializer_step bind_eq mismatch_eq ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨loans', extends', running', state', -⟩ :=
      ih loans inert context fuel' placed (child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simp [kind_eq, expressionChildren]))
        running stateTyped
    exact ⟨loans', extends', running', state', trivial⟩
  case letValue =>
    intro namespaceId frame state exprId ns expression pattern initializer body initializedFrame
      initializedState runtimeValue boundFrame finalFrame finalState control namespace_eq
      expression_eq kind_eq initializer_step bind_eq body_step ih_initializer ih_body
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans₁, extends₁, running₁, state₁, initializerTyped⟩ :=
      ih_initializer loans inert context fuel' placed (child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simp [kind_eq, expressionChildren]))
        running stateTyped
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    obtain ⟨valueType, valueType_eq, value_typed⟩ := initializerTyped.2
    simp only [type_eq, kind_eq, Bool.and_eq_true, initializerTyped.not_stops, Bool.false_or,
      valueType_eq] at nodeChecked
    obtain ⟨patternChecked, bodyFlows⟩ := nodeChecked
    have bound := running₁.bind placed patternChecked value_typed bind_eq
    obtain ⟨loans₂, extends₂, running₂, state₂, bodyTyped⟩ :=
      ih_body loans₁ inert context fuel' placed (child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simp [kind_eq, expressionChildren]))
        bound state₁
    refine ⟨loans₂, extends₂.trans extends₁, running₂, state₂, ?_⟩
    rcases Control.value_or_abrupt control with ⟨value, rfl⟩ | abrupt
    · refine ⟨fun fuel => ?_, type, exprType_of expression_eq type_eq, bodyTyped.flows bodyFlows⟩
      cases fuel with
      | zero => rfl
      | succ fuel =>
          unfold diverts
          rw [expression_eq]
          simp [kind_eq, bodyTyped.1 fuel, initializerTyped.1 fuel]
    · exact bodyTyped.parent expression_eq (by simp [kind_eq, runsPlainly])
        (by simp [kind_eq, expressionChildren]) abrupt
  case ifControl =>
    intro namespaceId frame state exprId ns expression condition thenBranch elseBranch finalFrame
      finalState control namespace_eq expression_eq kind_eq condition_step abrupt ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    have member : condition ∈ expressionChildren expression.kind := by
      rw [kind_eq]; cases elseBranch <;> simp [expressionChildren]
    obtain ⟨loans', extends', running', state', typed⟩ :=
      ih loans inert context fuel' placed (child_checkTree expression_eq (by simp [kind_eq, runsPlainly])
        children member) running stateTyped
    exact ⟨loans', extends', running', state', typed.parent expression_eq
      (by simp [kind_eq, runsPlainly]) member abrupt⟩
  case ifTrue =>
    intro namespaceId frame state exprId ns expression condition thenBranch elseBranch
      conditionFrame conditionState finalFrame finalState control namespace_eq expression_eq
      kind_eq condition_step branch_step ih_condition ih_branch
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    have conditionMember : condition ∈ expressionChildren expression.kind := by
      rw [kind_eq]; cases elseBranch <;> simp [expressionChildren]
    have branchMember : thenBranch ∈ expressionChildren expression.kind := by
      rw [kind_eq]; cases elseBranch <;> simp [expressionChildren]
    obtain ⟨loans₁, extends₁, running₁, state₁, conditionTyped⟩ :=
      ih_condition loans inert context fuel' placed (child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children conditionMember) running stateTyped
    obtain ⟨loans₂, extends₂, running₂, state₂, branchTyped⟩ :=
      ih_branch loans₁ inert context fuel' placed (child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children branchMember) running₁ state₁
    refine ⟨loans₂, extends₂.trans extends₁, running₂, state₂, ?_⟩
    rcases Control.value_or_abrupt control with ⟨value, rfl⟩ | abrupt
    · unfold Context.checkNode at nodeChecked
      rw [expression_eq] at nodeChecked
      simp only [type_eq, kind_eq, Bool.and_eq_true] at nodeChecked
      refine ⟨fun fuel => ?_, type, exprType_of expression_eq type_eq,
        branchTyped.flows nodeChecked.1.2⟩
      cases fuel with
      | zero => rfl
      | succ fuel =>
          unfold diverts
          rw [expression_eq]
          simp [kind_eq, branchTyped.1 fuel, conditionTyped.1 fuel]
    · exact branchTyped.parent expression_eq (by simp [kind_eq, runsPlainly]) branchMember abrupt
  case ifFalseUnit =>
    intro namespaceId frame state exprId ns expression condition thenBranch conditionFrame
      conditionState namespace_eq expression_eq kind_eq condition_step ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans', extends', running', state', conditionTyped⟩ :=
      ih loans inert context fuel' placed (child_checkTree expression_eq (by simp [kind_eq, runsPlainly])
        children (by simp [kind_eq, expressionChildren])) running stateTyped
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp only [type_eq, kind_eq, Bool.and_eq_true] at nodeChecked
    refine ⟨loans', extends', running', state', fun fuel => ?_, type,
      exprType_of expression_eq type_eq, packResults_typed (values := #[]) .nil nodeChecked.2⟩
    cases fuel with
    | zero => rfl
    | succ fuel =>
        unfold diverts
        rw [expression_eq]
        simp [kind_eq, conditionTyped.1 fuel]
  case ifFalse =>
    intro namespaceId frame state exprId ns expression condition thenBranch elseBranch
      conditionFrame conditionState finalFrame finalState control namespace_eq expression_eq
      kind_eq condition_step branch_step ih_condition ih_branch
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans₁, extends₁, running₁, state₁, conditionTyped⟩ :=
      ih_condition loans inert context fuel' placed (child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simp [kind_eq, expressionChildren]))
        running stateTyped
    obtain ⟨loans₂, extends₂, running₂, state₂, branchTyped⟩ :=
      ih_branch loans₁ inert context fuel' placed (child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simp [kind_eq, expressionChildren]))
        running₁ state₁
    refine ⟨loans₂, extends₂.trans extends₁, running₂, state₂, ?_⟩
    rcases Control.value_or_abrupt control with ⟨value, rfl⟩ | abrupt
    · unfold Context.checkNode at nodeChecked
      rw [expression_eq] at nodeChecked
      simp only [type_eq, kind_eq, Bool.and_eq_true] at nodeChecked
      refine ⟨fun fuel => ?_, type, exprType_of expression_eq type_eq,
        branchTyped.flows nodeChecked.2⟩
      cases fuel with
      | zero => rfl
      | succ fuel =>
          unfold diverts
          rw [expression_eq]
          simp [kind_eq, branchTyped.1 fuel, conditionTyped.1 fuel]
    · exact branchTyped.parent expression_eq (by simp [kind_eq, runsPlainly])
        (by simp [kind_eq, expressionChildren]) abrupt
  case assignControl =>
    intro namespaceId frame state exprId ns expression place child finalFrame finalState control
      namespace_eq expression_eq kind_eq child_step abrupt ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨loans', extends', running', state', typed⟩ :=
      ih loans inert context fuel' placed (child_checkTree expression_eq (by simp [kind_eq, runsPlainly])
        children (by simp [kind_eq, expressionChildren])) running stateTyped
    exact ⟨loans', extends', running', state', typed.parent expression_eq
      (by simp [kind_eq, runsPlainly]) (by simp [kind_eq, expressionChildren]) abrupt⟩
  case assignMismatch =>
    intro namespaceId frame state exprId ns expression place child childFrame childState
      runtimeValue kind thrown namespace_eq expression_eq kind_eq child_step resolve_eq mismatch
      mismatch_eq ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨loans', extends', running', state', -⟩ :=
      ih loans inert context fuel' placed (child_checkTree expression_eq (by simp [kind_eq, runsPlainly])
        children (by simp [kind_eq, expressionChildren])) running stateTyped
    exact ⟨loans', extends', running', state', trivial⟩
  case assignValue =>
    intro namespaceId frame state exprId ns expression place child childFrame childState resolved
      finalFrame finalState runtimeValue namespace_eq expression_eq kind_eq child_step resolve_eq
      write_eq ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans', extends', running', state', childTyped⟩ :=
      ih loans inert context fuel' placed (child_checkTree expression_eq (by simp [kind_eq, runsPlainly])
        children (by simp [kind_eq, expressionChildren])) running stateTyped
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp only [type_eq, kind_eq, Bool.and_eq_true] at nodeChecked
    obtain ⟨placeChecked, packed⟩ := nodeChecked
    split at placeChecked
    · rename_i placeType placeType_eq
      obtain ⟨variant, placeTyped⟩ := running'.place placed placeType_eq resolve_eq
      obtain ⟨written, rfl⟩ := placeTyped.write running'.typed (childTyped.flows placeChecked)
        write_eq
      refine ⟨loans', extends', running'.of_typed written (writeRuntimePlace?_instantiation write_eq),
        state', fun fuel => ?_, type, exprType_of expression_eq type_eq,
        packResults_typed (values := #[]) .nil packed⟩
      cases fuel with
      | zero => rfl
      | succ fuel =>
          unfold diverts
          rw [expression_eq]
          simp [kind_eq, childTyped.1 fuel]
    · cases placeChecked
  case assignPatternControl =>
    intro namespaceId frame state exprId ns expression pattern child finalFrame finalState control
      namespace_eq expression_eq kind_eq child_step abrupt ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨loans', extends', running', state', typed⟩ :=
      ih loans inert context fuel' placed (child_checkTree expression_eq (by simp [kind_eq, runsPlainly])
        children (by simp [kind_eq, expressionChildren])) running stateTyped
    exact ⟨loans', extends', running', state', typed.parent expression_eq
      (by simp [kind_eq, runsPlainly]) (by simp [kind_eq, expressionChildren]) abrupt⟩
  case assignPatternMismatch =>
    intro namespaceId frame state exprId ns expression pattern child childFrame finalState
      runtimeValue kind arguments namespace_eq expression_eq kind_eq child_step bind_eq
      mismatch_eq ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨loans', extends', running', state', -⟩ :=
      ih loans inert context fuel' placed (child_checkTree expression_eq (by simp [kind_eq, runsPlainly])
        children (by simp [kind_eq, expressionChildren])) running stateTyped
    exact ⟨loans', extends', running', state', trivial⟩
  case assignPatternValue =>
    intro namespaceId frame state exprId ns expression pattern child childFrame finalFrame
      finalState runtimeValue namespace_eq expression_eq kind_eq child_step bind_eq ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans', extends', running', state', childTyped⟩ :=
      ih loans inert context fuel' placed (child_checkTree expression_eq (by simp [kind_eq, runsPlainly])
        children (by simp [kind_eq, expressionChildren])) running stateTyped
    obtain ⟨valueType, valueType_eq, value_typed⟩ := childTyped.2
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp only [type_eq, kind_eq, Bool.and_eq_true, childTyped.not_stops, Bool.false_or,
      valueType_eq] at nodeChecked
    refine ⟨loans', extends', running'.bind placed nodeChecked.1 value_typed bind_eq, state',
      fun fuel => ?_, type, exprType_of expression_eq type_eq,
      packResults_typed (values := #[]) .nil nodeChecked.2⟩
    cases fuel with
    | zero => rfl
    | succ fuel =>
        unfold diverts
        rw [expression_eq]
        simp [kind_eq, childTyped.1 fuel]
  case matchControl =>
    intro namespaceId frame state exprId ns expression scrutinee arms finalFrame finalState control
      namespace_eq expression_eq kind_eq scrutinee_step abrupt ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    have member : scrutinee ∈ expressionChildren expression.kind := by
      rw [kind_eq]; exact match_children.1
    obtain ⟨loans', extends', running', state', typed⟩ :=
      ih loans inert context fuel' placed (child_checkTree expression_eq (by simp [kind_eq, runsPlainly])
        children member) running stateTyped
    exact ⟨loans', extends', running', state', typed.parent expression_eq
      (by simp [kind_eq, runsPlainly]) member abrupt⟩
  case matchValue =>
    intro namespaceId frame state exprId ns expression scrutinee arms scrutineeFrame
      scrutineeState runtimeValue finalFrame finalState control namespace_eq expression_eq kind_eq
      scrutinee_step arm_step ih_scrutinee ih_arms
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    have members := match_children (scrutinee := scrutinee) (arms := arms)
    rw [← kind_eq] at members
    obtain ⟨loans₁, extends₁, running₁, state₁, scrutineeTyped⟩ :=
      ih_scrutinee loans inert context fuel' placed (child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children members.1) running stateTyped
    obtain ⟨scrutineeType, scrutineeType_eq, value_typed⟩ := scrutineeTyped.2
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp only [type_eq, kind_eq, scrutineeTyped.not_stops, Bool.false_or, scrutineeType_eq,
      Array.all_eq_true', Bool.and_eq_true] at nodeChecked
    obtain ⟨loans₂, extends₂, running₂, state₂, armsTyped⟩ :=
      ih_arms loans₁ inert context fuel' scrutineeType type placed rfl (fun arm member => by
        have member := Array.mem_toList_iff.mp member
        obtain ⟨⟨pattern_, guard_⟩, body_⟩ := nodeChecked arm member
        refine ⟨pattern_, ?_, body_, child_checkTree expression_eq (by simp [kind_eq, runsPlainly])
          children (members.2 arm member).1⟩
        cases guard_eq : arm.guard with
        | none => rfl
        | some guard =>
            rw [guard_eq] at guard_
            simp only [Option.all_some, Bool.and_eq_true] at guard_ ⊢
            exact ⟨guard_, child_checkTree expression_eq (by simp [kind_eq, runsPlainly]) children
              ((members.2 arm member).2 guard guard_eq)⟩) value_typed running₁ state₁
    refine ⟨loans₂, extends₂.trans extends₁, running₂, state₂, ?_⟩
    rcases armsTyped with ⟨arm, armMember, armTyped⟩ | ⟨kind, arguments, rfl⟩
    · have armMember' := Array.mem_toList_iff.mp armMember
      rcases armTyped with bodyTyped | ⟨guard, guard_eq, guardTyped, abrupt⟩
      · rcases Control.value_or_abrupt control with ⟨value, rfl⟩ | abrupt
        · refine ⟨fun fuel => ?_, type, exprType_of expression_eq type_eq,
            bodyTyped.flows (nodeChecked arm armMember').2⟩
          cases fuel with
          | zero => rfl
          | succ fuel =>
              unfold diverts
              rw [expression_eq]
              simp only [kind_eq, scrutineeTyped.1 fuel, Bool.false_or, Bool.and_eq_false_iff,
                Bool.not_eq_false', Array.isEmpty_iff, Array.all_eq_false']
              right
              exact ⟨arm, armMember', by simp [bodyTyped.1 fuel]⟩
        · exact bodyTyped.parent expression_eq (by simp [kind_eq, runsPlainly])
            (members.2 arm armMember').1 abrupt
      · exact guardTyped.parent expression_eq (by simp [kind_eq, runsPlainly])
          ((members.2 arm armMember').2 guard guard_eq) abrupt
    · trivial
  case exhausted =>
    intro namespaceId ns frame state value kind arguments mismatch_eq
    intro loans inert context fuel scrutineeType resultType placed ns_eq armsChecked value_typed running
      stateTyped
    exact ⟨loans, .refl _, running, stateTyped, .inr ⟨kind, arguments, rfl⟩⟩
  case reject =>
    intro namespaceId ns frame state value arm arms finalFrame finalState control bind_eq
      tail_step ih
    intro loans inert context fuel scrutineeType resultType placed ns_eq armsChecked value_typed running
      stateTyped
    obtain ⟨loans', extends', running', state', typed⟩ :=
      ih loans inert context fuel scrutineeType resultType placed ns_eq
        (fun arm member => armsChecked arm (by simp [member])) value_typed running stateTyped
    refine ⟨loans', extends', running', state', ?_⟩
    rcases typed with ⟨arm', member, typed⟩ | thrown
    · exact .inl ⟨arm', by simp [member], typed⟩
    · exact .inr thrown
  case noGuard =>
    intro namespaceId ns frame state value arm arms armFrame finalFrame finalState control
      guard_eq bind_eq body_step ih
    intro loans inert context fuel scrutineeType resultType placed ns_eq armsChecked value_typed running
      stateTyped
    subst ns_eq
    obtain ⟨pattern_, -, -, body_⟩ := armsChecked arm (by simp)
    obtain ⟨loans', extends', running', state', typed⟩ :=
      ih loans inert context fuel placed body_ (running.bind placed pattern_ value_typed bind_eq)
        stateTyped
    exact ⟨loans', extends', running', state', .inl ⟨arm, by simp, .inl typed⟩⟩
  case guardControl =>
    intro namespaceId ns frame state value arm arms guard armFrame finalFrame finalState control
      guard_eq bind_eq guard_step abrupt ih
    intro loans inert context fuel scrutineeType resultType placed ns_eq armsChecked value_typed running
      stateTyped
    subst ns_eq
    obtain ⟨pattern_, guard_, -, -⟩ := armsChecked arm (by simp)
    rw [guard_eq] at guard_
    simp only [Option.all_some, Bool.and_eq_true] at guard_
    obtain ⟨loans', extends', running', state', typed⟩ :=
      ih loans inert context fuel placed guard_.2 (running.bind placed pattern_ value_typed bind_eq)
        stateTyped
    exact ⟨loans', extends', running', state', .inl ⟨arm, by simp, .inr ⟨guard, guard_eq, typed, abrupt⟩⟩⟩
  case guardTrue =>
    intro namespaceId ns frame state value arm arms guard armFrame guardFrame guardState
      finalFrame finalState control guard_eq bind_eq guard_step body_step ih_guard ih_body
    intro loans inert context fuel scrutineeType resultType placed ns_eq armsChecked value_typed running
      stateTyped
    subst ns_eq
    obtain ⟨pattern_, guard_, -, body_⟩ := armsChecked arm (by simp)
    rw [guard_eq] at guard_
    simp only [Option.all_some, Bool.and_eq_true] at guard_
    obtain ⟨loans₁, extends₁, running₁, state₁, -⟩ :=
      ih_guard loans inert context fuel placed guard_.2 (running.bind placed pattern_ value_typed bind_eq)
        stateTyped
    obtain ⟨loans₂, extends₂, running₂, state₂, typed⟩ :=
      ih_body loans₁ inert context fuel placed body_ running₁ state₁
    exact ⟨loans₂, extends₂.trans extends₁, running₂, state₂, .inl ⟨arm, by simp, .inl typed⟩⟩
  case guardFalse =>
    intro namespaceId ns frame state value arm arms guard armFrame guardFrame guardState
      finalFrame finalState control guard_eq bind_eq guard_step tail_step ih_guard ih_tail
    intro loans inert context fuel scrutineeType resultType placed ns_eq armsChecked value_typed running
      stateTyped
    obtain ⟨loans', extends', running', state', typed⟩ :=
      ih_tail loans inert context fuel scrutineeType resultType placed ns_eq
        (fun arm member => armsChecked arm (by simp [member])) value_typed running stateTyped
    refine ⟨loans', extends', running', state', ?_⟩
    rcases typed with ⟨arm', member, typed⟩ | thrown
    · exact .inl ⟨arm', by simp [member], typed⟩
    · exact .inr thrown
  case loopRepeatValue =>
    intro namespaceId frame state exprId ns expression label body bodyFrame bodyState
      runtimeValue finalFrame finalState control namespace_eq expression_eq kind_eq body_step
      repeat_step ih_body ih_repeat
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', fuel_eq, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans₁, extends₁, running₁, state₁, -⟩ :=
      ih_body loans inert { context with loops := type :: context.loops } fuel' (placed.loops _)
        (loop_child_checkTree expression_eq kind_eq type_eq children) (Running.loops.mpr running)
        stateTyped
    obtain ⟨loans₂, extends₂, running₂, state₂, typed⟩ :=
      ih_repeat loans₁ inert context fuel placed checked (Running.loops.mp running₁) state₁
    exact ⟨loans₂, extends₂.trans extends₁, running₂, state₂, typed⟩
  case loopRepeatContinue =>
    intro namespaceId frame state exprId ns expression label body bodyFrame bodyState finalFrame
      finalState control namespace_eq expression_eq kind_eq body_step repeat_step ih_body
      ih_repeat
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', fuel_eq, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans₁, extends₁, running₁, state₁, -⟩ :=
      ih_body loans inert { context with loops := type :: context.loops } fuel' (placed.loops _)
        (loop_child_checkTree expression_eq kind_eq type_eq children) (Running.loops.mpr running)
        stateTyped
    obtain ⟨loans₂, extends₂, running₂, state₂, typed⟩ :=
      ih_repeat loans₁ inert context fuel placed checked (Running.loops.mp running₁) state₁
    exact ⟨loans₂, extends₂.trans extends₁, running₂, state₂, typed⟩
  case loopBreak =>
    intro namespaceId frame state exprId ns expression label body finalFrame finalState value
      namespace_eq expression_eq kind_eq body_step ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans', extends', running', state', breaks, loopType, loopType_eq, value_typed⟩ :=
      ih loans inert { context with loops := type :: context.loops } fuel' (placed.loops _)
        (loop_child_checkTree expression_eq kind_eq type_eq children) (Running.loops.mpr running)
        stateTyped
    simp only [List.getElem?_cons_zero, Option.some.injEq] at loopType_eq
    subst loopType_eq
    refine ⟨loans', extends', Running.loops.mp running', state', fun fuel => ?_, type,
      exprType_of expression_eq type_eq, value_typed⟩
    cases fuel with
    | zero => rfl
    | succ fuel =>
        unfold diverts
        rw [expression_eq]
        simp [kind_eq, breaks fuel]
  case loopOuterBreak =>
    intro namespaceId frame state exprId ns expression label body finalFrame finalState nest value
      namespace_eq expression_eq kind_eq body_step ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans', extends', running', state', breaks, loopType, loopType_eq, value_typed⟩ :=
      ih loans inert { context with loops := type :: context.loops } fuel' (placed.loops _)
        (loop_child_checkTree expression_eq kind_eq type_eq children) (Running.loops.mpr running)
        stateTyped
    refine ⟨loans', extends', Running.loops.mp running', state', fun fuel => ?_, loopType,
      by simpa using loopType_eq, value_typed⟩
    cases fuel with
    | zero => rfl
    | succ fuel =>
        unfold breaksTo
        rw [expression_eq]
        simp [kind_eq, breaks fuel]
  case loopOuterContinue =>
    intro namespaceId frame state exprId ns expression label body finalFrame finalState nest
      namespace_eq expression_eq kind_eq body_step ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans', extends', running', state', -⟩ :=
      ih loans inert { context with loops := type :: context.loops } fuel' (placed.loops _)
        (loop_child_checkTree expression_eq kind_eq type_eq children) (Running.loops.mpr running)
        stateTyped
    exact ⟨loans', extends', Running.loops.mp running', state', trivial⟩
  case loopReturn =>
    intro namespaceId frame state exprId ns expression label body finalFrame finalState values
      namespace_eq expression_eq kind_eq body_step ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans', extends', running', state', typed⟩ :=
      ih loans inert { context with loops := type :: context.loops } fuel' (placed.loops _)
        (loop_child_checkTree expression_eq kind_eq type_eq children) (Running.loops.mpr running)
        stateTyped
    exact ⟨loans', extends', Running.loops.mp running', state', typed⟩
  case loopThrow =>
    intro namespaceId frame state exprId ns expression label body finalFrame finalState kind
      arguments namespace_eq expression_eq kind_eq body_step ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans', extends', running', state', -⟩ :=
      ih loans inert { context with loops := type :: context.loops } fuel' (placed.loops _)
        (loop_child_checkTree expression_eq kind_eq type_eq children) (Running.loops.mpr running)
        stateTyped
    exact ⟨loans', extends', Running.loops.mp running', state', trivial⟩
  case callReturned =>
    intro namespaceId frame state exprId ns expression reference instantiations
      arguments surface argumentState argumentFrame values handle finalState results
      namespace_eq expression_eq kind_eq operands resolve_eq calleeStep ih_operands ih_callee
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans', extends', running', state', valuesTyped⟩ :=
      ih_operands loans inert context fuel' placed (fun id member => child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simpa [kind_eq, expressionChildren] using member))
        running stateTyped
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp only [type_eq, kind_eq, Context.checkOperation] at nodeChecked
    obtain ⟨loans'', extends'', running'', state'', typed⟩ :=
      running'.call placed state' valuesTyped nodeChecked resolve_eq ih_callee
    exact ⟨loans'', extends''.trans extends', running''.of_typed
      (running''.typed.of_locals (registerReturnedLoan_locals _ _ _))
      (registerReturnedLoan_instantiation _ _ _), state'',
      valuesTyped.operation_quiet expression_eq kind_eq, type, exprType_of expression_eq type_eq,
      typed results rfl⟩
  case callThrew =>
    intro namespaceId frame state exprId ns expression reference instantiations
      arguments surface argumentState argumentFrame values handle finalState kind thrown
      namespace_eq expression_eq kind_eq operands resolve_eq calleeStep ih_operands ih_callee
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans', extends', running', state', valuesTyped⟩ :=
      ih_operands loans inert context fuel' placed (fun id member => child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simpa [kind_eq, expressionChildren] using member))
        running stateTyped
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp only [type_eq, kind_eq, Context.checkOperation] at nodeChecked
    obtain ⟨loans'', extends'', running'', state'', -⟩ :=
      running'.call placed state' valuesTyped nodeChecked resolve_eq ih_callee
    exact ⟨loans'', extends''.trans extends', running'', state'', trivial⟩
  case callArgumentsControl | constructorArgumentsControl | destructorArgumentsControl
      | closureArgumentsControl | invokeArgumentsControl | profileArgumentsControl
      | primitiveArgumentsControl | globalArgumentsControl | assertArgumentsControl
      | operationArgumentsControl =>
    intros
    apply operands_control <;> assumption
  case constructorValue =>
    intro namespaceId frame state exprId ns expression reference variant instantiations arguments
      surface finalState finalFrame values runtimeValue namespace_eq expression_eq kind_eq operands
      construct_eq ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans', extends', running', state', valuesTyped⟩ :=
      ih loans inert context fuel' placed (fun id member => child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simpa [kind_eq, expressionChildren] using member))
        running stateTyped
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp only [type_eq, kind_eq, Context.checkOperation, Context.checkCall, placed.unit_eq]
      at nodeChecked
    refine ⟨loans', extends', running', state', valuesTyped.operation_quiet expression_eq kind_eq,
      type, exprType_of expression_eq type_eq, ?_⟩
    cases target : structTarget? unit context.ns reference with
    | none =>
        simp only [target, Option.isNone_iff_eq_none] at nodeChecked
        simp [constructNominal?, resolveStruct_none placed.namespace_eq nodeChecked] at construct_eq
    | some found =>
        obtain ⟨targetNs, declaration, spelled⟩ := found
        cases resolved : resolveArguments context.ns context.env instantiations with
        | none => simp [target, resolved] at nodeChecked
        | some semantic =>
          simp only [target, resolved] at nodeChecked
          split at nodeChecked
          · rename_i fieldTypes fieldTypes_eq
            simp only [Bool.and_eq_true, beq_iff_eq] at nodeChecked
            obtain ⟨⟨size_eq, flows⟩, rfl⟩ := nodeChecked
            exact constructNominal_typed placed.namespace_eq target fieldTypes_eq
              (by simpa using valuesTyped.flows (by simpa using size_eq) flows) construct_eq
          · cases nodeChecked
  case destructorValue =>
    intro namespaceId frame state exprId ns expression reference variant instantiations arguments
      surface finalState finalFrame value fields namespace_eq expression_eq kind_eq operands
      destruct_eq ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans', extends', running', state', valuesTyped⟩ :=
      ih loans inert context fuel' placed (fun id member => child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simpa [kind_eq, expressionChildren] using member))
        running stateTyped
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp only [type_eq, kind_eq, Context.checkOperation, Context.checkCall, placed.unit_eq]
      at nodeChecked
    refine ⟨loans', extends', running', state', valuesTyped.operation_quiet expression_eq kind_eq,
      type, exprType_of expression_eq type_eq, ?_⟩
    cases target : structTarget? unit context.ns reference with
    | none =>
        simp only [target, Option.isNone_iff_eq_none] at nodeChecked
        unfold destructNominal? at destruct_eq
        split at destruct_eq
        · simp [resolveStruct_none placed.namespace_eq nodeChecked] at destruct_eq
        · cases destruct_eq
    | some found =>
        obtain ⟨targetNs, declaration, spelled⟩ := found
        obtain ⟨operand, operands_eq, operandTyped⟩ : ∃ operand, arguments.toList = [operand] ∧
            ValueTyped unit loans' context operand value := by
          cases list_eq : arguments.toList with
          | nil => rw [list_eq] at valuesTyped; simp [ValuesTyped] at valuesTyped
          | cons operand rest =>
              rw [list_eq] at valuesTyped
              cases rest with
              | nil => exact ⟨operand, rfl, valuesTyped.1⟩
              | cons _ _ => simp [ValuesTyped] at valuesTyped
        cases resolved : resolveArguments context.ns context.env instantiations with
        | none => simp [target, resolved] at nodeChecked
        | some semantic =>
          simp only [target, resolved, operands_eq, Bool.and_eq_true, beq_iff_eq] at nodeChecked
          obtain ⟨operandType, fieldsChecked⟩ := nodeChecked
          split at fieldsChecked
          · rename_i fieldTypes fieldTypes_eq
            obtain ⟨-, type', type'_eq, valueTyped⟩ := operandTyped
            rw [operandType, Option.some.injEq] at type'_eq
            subst type'_eq
            exact packResults_typed (destructNominal_typed placed.namespace_eq target fieldTypes_eq
              valueTyped destruct_eq) fieldsChecked
          · cases fieldsChecked
  case closureValue =>
    intro namespaceId frame state exprId ns expression reference mask instantiations captures
      surface finalState finalFrame values handle namespace_eq expression_eq kind_eq operands
      resolve_eq ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans', extends', running', state', valuesTyped⟩ :=
      ih loans inert context fuel' placed (fun id member => child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simpa [kind_eq, expressionChildren] using member))
        running stateTyped
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp only [type_eq, kind_eq, Context.checkOperation] at nodeChecked
    exact ⟨loans', extends', running', state', valuesTyped.operation_quiet expression_eq kind_eq,
      type, exprType_of expression_eq type_eq,
      running'.closure placed valuesTyped nodeChecked resolve_eq⟩
  case invokeReturned =>
    intro namespaceId frame state exprId ns expression instantiations arguments surface
      argumentState argumentFrame handle mask typeInstantiation captures values composed
      finalState results namespace_eq expression_eq kind_eq operands compose_eq calleeStep
      ih_operands ih_callee
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans', extends', running', state', valuesTyped⟩ :=
      ih_operands loans inert context fuel' placed (fun id member => child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simpa [kind_eq, expressionChildren] using member))
        running stateTyped
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp only [type_eq, kind_eq, Context.checkOperation] at nodeChecked
    obtain ⟨loans'', extends'', running'', state'', typed⟩ :=
      running'.invokeChecked state' valuesTyped nodeChecked compose_eq ih_callee
    exact ⟨loans'', extends''.trans extends', running''.of_typed
      (running''.typed.of_locals (registerReturnedLoan_locals _ _ _))
      (registerReturnedLoan_instantiation _ _ _), state'',
      valuesTyped.operation_quiet expression_eq kind_eq, type, exprType_of expression_eq type_eq,
      typed results rfl⟩
  case invokeThrew =>
    intro namespaceId frame state exprId ns expression instantiations arguments surface
      argumentState argumentFrame handle mask typeInstantiation captures values composed
      finalState kind thrown namespace_eq expression_eq kind_eq operands compose_eq calleeStep
      ih_operands ih_callee
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans', extends', running', state', valuesTyped⟩ :=
      ih_operands loans inert context fuel' placed (fun id member => child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simpa [kind_eq, expressionChildren] using member))
        running stateTyped
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp only [type_eq, kind_eq, Context.checkOperation] at nodeChecked
    obtain ⟨loans'', extends'', running'', state'', -⟩ :=
      running'.invokeChecked state' valuesTyped nodeChecked compose_eq ih_callee
    exact ⟨loans'', extends''.trans extends', running'', state'', trivial⟩
  case constantValue =>
    intro namespaceId frame state exprId ns expression reference handle targetNs declaration
      targetFrame finalState runtimeValue namespace_eq expression_eq kind_eq resolve_eq
      target_namespace_eq declaration_eq initializer ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, -⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨table, constantType, constantType_eq, flows, loans', extends', -, state', valueTyped⟩ :=
      constant_initializer target_namespace_eq declaration_eq ih stateTyped
    obtain ⟨targetNs', declaration', target_eq, targetNs'_eq, declaration'_eq⟩ :=
      constantTarget_of_resolve placed.namespace_eq resolve_eq
    rw [target_namespace_eq, Option.some.injEq] at targetNs'_eq
    subst targetNs'_eq
    rw [declaration_eq, Option.some.injEq] at declaration'_eq
    subst declaration'_eq
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp only [type_eq, kind_eq, placed.unit_eq, target_eq, constantType_eq, beq_iff_eq,
      Option.some.injEq] at nodeChecked
    subst nodeChecked
    exact ⟨loans', extends', running.weaken extends', state',
      diverts_false_of expression_eq (by simp [kind_eq]), constantType,
      exprType_of expression_eq type_eq, valueTyped.flows flows⟩
  case constantControl =>
    intro namespaceId frame state exprId ns expression reference handle targetNs declaration
      targetFrame finalState control namespace_eq expression_eq kind_eq resolve_eq
      target_namespace_eq declaration_eq initializer abrupt ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨table, -, -, -, loans', extends', -, state', typed⟩ :=
      constant_initializer target_namespace_eq declaration_eq ih stateTyped
    refine ⟨loans', extends', running.weaken extends', state', ?_⟩
    cases abrupt with
    | break_ nest value =>
        obtain ⟨-, loopType, loopType_eq, -⟩ := typed
        simp [constantContext] at loopType_eq
    | continue_ nest => trivial
    | return_ values =>
        obtain ⟨results, results_eq, -⟩ := typed
        simp [constantContext] at results_eq
    | throw_ kind arguments => trivial
  case native =>
    intro handle typeInstantiation initialState arguments ns declaration finalState outcome
      namespace_eq declaration_eq arity_eq body_eq native_eq hole_free
    exact natives handle typeInstantiation initialState arguments finalState outcome native_eq
  case body =>
    intro handle typeInstantiation initialState arguments ns declaration frame root finalFrame
      evaluatedState finalState control outcome namespace_eq declaration_eq frame_eq body_eq
      body_step outcome_eq hole_free finalize_eq ih
    intro loans inert ns' declaration' semantic parameters results namespace_eq' declaration_eq'
      faithful signature_eq argumentsTyped stateTyped
    rw [namespace_eq, Option.some.injEq] at namespace_eq'
    subst namespace_eq'
    rw [declaration_eq, Option.some.injEq] at declaration_eq'
    subst declaration_eq'
    obtain ⟨-, -, -, -, table, table_eq, functions, -⟩ :=
      checkUnit_namespace executable.typed namespace_eq
    have checked := functions _ _ declaration_eq
    unfold checkFunction at checked
    rw [body_eq] at checked
    dsimp only at checked
    split at checked
    · cases checked
    rename_i staticParameters staticResults static_eq
    simp only [Bool.and_eq_true] at checked
    obtain ⟨⟨parametersChecked, rootChecked⟩, treeChecked⟩ := checked
    have frameEnv_eq : frameEnv declaration.signature.generics semantic =
        (staticEnv declaration.signature.generics).map (·.subst semantic) := rfl
    rw [frameEnv_eq, signatureTypes?_subst (arguments := semantic) static_eq, Option.some.injEq,
      Prod.mk.injEq] at signature_eq
    obtain ⟨rfl, rfl⟩ := signature_eq
    have placed : Placed executable handle.namespaceId
        ((functionContext unit executable.targetPointerWidth ns table
          (handle.namespaceId.index, handle.functionId.index) declaration staticResults).subst
          semantic) := ⟨rfl, rfl, namespace_eq, table_eq⟩
    have running : Running unit loans
        ((functionContext unit executable.targetPointerWidth ns table
          (handle.namespaceId.index, handle.functionId.index) declaration staticResults).subst
          semantic) frame := by
      refine ⟨initialFrame?_typed rfl (fun index parameter parameter_eq => ?_) argumentsTyped
        frame_eq, ?_⟩
      · simp only [List.getElem?_map, Option.map_eq_some_iff] at parameter_eq
        obtain ⟨staticParameter, staticParameter_eq, rfl⟩ := parameter_eq
        have holds := List.all_eq_true.mp parametersChecked (staticParameter, index)
          (List.mem_zipIdx_iff_getElem?.mpr (by simpa using staticParameter_eq))
        simp only [beq_iff_eq] at holds
        exact Context.localType_subst (arguments := semantic) holds
      · show FrameInstantiation ns (table.at (handle.namespaceId.index, handle.functionId.index))
          frame.typeInstantiation ((staticEnv declaration.signature.generics).map (·.subst semantic))
        have required_eq : requiredAt unit handle =
            table.at (handle.namespaceId.index, handle.functionId.index) := by
          simp [requiredAt, table_eq]
        rw [initialFrame?_instantiation frame_eq, ← required_eq]
        exact faithful
    obtain ⟨loans', extends', running', state', controlTyped⟩ :=
      ih loans inert _ _ placed (Context.checkTree_subst (arguments := semantic) _ treeChecked) running stateTyped
    have resultsLength := signatureTypes?_results_length static_eq
    subst finalize_eq
    cases control with
    | value result =>
        simp only [finishControl?, Functor.map, Option.map_eq_some_iff] at outcome_eq
        obtain ⟨values, unpack_eq, rfl⟩ := outcome_eq
        refine ⟨loans', extends', exportReturnedFrameLoans_typed running'.typed state',
          fun values' returned_eq => ?_⟩
        cases returned_eq
        obtain ⟨quiet, type, type_eq, typed⟩ := controlTyped
        split at rootChecked
        · rename_i rootType rootType_eq
          rw [Context.exprType_subst (arguments := semantic) rootType_eq, Option.some.injEq] at type_eq
          subst type_eq
          simp only [Bool.or_eq_true, beq_iff_eq] at rootChecked
          rcases rootChecked with (packed | never) | diverted
          · exact unpackFallthrough_typed typed (packs_subst (arguments := semantic) packed)
              (by simpa [resultsLength] using unpack_eq)
          · subst never
            exact (typed.inhabited (by simp [SemTy.subst, SemTy.unshared])).elim
          · exact absurd ((quiet _).symm.trans diverted) Bool.false_ne_true
        · cases rootChecked
    | return_ values =>
        simp only [finishControl?] at outcome_eq
        split at outcome_eq
        · simp only [Option.some.injEq] at outcome_eq
          subst outcome_eq
          refine ⟨loans', extends', exportReturnedFrameLoans_typed running'.typed state',
            fun values' returned_eq => ?_⟩
          cases returned_eq
          obtain ⟨results', results_eq, typed⟩ := controlTyped
          simp only [Context.subst, functionContext, Option.map_some, Option.some.injEq]
            at results_eq
          subst results_eq
          exact typed
        · cases outcome_eq
    | throw_ kind thrown =>
        simp only [finishControl?, Option.some.injEq] at outcome_eq
        subst outcome_eq
        simp only [finalizeFunctionState]
        split
        · split
          · exact ⟨loans, .refl _, stateTyped, fun _ returned_eq => by cases returned_eq⟩
          · exact ⟨loans', extends', exportFrameLoans_typed running'.typed state',
              fun _ returned_eq => by cases returned_eq⟩
        · exact ⟨loans', extends', exportFrameLoans_typed running'.typed state',
            fun _ returned_eq => by cases returned_eq⟩
    | break_ nest value => simp [finishControl?] at outcome_eq
    | continue_ nest => simp [finishControl?] at outcome_eq
  case profileValue =>
    intro namespaceId frame state exprId ns expression operation targets instantiations
      arguments surface finalState finalFrame values runtimeValue namespace_eq expression_eq
      kind_eq operands evaluate_eq ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, -⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp [type_eq, kind_eq, Context.checkOperation] at nodeChecked
  case profileThrow =>
    intro namespaceId frame state exprId ns expression operation targets instantiations
      arguments surface finalState finalFrame values kind thrown namespace_eq expression_eq
      kind_eq operands evaluate_eq ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, -⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp [type_eq, kind_eq, Context.checkOperation] at nodeChecked
  case primitiveValue =>
    intro namespaceId frame state exprId ns expression operation instantiations arguments surface
      finalState finalFrame values runtimeValue namespace_eq expression_eq kind_eq operands
      evaluate_eq ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans', extends', running', state', valuesTyped⟩ :=
      ih loans inert context fuel' placed (fun id member => child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simpa [kind_eq, expressionChildren] using member))
        running stateTyped
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp only [type_eq, kind_eq, Context.checkOperation] at nodeChecked
    split at nodeChecked
    · rename_i types types_eq
      rw [placed.width] at nodeChecked
      rw [executable.width_eq] at evaluate_eq
      exact ⟨loans', extends', running', state', valuesTyped.operation_quiet expression_eq kind_eq,
        type, exprType_of expression_eq type_eq, evaluatePrimitive_typed ⟨_, type_eq⟩ nodeChecked
          (by simpa using valuesTyped.types types_eq) evaluate_eq⟩
    · cases nodeChecked
  case primitiveThrow =>
    intro namespaceId frame state exprId ns expression operation instantiations arguments surface
      finalState finalFrame values kind thrown namespace_eq expression_eq kind_eq operands
      evaluate_eq ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, -, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨loans', extends', running', state', -⟩ :=
      ih loans inert context fuel' placed (fun id member => child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simpa [kind_eq, expressionChildren] using member))
        running stateTyped
    exact ⟨loans', extends', running', state', trivial⟩
  case globalValue =>
    intro namespaceId frame state exprId ns expression kind instantiations arguments surface
      argumentState argumentFrame values finalFrame finalState runtimeValue namespace_eq
      expression_eq kind_eq operands evaluate_eq ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans', extends', running', state', valuesTyped⟩ :=
      ih loans inert context fuel' placed (fun id member => child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simpa [kind_eq, expressionChildren] using member))
        running stateTyped
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp only [type_eq, kind_eq] at nodeChecked
    obtain ⟨⟨loans'', extends'', frameTyped, stateTyped'', valueTyped⟩, resource, rfl⟩ :=
      running'.global placed state' valuesTyped nodeChecked evaluate_eq
    exact ⟨loans'', extends''.trans extends', running'.of_typed frameTyped
      (evaluateGlobalOperation?_instantiation evaluate_eq), stateTyped'',
      valuesTyped.operation_quiet expression_eq kind_eq, type, exprType_of expression_eq type_eq,
      valueTyped⟩
  case globalThrow =>
    intro namespaceId frame state exprId ns expression globalKind instantiations arguments surface
      argumentState argumentFrame values finalFrame finalState throwKind thrown namespace_eq
      expression_eq kind_eq operands evaluate_eq ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans', extends', running', state', valuesTyped⟩ :=
      ih loans inert context fuel' placed (fun id member => child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simpa [kind_eq, expressionChildren] using member))
        running stateTyped
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp only [type_eq, kind_eq] at nodeChecked
    obtain ⟨⟨rfl, rfl⟩, -⟩ := running'.global placed state' valuesTyped nodeChecked evaluate_eq
    exact ⟨loans', extends', running', state', trivial⟩
  case assertTrue =>
    intro namespaceId frame state exprId ns expression instantiations arguments surface
      finalState finalFrame namespace_eq expression_eq kind_eq operands ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans', extends', running', state', valuesTyped⟩ :=
      ih loans inert context fuel' placed (fun id member => child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simpa [kind_eq, expressionChildren] using member))
        running stateTyped
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp only [type_eq, kind_eq, Context.checkOperation] at nodeChecked
    exact ⟨loans', extends', running', state', valuesTyped.operation_quiet expression_eq kind_eq,
      type, exprType_of expression_eq type_eq, packResults_typed (values := #[]) .nil nodeChecked⟩
  case assertFalse =>
    intro namespaceId frame state exprId ns expression instantiations arguments surface
      finalState finalFrame namespace_eq expression_eq kind_eq operands ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, -, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨loans', extends', running', state', -⟩ :=
      ih loans inert context fuel' placed (fun id member => child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simpa [kind_eq, expressionChildren] using member))
        running stateTyped
    exact ⟨loans', extends', running', state', trivial⟩
  case operationValue =>
    intro namespaceId frame state exprId ns expression operation instantiations arguments surface
      argumentState argumentFrame values finalFrame finalState runtimeValue namespace_eq
      expression_eq kind_eq operands evaluate_eq ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨type, type_eq⟩ := checkNode_type nodeChecked expression_eq
    obtain ⟨loans', extends', running', state', valuesTyped⟩ :=
      ih loans inert context fuel' placed (fun id member => child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simpa [kind_eq, expressionChildren] using member))
        running stateTyped
    unfold Context.checkNode at nodeChecked
    rw [expression_eq] at nodeChecked
    simp only [type_eq, kind_eq] at nodeChecked
    obtain ⟨loans'', extends'', running'', state'', typed⟩ := running'.placeOperation placed state'
      expression_eq kind_eq type_eq valuesTyped nodeChecked evaluate_eq
    exact ⟨loans'', extends''.trans extends', running'', state'',
      valuesTyped.operation_quiet expression_eq kind_eq, type, exprType_of expression_eq type_eq,
      typed⟩
  case operationMismatch =>
    intro namespaceId frame state exprId ns expression operation instantiations arguments surface
      argumentState argumentFrame values kind thrown namespace_eq expression_eq kind_eq operands
      evaluate_eq mismatch mismatch_eq ih
    intro loans inert context fuel placed checked running stateTyped
    obtain ⟨fuel', rfl, nodeChecked, children⟩ := checkTree_step checked
    cases placed.ns_eq namespace_eq
    obtain ⟨loans', extends', running', state', -⟩ :=
      ih loans inert context fuel' placed (fun id member => child_checkTree expression_eq
        (by simp [kind_eq, runsPlainly]) children (by simpa [kind_eq, expressionChildren] using member))
        running stateTyped
    exact ⟨loans', extends', running', state', trivial⟩

end LeanerIR
