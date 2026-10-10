-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Validation.StaticTyping

/-!
# Checks under instantiation

A generic body is checked once, under its rigid parameters; a frame runs it
under semantic arguments (`Context.subst`). Every rule is a positive check
of equalities and shapes of types, which substitution keeps, so a body
checked under its parameters checks under any arguments (`checkTree_subst`).
-/

namespace LeanerIR.Validation.StaticTyping

/-- A context with its generic environment, results, and loop types under
semantic arguments: where a body checked under its rigid parameters runs. -/
def Context.subst (context : Context) (arguments : Array SemArg) : Context :=
  { context with
    env := context.env.map (·.subst arguments)
    results := context.results.map (·.map (·.subst arguments))
    loops := context.loops.map (·.subst arguments) }

section Projections

variable (context : Context) (arguments : Array SemArg)

@[simp] theorem Context.subst_unit : (context.subst arguments).unit = context.unit := rfl
@[simp] theorem Context.subst_pointerWidth :
    (context.subst arguments).pointerWidth = context.pointerWidth := rfl
@[simp] theorem Context.subst_ns : (context.subst arguments).ns = context.ns := rfl
@[simp] theorem Context.subst_locals : (context.subst arguments).locals = context.locals := rfl
@[simp] theorem Context.subst_required :
    (context.subst arguments).required = context.required := rfl
@[simp] theorem Context.subst_table : (context.subst arguments).table = context.table := rfl
@[simp] theorem Context.subst_env :
    (context.subst arguments).env = context.env.map (·.subst arguments) := rfl
@[simp] theorem Context.subst_results :
    (context.subst arguments).results = context.results.map (·.map (·.subst arguments)) := rfl
@[simp] theorem Context.subst_loops :
    (context.subst arguments).loops = context.loops.map (·.subst arguments) := rfl

end Projections

section Subst

variable (arguments : Array SemArg)

theorem resolveIn_subst {ns : ValidatedNamespace} {env : Array SemArg} {typeId : TypeId}
    {type : SemTy} (resolved : resolveIn ns env typeId = some type) :
    resolveIn ns (env.map (·.subst arguments)) typeId = some (type.subst arguments) :=
  SemTy.resolveFuel_subst _ _ _ _ _ _ resolved

variable {arguments}

/-- A type the checker reads as an integer of the target is no parameter. -/
theorem integral_subst {pointerWidth : Option Nat} {type : SemTy}
    (holds_ : integral pointerWidth type = true) : type.subst arguments = type := by
  cases type <;> simp_all [integral, SemTy.subst]

theorem fixedInteger_subst {pointerWidth : Option Nat} {type : SemTy}
    (holds_ : fixedInteger pointerWidth type = true) : type.subst arguments = type := by
  cases type <;> simp_all [fixedInteger, SemTy.subst]

theorem holds_subst {pointerWidth : Option Nat} {type : SemTy} {value : Int}
    (holds_ : holds pointerWidth type value = true) : type.subst arguments = type := by
  cases type <;> simp_all [holds, SemTy.subst]

theorem packs_subst {results : List SemTy} {type : SemTy}
    (packed : packs results type = true) :
    packs (results.map (·.subst arguments)) (type.subst arguments) = true := by
  match results, packed with
  | [], packed =>
      simp only [packs, Bool.or_eq_true, beq_iff_eq] at packed
      rcases packed with rfl | rfl
      · simp [packs, SemTy.subst]
      · simp [packs, SemTy.subst, SemTy.substList]
  | [result], packed =>
      simp only [packs, beq_iff_eq] at packed ⊢
      simp [packed]
  | first :: second :: rest, packed =>
      simp only [packs, beq_iff_eq] at packed ⊢
      subst packed
      simp [SemTy.subst, SemTy.substList_eq_map]

mutual
theorem constTyped_subst {pointerWidth : Option Nat} :
    ∀ (value : ConstValue) (type : SemTy), constTyped pointerWidth value type = true →
      constTyped pointerWidth value (type.subst arguments) = true
  | .unit, type, typed => by
      simp only [constTyped, Bool.or_eq_true, beq_iff_eq] at typed ⊢
      rcases typed with rfl | rfl
      · exact .inl (by simp [SemTy.subst])
      · exact .inr (by simp [SemTy.subst, SemTy.substList])
  | .bool _, type, typed => by
      simp only [constTyped, beq_iff_eq] at typed ⊢; subst typed; simp [SemTy.subst]
  | .character _, type, typed => by
      cases type <;> simp_all [constTyped, SemTy.subst]
  | .integer _, type, typed => by
      cases type <;> simp_all [constTyped, SemTy.subst]
  | .address _, type, typed => by
      simp only [constTyped, beq_iff_eq] at typed ⊢; subst typed; simp [SemTy.subst]
  | .string _, type, typed => by
      simp only [constTyped, beq_iff_eq] at typed ⊢; subst typed; simp [SemTy.subst]
  | .bytes _, type, typed => by
      simp only [constTyped, beq_iff_eq] at typed ⊢; subst typed; simp [SemTy.subst]
  | .vector elements, type, typed => by
      cases type with
      | vector element length =>
          simp only [constTyped, Bool.and_eq_true] at typed
          simp only [SemTy.subst, constTyped, Bool.and_eq_true]
          exact ⟨constsTypedEach_subst elements.toList element typed.1, typed.2⟩
      | _ => simp [constTyped] at typed
  | .tuple elements, type, typed => by
      cases type with
      | tuple types =>
          simp only [constTyped] at typed
          simp only [SemTy.subst, SemTy.substList_eq_map, constTyped]
          exact constsTyped_subst elements.toList types typed
      | _ => simp [constTyped] at typed
  | .profile _, type, typed => by simp [constTyped] at typed

theorem constsTypedEach_subst {pointerWidth : Option Nat} :
    ∀ (values : List ConstValue) (type : SemTy), constsTypedEach pointerWidth values type = true →
      constsTypedEach pointerWidth values (type.subst arguments) = true
  | [], _, _ => rfl
  | value :: values, type, typed => by
      simp only [constsTypedEach, Bool.and_eq_true] at typed ⊢
      exact ⟨constTyped_subst value type typed.1, constsTypedEach_subst values type typed.2⟩

theorem constsTyped_subst {pointerWidth : Option Nat} :
    ∀ (values : List ConstValue) (types : List SemTy), constsTyped pointerWidth values types = true →
      constsTyped pointerWidth values (types.map (·.subst arguments)) = true
  | [], [], _ => rfl
  | value :: values, type :: types, typed => by
      simp only [constsTyped, Bool.and_eq_true, List.map_cons] at typed ⊢
      exact ⟨constTyped_subst value type typed.1, constsTyped_subst values types typed.2⟩
  | [], _ :: _, typed | _ :: _, [], typed => by simp [constsTyped] at typed
end

theorem all_eq_subst {types : List SemTy} {type : SemTy}
    (all : types.all (· == type) = true) :
    (types.map (·.subst arguments)).all (· == type.subst arguments) = true := by
  simp only [List.all_eq_true, beq_iff_eq, List.mem_map] at all ⊢
  rintro _ ⟨element, member, rfl⟩
  rw [all element member]

theorem primitiveTyped_subst {pointerWidth : Option Nat} {operation : PrimitiveOperation}
    {operands : List SemTy} {result : SemTy}
    (typed : primitiveTyped pointerWidth operation operands result = true) :
    primitiveTyped pointerWidth operation (operands.map (·.subst arguments))
      (result.subst arguments) = true := by
  have closedResult : fixedInteger pointerWidth result = true → result.subst arguments = result :=
    fixedInteger_subst
  cases operation <;> simp only [primitiveTyped] at typed ⊢
  case tuple =>
    simp only [beq_iff_eq] at typed ⊢; subst typed; simp [SemTy.subst, SemTy.substList_eq_map]
  case vector =>
    cases result with
    | vector element length =>
        simp only [Bool.and_eq_true] at typed
        simp only [SemTy.subst, Bool.and_eq_true, List.length_map]
        exact ⟨all_eq_subst typed.1, typed.2⟩
    | _ => simp at typed
  case repeatVector =>
    cases result with
    | vector element length =>
        cases length with
        | some length =>
            cases length with
            | integer count =>
                match operands, typed with
                | [operand], typed =>
                    simp only [beq_iff_eq] at typed
                    subst typed
                    simp [SemTy.subst]
            | _ => simp at typed
        | none => simp at typed
    | _ => simp at typed
  case pushVector =>
    match operands, typed with
    | [.vector element length, value], typed =>
        simp only [Bool.and_eq_true, beq_iff_eq] at typed
        obtain ⟨rfl, rfl⟩ := typed
        simp [SemTy.subst]
  case concatVector =>
    match operands, typed with
    | [.vector element length, .vector other otherLength], typed =>
        simp only [Bool.and_eq_true, beq_iff_eq] at typed
        obtain ⟨rfl, rfl⟩ := typed
        simp [SemTy.subst]
  case insertVector =>
    match operands, typed with
    | [.vector element length, index, value], typed =>
        simp only [Bool.and_eq_true, beq_iff_eq] at typed
        obtain ⟨⟨integral_, rfl⟩, rfl⟩ := typed
        simp [SemTy.subst, integral_subst integral_, integral_]
  case removeVector =>
    match operands, typed with
    | [.vector element length, index], typed =>
        simp only [Bool.and_eq_true, beq_iff_eq] at typed
        obtain ⟨integral_, rfl⟩ := typed
        simp [SemTy.subst, SemTy.substList, integral_subst integral_, integral_]
  case swapVector | reverseSliceVector | slice =>
    match operands, typed with
    | [.vector element length, first, second], typed =>
        simp only [Bool.and_eq_true, beq_iff_eq] at typed
        obtain ⟨⟨first_, second_⟩, rfl⟩ := typed
        simp [SemTy.subst, integral_subst first_, integral_subst second_, first_, second_]
  case containsVector | signerAddress | logicalAnd | logicalOr | logicalNot | equal | notEqual
      | less | greater | lessEqual | greaterEqual =>
    simp only [beq_iff_eq] at typed ⊢; subst typed; simp [SemTy.subst]
  case indexOfVector =>
    match result, typed with
    | .tuple [.bool, index], typed => simp [SemTy.subst, SemTy.substList, integral_subst typed, typed]
  case destroyEmptyVector | checkVectorIndex =>
    have := packs_subst (arguments := arguments) typed
    simpa using this
  case length => rw [integral_subst typed]; exact typed
  case index =>
    match operands, typed with
    | [.vector element length, index], typed =>
        simp only [Bool.and_eq_true, beq_iff_eq] at typed
        obtain ⟨integral_, rfl⟩ := typed
        simp [SemTy.subst, integral_subst integral_, integral_]
  case compare =>
    simp only [Bool.and_eq_true] at typed
    rw [holds_subst typed.1.1]
    simp [typed]
  case add | subtract | multiply | divide | modulo | checkedAdd | checkedSubtract
      | checkedMultiply | checkedDivide | checkedModulo | negate | checkedNegate | bitwiseNot
      | shiftLeft | shiftRight | checkedShiftLeft | checkedShiftRight =>
    rw [closedResult typed]; exact typed
  case bitwiseOr | bitwiseAnd | bitwiseXor =>
    simp only [Bool.or_eq_true, Bool.and_eq_true, beq_iff_eq] at typed ⊢
    rcases typed with ⟨rfl, rfl⟩ | ⟨integer, rfl⟩
    · exact .inl ⟨by simp [SemTy.subst], by simp [SemTy.subst]⟩
    · exact .inr ⟨by rw [closedResult integer]; exact integer, by simp [closedResult integer]⟩
  case cast | checkedCast =>
    simp only [Bool.or_eq_true, beq_iff_eq] at typed ⊢
    rcases typed with integer | rfl
    · exact .inl (by rw [closedResult integer]; exact integer)
    · exact .inr (by simp [SemTy.subst])
  case overflowingAdd | overflowingSubtract | overflowingMultiply =>
    match result, typed with
    | .tuple [value, .bool], typed => simp [SemTy.subst, SemTy.substList, fixedInteger_subst typed, typed]
  case copyValue | moveValue =>
    simp only [beq_iff_eq] at typed ⊢; subst typed; rfl

private theorem List.mapM_argSubst {α : Type} {f g : α → Option SemArg} :
    ∀ {xs : List α} {ys : List SemArg}, xs.mapM f = some ys →
      (∀ x y, f x = some y → g x = some (y.subst arguments)) →
      xs.mapM g = some (ys.map (·.subst arguments))
  | [], ys, mapped, _ => by simp_all
  | x :: xs, ys, mapped, step => by
      simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff,
        Option.pure_def, Option.some.injEq] at mapped ⊢
      obtain ⟨y, y_eq, rest, rest_eq, rfl⟩ := mapped
      exact ⟨_, step x y y_eq, _, List.mapM_argSubst rest_eq step, by simp⟩

theorem resolveArguments_subst {ns : ValidatedNamespace} {env : Array SemArg}
    {instantiations : Array GenericArgument} {resolved : Array SemArg}
    (found : resolveArguments ns env instantiations = some resolved) :
    resolveArguments ns (env.map (·.subst arguments)) instantiations =
      some (resolved.map (·.subst arguments)) := by
  unfold resolveArguments at found ⊢
  rw [Array.mapM_eq_mapM_toList] at found ⊢
  simp only [Functor.map, Option.map_eq_some_iff] at found ⊢
  obtain ⟨list, list_eq, rfl⟩ := found
  refine ⟨list.map (·.subst arguments), List.mapM_argSubst list_eq ?_, by simp⟩
  intro argument result step
  cases argument with
  | typeArg value =>
      simp only [Option.map_eq_some_iff] at step ⊢
      obtain ⟨type, type_eq, rfl⟩ := step
      exact ⟨_, resolveIn_subst arguments type_eq, rfl⟩
  | const value => simp only [Option.some.injEq] at step; subst step; rfl
  | lifetime value => simp only [Option.some.injEq] at step; subst step; rfl
  | evidence value => simp only [Option.some.injEq] at step; subst step; rfl

private theorem List.mapM_subst {α : Type} {f g : α → Option SemTy} :
    ∀ {xs : List α} {ys : List SemTy}, xs.mapM f = some ys →
      (∀ x y, f x = some y → g x = some (y.subst arguments)) →
      xs.mapM g = some (ys.map (·.subst arguments))
  | [], ys, mapped, _ => by simp_all
  | x :: xs, ys, mapped, step => by
      simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff,
        Option.pure_def, Option.some.injEq] at mapped ⊢
      obtain ⟨y, y_eq, rest, rest_eq, rfl⟩ := mapped
      exact ⟨_, step x y y_eq, _, List.mapM_subst rest_eq step, by simp⟩

theorem fieldType?_subst {targetNs : ValidatedNamespace} {declaration : StructDecl}
    {variant : Option String} {nominalArguments : List SemArg} {fieldName : String}
    {type : SemTy}
    (found : fieldType? targetNs declaration variant nominalArguments fieldName = some type) :
    fieldType? targetNs declaration variant (nominalArguments.map (·.subst arguments)) fieldName =
      some (type.subst arguments) := by
  simp only [fieldType?, Option.bind_eq_bind, Option.bind_eq_some_iff] at found ⊢
  obtain ⟨fields, fields_eq, field, field_eq, resolved⟩ := found
  refine ⟨fields, fields_eq, field, field_eq, ?_⟩
  have := resolveIn_subst arguments resolved
  simpa using this

theorem fieldTypes?_subst {targetNs : ValidatedNamespace} {declaration : StructDecl}
    {variant : Option String} {nominalArguments : List SemArg} {types : List SemTy}
    (found : fieldTypes? targetNs declaration variant nominalArguments = some types) :
    fieldTypes? targetNs declaration variant (nominalArguments.map (·.subst arguments)) =
      some (types.map (·.subst arguments)) := by
  simp only [fieldTypes?, Option.bind_eq_bind, Option.bind_eq_some_iff] at found ⊢
  obtain ⟨fields, fields_eq, resolved⟩ := found
  refine ⟨fields, fields_eq, List.mapM_subst resolved fun field type found => ?_⟩
  have := resolveIn_subst arguments found
  simpa using this

theorem signatureTypes?_subst {targetNs : ValidatedNamespace}
    {declaration : FunctionDecl FunctionBody} {env : Array SemArg}
    {parameters results : List SemTy}
    (found : signatureTypes? targetNs declaration env = some (parameters, results)) :
    signatureTypes? targetNs declaration (env.map (·.subst arguments)) =
      some (parameters.map (·.subst arguments), results.map (·.subst arguments)) := by
  simp only [signatureTypes?, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.some.injEq,
    Prod.mk.injEq] at found ⊢
  obtain ⟨parameters', parameters_eq, results', results_eq, rfl, rfl⟩ := found
  exact ⟨_, List.mapM_subst parameters_eq fun _ _ found => resolveIn_subst arguments found,
    _, List.mapM_subst results_eq fun _ _ found => resolveIn_subst arguments found, rfl, rfl⟩

private theorem List.filterMapM_subst {α : Type} {f g : α → Option (Option SemTy)} :
    ∀ {xs : List α} {ys : List SemTy}, xs.filterMapM f = some ys →
      (∀ x result, f x = some result → g x = some (result.map (·.subst arguments))) →
      xs.filterMapM g = some (ys.map (·.subst arguments))
  | [], ys, mapped, _ => by simp_all [List.filterMapM_nil]
  | x :: xs, ys, mapped, step => by
      simp only [List.filterMapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff] at mapped ⊢
      obtain ⟨head, head_eq, mapped⟩ := mapped
      refine ⟨_, step x head head_eq, ?_⟩
      cases head with
      | none =>
          simpa using List.filterMapM_subst (by simpa using mapped) step
      | some type =>
          simp only [Option.bind_eq_some_iff, pure, Option.some.injEq] at mapped
          obtain ⟨rest, rest_eq, rfl⟩ := mapped
          simp only [Option.map_some, Option.bind_eq_some_iff, pure, Option.some.injEq]
          exact ⟨_, List.filterMapM_subst rest_eq step, by simp⟩

theorem fieldTypesAcross?_subst {targetNs : ValidatedNamespace} {declaration : StructDecl}
    {nominalArguments : List SemArg} {fieldName : String} {types : List SemTy}
    (found : fieldTypesAcross? targetNs declaration nominalArguments fieldName = some types) :
    fieldTypesAcross? targetNs declaration (nominalArguments.map (·.subst arguments)) fieldName =
      some (types.map (·.subst arguments)) := by
  unfold fieldTypesAcross? at found ⊢
  split at found
  · rename_i empty
    simp only [empty, if_true, Option.map_eq_some_iff] at found ⊢
    obtain ⟨type, type_eq, rfl⟩ := found
    exact ⟨_, fieldType?_subst type_eq, rfl⟩
  · rename_i nonempty
    simp only [nonempty, Bool.false_eq_true, if_false] at found ⊢
    refine List.filterMapM_subst found fun variant result step => ?_
    simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at step ⊢
    obtain ⟨name, name_eq, fields, fields_eq, step⟩ := step
    refine ⟨name, name_eq, fields, fields_eq, ?_⟩
    split at step
    · rename_i holds
      simp only [holds, if_true, Functor.map, Option.map_eq_some_iff] at step ⊢
      obtain ⟨type, type_eq, rfl⟩ := step
      exact ⟨_, fieldType?_subst type_eq, rfl⟩
    · rename_i fails
      simp only [fails, Bool.false_eq_true, if_false, Option.some.injEq] at step ⊢
      subst step
      rfl

theorem fieldTypeAt?_subst {targetNs : ValidatedNamespace} {declaration : StructDecl}
    {variant : Option String} {nominalArguments : List SemArg} {fieldName : String}
    {type : SemTy}
    (found : fieldTypeAt? targetNs declaration variant nominalArguments fieldName = some type) :
    fieldTypeAt? targetNs declaration variant (nominalArguments.map (·.subst arguments))
      fieldName = some (type.subst arguments) := by
  cases variant with
  | some name => exact fieldType?_subst found
  | none =>
      simp only [fieldTypeAt?] at found ⊢
      split at found
      · rename_i first rest types_eq
        split at found
        · rename_i agree
          simp only [Option.some.injEq] at found
          subst found
          rw [fieldTypesAcross?_subst types_eq]
          simp only [List.map_cons]
          rw [if_pos (all_eq_subst agree)]
        · cases found
      · cases found

namespace Context

theorem nominalOperand_subst {operand : SemTy} {name : QualifiedName}
    {nominalArguments : List SemArg}
    (found : nominalOperand operand = some (name, nominalArguments)) :
    nominalOperand (operand.subst arguments) =
      some (name, nominalArguments.map (·.subst arguments)) := by
  cases operand with
  | nominal name' arguments' =>
      simp only [nominalOperand, Option.some.injEq, Prod.mk.injEq] at found
      obtain ⟨rfl, rfl⟩ := found
      simp [SemTy.subst, SemArg.substList_eq_map, nominalOperand]
  | reference kind referent =>
      cases referent <;> simp only [nominalOperand, reduceCtorEq] at found
      simp only [Option.some.injEq, Prod.mk.injEq] at found
      obtain ⟨rfl, rfl⟩ := found
      simp [SemTy.subst, SemArg.substList_eq_map, nominalOperand]
  | _ => simp [nominalOperand] at found

theorem selected_subst {operand result : SemTy} {field field' : SemTy → Bool}
    (step : ∀ type, field type = true → field' (type.subst arguments) = true)
    (chosen : selected operand result field = true) :
    selected (operand.subst arguments) (result.subst arguments) field' = true := by
  unfold selected at chosen ⊢
  simp only [Bool.or_eq_true] at chosen ⊢
  rcases chosen with direct | through
  · exact .inl (step _ direct)
  · right
    split at through
    · rename_i kind _ resultKind referent
      simp only [Bool.and_eq_true] at through
      simp only [SemTy.subst, Bool.and_eq_true]
      exact ⟨through.1, step _ through.2⟩
    · cases through

end Context

namespace Context

theorem typeOf_subst {context : Context} {typeId : TypeId} {type : SemTy}
    (found : context.typeOf typeId = some type) :
    (context.subst arguments).typeOf typeId = some (type.subst arguments) :=
  resolveIn_subst arguments found

theorem exprType_subst {context : Context} {id : ExprId} {type : SemTy}
    (found : context.exprType id = some type) :
    (context.subst arguments).exprType id = some (type.subst arguments) := by
  simp only [exprType, Option.bind_eq_bind, Option.bind_eq_some_iff] at found ⊢
  obtain ⟨expression, expression_eq, found⟩ := found
  exact ⟨expression, expression_eq, typeOf_subst found⟩

theorem localType_subst {context : Context} {id : LocalId} {type : SemTy}
    (found : context.localType id = some type) :
    (context.subst arguments).localType id = some (type.subst arguments) := by
  simp only [localType, Option.bind_eq_bind, Option.bind_eq_some_iff] at found ⊢
  obtain ⟨declaration, declaration_eq, found⟩ := found
  exact ⟨declaration, declaration_eq, typeOf_subst found⟩

theorem stops_subst {context : Context} {id : ExprId} (stopped : context.stops id = true) :
    (context.subst arguments).stops id = true := by
  simp only [stops, Bool.or_eq_true, beq_iff_eq] at stopped ⊢
  rcases stopped with never | diverted
  · exact .inl (by rw [exprType_subst never]; rfl)
  · exact .inr diverted

theorem flows_subst {context : Context} {id : ExprId} {type : SemTy}
    (flowing : context.flows id type = true) :
    (context.subst arguments).flows id (type.subst arguments) = true := by
  simp only [flows, Bool.or_eq_true, beq_iff_eq] at flowing ⊢
  rcases flowing with typed | stopped
  · exact .inl (exprType_subst typed)
  · exact .inr (stops_subst stopped)


theorem placeType_subst {context : Context} :
    ∀ (fuel : Nat) {id : PlaceId} {type : SemTy} {variant : Option String},
      context.placeType fuel id = some (type, variant) →
      (context.subst arguments).placeType fuel id = some (type.subst arguments, variant)
  | 0, _, _, _, found => by simp [placeType] at found
  | fuel + 1, id, type, variant, found => by
      unfold placeType at found ⊢
      simp only [subst_ns]
      cases place_eq : context.ns.places[id.index]? with
      | none => simp [place_eq] at found
      | some place =>
      rw [place_eq] at found
      cases place with
      | localVar localId =>
          simp only [Functor.map, Option.map_eq_some_iff, Prod.mk.injEq] at found ⊢
          obtain ⟨type', found, rfl, rfl⟩ := found
          exact ⟨_, localType_subst found, rfl, rfl⟩
      | deref base =>
          simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at found ⊢
          obtain ⟨⟨baseType, baseVariant⟩, base_eq, found⟩ := found
          refine ⟨_, placeType_subst fuel base_eq, ?_⟩
          cases baseType with
          | reference kind referent =>
              simp only [SemTy.subst, subst_unit] at found ⊢
              split at found
              · rename_i agrees
                simp only [Option.some.injEq, Prod.mk.injEq] at found
                obtain ⟨rfl, rfl⟩ := found
                simp [agrees]
              · cases found
          | _ => simp at found
      | field base owner field =>
          simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at found ⊢
          obtain ⟨⟨baseType, baseVariant⟩, base_eq, found⟩ := found
          refine ⟨_, placeType_subst fuel base_eq, ?_⟩
          cases baseType with
          | nominal name nominalArguments =>
              simp only [SemTy.subst, SemArg.substList_eq_map, subst_unit,
                Option.bind_eq_some_iff] at found ⊢
              obtain ⟨⟨targetNs, declaration, spelled⟩, target, found⟩ := found
              refine ⟨_, target, ?_⟩
              split at found
              · cases found
              rename_i same
              rw [if_neg same]
              simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at found ⊢
              obtain ⟨fieldName, fieldName_eq, fieldType, fieldType_eq, rfl, rfl⟩ := found
              exact ⟨fieldName, fieldName_eq, _, fieldTypeAt?_subst fieldType_eq, rfl, rfl⟩
          | _ => simp at found
      | index base index =>
          simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at found ⊢
          obtain ⟨⟨baseType, baseVariant⟩, base_eq, found⟩ := found
          refine ⟨_, placeType_subst fuel base_eq, ?_⟩
          cases baseType with
          | vector element length =>
              simp only [SemTy.subst] at found ⊢
              split at found
              · rename_i indexType indexType_eq
                rw [exprType_subst indexType_eq]
                split at found
                · rename_i integral_
                  simp only [Option.some.injEq, Prod.mk.injEq] at found
                  obtain ⟨rfl, rfl⟩ := found
                  simp [integral_subst integral_, integral_]
                · cases found
              · cases found
          | tuple elements =>
              simp only [SemTy.subst, SemTy.substList_eq_map] at found ⊢
              split at found
              · split at found
                · cases found
                rename_i nonnegative
                rw [if_neg nonnegative]
                simp only [Functor.map, Option.map_eq_some_iff, Prod.mk.injEq] at found ⊢
                obtain ⟨element, element_eq, rfl, rfl⟩ := found
                exact ⟨_, by simp [element_eq], rfl, rfl⟩
              · cases found
          | _ => simp at found
      | subslice base start stop fromEnd =>
          simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at found ⊢
          obtain ⟨⟨baseType, baseVariant⟩, base_eq, found⟩ := found
          refine ⟨_, placeType_subst fuel base_eq, ?_⟩
          cases baseType with
          | vector element length =>
              simp only [SemTy.subst, Option.some.injEq, Prod.mk.injEq] at found ⊢
              obtain ⟨rfl, rfl⟩ := found
              exact ⟨rfl, rfl⟩
          | _ => simp at found
      | downcast base variantId =>
          simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at found ⊢
          obtain ⟨⟨baseType, baseVariant⟩, base_eq, found⟩ := found
          refine ⟨_, placeType_subst fuel base_eq, ?_⟩
          cases baseType with
          | nominal name nominalArguments =>
              simp only [SemTy.subst, Option.bind_eq_some_iff, Option.some.injEq,
                Prod.mk.injEq] at found ⊢
              obtain ⟨variantName, variantName_eq, rfl, rfl⟩ := found
              exact ⟨variantName, variantName_eq, rfl, rfl⟩
          | _ => simp at found

theorem placeTypeOf_subst {context : Context} {id : PlaceId} {type : SemTy}
    (found : context.placeTypeOf id = some type) :
    (context.subst arguments).placeTypeOf id = some (type.subst arguments) := by
  simp only [placeTypeOf, Functor.map, Option.map_eq_some_iff] at found ⊢
  obtain ⟨⟨type', variant⟩, found, rfl⟩ := found
  exact ⟨_, placeType_subst _ found, rfl⟩


theorem zip_all_subst {α : Type} {xs : List α} {types : List SemTy}
    {check check' : α × SemTy → Bool}
    (step : ∀ x type, check (x, type) = true → check' (x, type.subst arguments) = true)
    (all : (xs.zip types).all check = true) :
    (xs.zip (types.map (·.subst arguments))).all check' = true := by
  induction xs generalizing types with
  | nil => simp
  | cons x rest ih =>
      cases types with
      | nil => simp
      | cons type rest' =>
          simp only [List.zip_cons_cons, List.all_cons, Bool.and_eq_true, List.map_cons] at all ⊢
          exact ⟨step x type all.1, ih all.2⟩

theorem patternTypedFuel_subst {context : Context} :
    ∀ (fuel : Nat) {id : PatternId} {type : SemTy},
      context.patternTypedFuel fuel id type = true →
      (context.subst arguments).patternTypedFuel fuel id (type.subst arguments) = true
  | 0, _, _, typed => by simp [patternTypedFuel] at typed
  | fuel + 1, id, type, typed => by
      unfold patternTypedFuel at typed ⊢
      simp only [subst_ns]
      cases pattern_eq : context.ns.patterns[id.index]? with
      | none => simp [pattern_eq] at typed
      | some pattern =>
      rw [pattern_eq] at typed
      obtain ⟨loc, typeId, kind⟩ := pattern
      cases kind with
      | wildcard => rfl
      | «variable» localId =>
          simp only [beq_iff_eq] at typed ⊢
          exact localType_subst typed
      | tuple elements =>
          cases type with
          | tuple types =>
              simp only [Bool.and_eq_true, beq_iff_eq] at typed
              simp only [SemTy.subst, SemTy.substList_eq_map, Bool.and_eq_true, beq_iff_eq,
                List.length_map]
              exact ⟨typed.1, zip_all_subst (fun pattern type typed =>
                patternTypedFuel_subst fuel typed) typed.2⟩
          | _ => simp at typed
      | constructor name instantiations variant fields =>
          simp only [subst_unit, subst_env] at typed ⊢
          split at typed
          · rename_i targetNs declaration spelled resolved target resolved_eq
            rw [target, resolveArguments_subst (arguments := arguments) resolved_eq]
            simp only [Bool.and_eq_true, beq_iff_eq] at typed ⊢
            obtain ⟨rfl, typed⟩ := typed
            refine ⟨by simp [SemTy.subst, SemArg.substList_eq_map], ?_⟩
            split at typed
            · rename_i types types_eq
              have := fieldTypes?_subst (arguments := arguments) types_eq
              rw [Array.toList_map, this]
              simp only [Bool.and_eq_true, beq_iff_eq, List.length_map] at typed ⊢
              exact ⟨typed.1, zip_all_subst (fun pattern type typed =>
                patternTypedFuel_subst fuel typed) typed.2⟩
            · cases typed
          · cases typed
      | literal value =>
          exact constTyped_subst _ _ typed
      | range lower upper inclusive =>
          simp only [Bool.or_eq_true, beq_iff_eq] at typed ⊢
          rcases typed with integral_ | rfl
          · exact .inl (by rw [integral_subst integral_]; exact integral_)
          · exact .inr (by simp [SemTy.subst])

theorem patternTyped_subst {context : Context} {id : PatternId} {type : SemTy}
    (typed : context.patternTyped id type = true) :
    (context.subst arguments).patternTyped id (type.subst arguments) = true :=
  patternTypedFuel_subst _ typed

theorem extract_map {α β : Type} (f : α → β) :
    ∀ (mask : Nat) (captured : Bool) (values : List α),
      ClosureMask.extract mask captured (values.map f) =
        (ClosureMask.extract mask captured values).map f
  | _, _, [] => rfl
  | mask, captured, value :: values => by
      simp only [List.map_cons, ClosureMask.extract, extract_map f (mask / 2) captured values]
      split <;> rfl

@[simp] theorem subst_consults (context : Context) :
    (context.subst arguments).consults = context.consults := rfl

@[simp] theorem subst_edgeClosed (context : Context) :
    (context.subst arguments).edgeClosed = context.edgeClosed := rfl

theorem checkCall_subst {context : Context} {result : SemTy} {kind : CallKind}
    {instantiations : Array GenericArgument} {operands : Array ExprId}
    (checked : context.checkCall result kind instantiations operands = true) :
    (context.subst arguments).checkCall (result.subst arguments) kind instantiations operands =
      true := by
  cases kind with
  | function reference =>
      unfold checkCall at checked ⊢
      simp only [subst_unit, subst_ns, subst_env, subst_edgeClosed] at checked ⊢
      split at checked
      · rename_i target_eq
        rw [target_eq]
      · rename_i targetNs declaration resolved target_eq resolved_eq
        rw [target_eq, resolveArguments_subst (arguments := arguments) resolved_eq]
        simp only [Bool.and_eq_true, beq_iff_eq, Array.size_map] at checked ⊢
        obtain ⟨⟨⟨arity, kinds⟩, edge⟩, checked⟩ := checked
        refine ⟨⟨⟨arity, kinds⟩, edge⟩, ?_⟩
        split at checked
        · rename_i parameters results signature_eq
          rw [signatureTypes?_subst (arguments := arguments) signature_eq]
          simp only [Bool.and_eq_true, beq_iff_eq, List.length_map] at checked ⊢
          exact ⟨⟨checked.1.1, zip_all_subst (fun operand parameter flowing =>
            flows_subst flowing) checked.1.2⟩, packs_subst checked.2⟩
        · cases checked
      · cases checked
  | constructor reference variant =>
      unfold checkCall at checked ⊢
      simp only [subst_unit, subst_ns, subst_env] at checked ⊢
      split at checked
      · rename_i target_eq
        rw [target_eq]
        exact checked
      · rename_i targetNs declaration spelled resolved target_eq resolved_eq
        rw [target_eq, resolveArguments_subst (arguments := arguments) resolved_eq]
        split at checked
        · rename_i fields fields_eq
          simp only [Array.toList_map, fieldTypes?_subst (arguments := arguments) fields_eq]
          simp only [Bool.and_eq_true, beq_iff_eq, List.length_map] at checked ⊢
          obtain ⟨⟨size_eq, all⟩, rfl⟩ := checked
          exact ⟨⟨size_eq, zip_all_subst (fun operand field flowing => flows_subst flowing) all⟩,
            by simp [SemTy.subst, SemArg.substList_eq_map]⟩
        · cases checked
      · cases checked
  | destructor reference variant =>
      unfold checkCall at checked ⊢
      simp only [subst_unit, subst_ns, subst_env] at checked ⊢
      split at checked
      · rename_i target_eq
        rw [target_eq]
        exact checked
      · rename_i targetNs declaration spelled resolved operand target_eq resolved_eq operands_eq
        rw [target_eq, resolveArguments_subst (arguments := arguments) resolved_eq, operands_eq]
        simp only [Bool.and_eq_true, beq_iff_eq] at checked ⊢
        obtain ⟨operand_eq, checked⟩ := checked
        refine ⟨by rw [exprType_subst operand_eq]; simp [SemTy.subst, SemArg.substList_eq_map], ?_⟩
        split at checked
        · rename_i fields fields_eq
          simp only [Array.toList_map, fieldTypes?_subst (arguments := arguments) fields_eq]
          exact packs_subst checked
        · cases checked
      · cases checked
  | closure reference mask =>
      unfold checkCall at checked ⊢
      simp only [subst_unit, subst_ns, subst_env, subst_edgeClosed] at checked ⊢
      split at checked
      · rename_i target_eq
        rw [target_eq]
      · rename_i targetNs declaration resolved openTypes resultType target_eq resolved_eq
        rw [target_eq, resolveArguments_subst (arguments := arguments) resolved_eq]
        simp only [SemTy.subst, SemTy.substList_eq_map]
        simp only [Bool.and_eq_true, beq_iff_eq, Array.size_map] at checked ⊢
        obtain ⟨⟨⟨⟨arity, kinds⟩, edge⟩, bound⟩, checked⟩ := checked
        refine ⟨⟨⟨⟨arity, kinds⟩, edge⟩, bound⟩, ?_⟩
        split at checked
        · rename_i parameters results signature_eq
          rw [signatureTypes?_subst (arguments := arguments) signature_eq]
          simp only [Bool.and_eq_true, beq_iff_eq, extract_map, List.length_map] at checked ⊢
          obtain ⟨⟨⟨captures_size, captures⟩, rfl⟩, packed⟩ := checked
          exact ⟨⟨⟨captures_size, zip_all_subst (fun capture parameter flowing =>
            flows_subst flowing) captures⟩, rfl⟩, packs_subst packed⟩
        · cases checked
      · cases checked
  | invoke =>
      simp only [checkCall] at checked ⊢
      split at checked
      · rename_i callee supplied operands_eq
        split at checked
        · rename_i parameters resultType callee_eq
          rw [exprType_subst callee_eq]
          simp only [SemTy.subst, SemTy.substList_eq_map]
          simp only [Bool.and_eq_true, beq_iff_eq, List.length_map] at checked ⊢
          obtain ⟨⟨length, all⟩, rfl⟩ := checked
          exact ⟨⟨length, zip_all_subst (fun operand parameter flowing =>
            flows_subst flowing) all⟩, rfl⟩
        · cases checked
      · cases checked
  | extension _ _ => simp [checkCall] at checked

private theorem List.mapM_exprType_subst {context : Context} :
    ∀ {operands : List ExprId} {types : List SemTy},
      operands.mapM context.exprType = some types →
      operands.mapM (context.subst arguments).exprType = some (types.map (·.subst arguments))
  | [], types, mapped => by simp_all
  | operand :: rest, types, mapped => by
      simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff,
        Option.pure_def, Option.some.injEq] at mapped ⊢
      obtain ⟨type, type_eq, tail, tail_eq, rfl⟩ := mapped
      exact ⟨_, exprType_subst type_eq, _, List.mapM_exprType_subst tail_eq, by simp⟩

theorem checkData_subst {context : Context} {result : SemTy} {operation : DataOperation}
    {operands : Array ExprId} (checked : context.checkData result operation operands = true) :
    (context.subst arguments).checkData (result.subst arguments) operation operands = true := by
  cases operation with
  | select reference field =>
      simp only [checkData, subst_unit, subst_ns] at checked ⊢
      split at checked
      · rename_i targetNs declaration spelled operand target_eq operands_eq
        split at checked
        · rename_i operandType operandType_eq
          rw [exprType_subst operandType_eq]
          simp only
          split at checked
          · rename_i name nominalArguments operand_eq
            rw [nominalOperand_subst operand_eq]
            simp only [Bool.and_eq_true, beq_iff_eq] at checked ⊢
            obtain ⟨rfl, checked⟩ := checked
            refine ⟨rfl, ?_⟩
            split at checked
            · rename_i types types_eq
              rw [fieldTypesAcross?_subst types_eq]
              simp only [Bool.and_eq_true, List.isEmpty_map] at checked ⊢
              exact ⟨checked.1, selected_subst (fun _ all => all_eq_subst all) checked.2⟩
            · cases checked
          · cases checked
        · cases checked
      · cases checked
  | selectVariants reference fields =>
      simp only [checkData, subst_unit, subst_ns] at checked ⊢
      split at checked
      · rename_i targetNs declaration spelled operand target_eq operands_eq
        split at checked
        · rename_i operandType operandType_eq
          rw [exprType_subst operandType_eq]
          simp only
          split at checked
          · rename_i name nominalArguments operand_eq
            rw [nominalOperand_subst operand_eq]
            simp only [Bool.and_eq_true, beq_iff_eq] at checked ⊢
            obtain ⟨rfl, checked⟩ := checked
            refine ⟨rfl, selected_subst (fun type all => ?_) checked⟩
            simp only [Array.all_eq_true', beq_iff_eq] at all ⊢
            intro field member
            exact fieldType?_subst (all field member)
          · cases checked
        · cases checked
      · cases checked
  | testVariants reference variants =>
      simp only [checkData, beq_iff_eq] at checked ⊢
      subst checked
      rfl
  | discriminant reference =>
      simp only [checkData, Bool.and_eq_true] at checked ⊢
      rw [integral_subst checked.1]
      exact checked
  | updateField reference field =>
      simp only [checkData, subst_unit, subst_ns] at checked ⊢
      split at checked
      · rename_i targetNs declaration spelled operand replacement target_eq operands_eq
        split at checked
        · rename_i name nominalArguments operand_eq
          rw [exprType_subst operand_eq]
          simp only [SemTy.subst, SemArg.substList_eq_map, Bool.and_eq_true, beq_iff_eq]
            at checked ⊢
          obtain ⟨⟨rfl, rfl⟩, checked⟩ := checked
          refine ⟨⟨rfl, by simp [SemTy.subst, SemArg.substList_eq_map]⟩, ?_⟩
          split at checked
          · rename_i types types_eq
            rw [fieldTypesAcross?_subst types_eq]
            simp only [List.all_eq_true, List.mem_map] at checked ⊢
            rintro _ ⟨type, member, rfl⟩
            exact flows_subst (checked type member)
          · cases checked
        · cases checked
      · cases checked

theorem checkOperation_subst {context : Context} {result : SemTy} {operation : Operation}
    {instantiations : Array GenericArgument} {operands : Array ExprId}
    (checked : context.checkOperation result operation instantiations operands = true) :
    (context.subst arguments).checkOperation (result.subst arguments) operation instantiations
      operands = true := by
  cases operation with
  | copy place | read place | move place =>
      simp only [checkOperation, Bool.and_eq_true, beq_iff_eq] at checked ⊢
      exact ⟨checked.1, placeTypeOf_subst checked.2⟩
  | borrow kind place =>
      simp only [checkOperation, Bool.and_eq_true] at checked ⊢
      refine ⟨checked.1, ?_⟩
      have checked := checked.2
      split at checked
      · rename_i referenceKind referent kind_eq referent_eq
        rw [kind_eq, placeTypeOf_subst referent_eq]
        simp only [beq_iff_eq] at checked ⊢
        subst checked
        rfl
      · cases checked
  | write place =>
      simp only [checkOperation] at checked ⊢
      split at checked
      · rename_i value type operands_eq type_eq
        rw [operands_eq, placeTypeOf_subst type_eq]
        simp only [Bool.and_eq_true] at checked ⊢
        exact ⟨flows_subst checked.1, by simpa using packs_subst checked.2⟩
      · cases checked
  | drop place =>
      simp only [checkOperation, Bool.and_eq_true, Option.isSome_iff_exists] at checked ⊢
      obtain ⟨⟨empty, type, type_eq⟩, packed⟩ := checked
      exact ⟨⟨empty, _, placeTypeOf_subst type_eq⟩, by simpa using packs_subst packed⟩
  | call kind =>
      simp only [checkOperation] at checked ⊢
      exact checkCall_subst checked
  | global kind =>
      simp only [checkOperation] at checked ⊢
      split at checked
      · rename_i resource instantiations_eq
        simp only [subst_consults, Bool.and_eq_true] at checked ⊢
        obtain ⟨consulted, checked⟩ := checked
        refine ⟨consulted, ?_⟩
        cases resource_eq : context.typeOf resource.typeId with
        | none => rw [resource_eq] at checked; cases checked
        | some resourceType =>
            rw [resource_eq] at checked
            rw [typeOf_subst resource_eq]
            simp only at checked ⊢
            rcases operands_eq : operands.toList with
              _ | ⟨first, _ | ⟨second, _ | ⟨third, rest⟩⟩⟩ <;>
              cases kind <;> simp only [operands_eq] at checked ⊢ <;>
              first
                | (simp at checked; done)
                | (simp only [beq_iff_eq] at checked ⊢; subst checked; rfl)
                | (simp only [Bool.and_eq_true] at checked ⊢
                   exact ⟨flows_subst checked.1, by simpa using packs_subst checked.2⟩)
                | (split at checked
                   · simp only [beq_iff_eq] at checked ⊢
                     subst checked
                     rfl
                   · cases checked)
      · cases checked
  | primitive operation =>
      simp only [checkOperation] at checked ⊢
      split at checked
      · rename_i types types_eq
        rw [List.mapM_exprType_subst types_eq]
        exact primitiveTyped_subst checked
      · cases checked
  | reference kind =>
      simp only [checkOperation] at checked ⊢
      split at checked
      · rename_i reference operands_eq
        split at checked
        · rename_i referenceKind referent reference_eq
          rw [exprType_subst reference_eq]
          simp only [SemTy.subst, Bool.and_eq_true, beq_iff_eq, subst_ns] at checked ⊢
          obtain ⟨rfl, agrees⟩ := checked
          exact ⟨rfl, agrees⟩
        · cases checked
      · rename_i freezeKind reference operands_eq
        split at checked
        · rename_i referenceKind referent reference_eq
          rw [exprType_subst reference_eq]
          simp only [SemTy.subst, Bool.and_eq_true, beq_iff_eq, subst_ns] at checked ⊢
          obtain ⟨rfl, agrees⟩ := checked
          exact ⟨rfl, agrees⟩
        · cases checked
      · rename_i reference value operands_eq
        split at checked
        · rename_i referent reference_eq
          rw [exprType_subst reference_eq]
          simp only [SemTy.subst, Bool.and_eq_true] at checked ⊢
          exact ⟨flows_subst checked.1, by simpa using packs_subst checked.2⟩
        · cases checked
      · rfl
      · cases checked
  | data operation =>
      simp only [checkOperation] at checked ⊢
      exact checkData_subst checked
  | assert =>
      simp only [checkOperation] at checked ⊢
      simpa using packs_subst checked
  | specification _ | profile _ _ => simp [checkOperation] at checked

/-- A type resolved without arguments has no parameter to substitute. -/
theorem resolveIn_closed {ns : ValidatedNamespace} {typeId : TypeId} {type : SemTy}
    (resolved : resolveIn ns #[] typeId = some type) : type.subst arguments = type := by
  have := resolveIn_subst arguments resolved
  simp only [Array.map_empty] at this
  rw [resolved, Option.some.injEq] at this
  exact this.symm

theorem checkNode_subst {context : Context} {id : ExprId} (checked : context.checkNode id = true) :
    (context.subst arguments).checkNode id = true := by
  unfold checkNode at checked ⊢
  simp only [subst_ns] at checked ⊢
  cases expression_eq : context.ns.expressions[id.index]? with
  | none => rw [expression_eq] at checked; cases checked
  | some expression =>
  rw [expression_eq] at checked
  simp only at checked ⊢
  cases type_eq : context.typeOf expression.typeId with
  | none => rw [type_eq] at checked; cases checked
  | some type =>
  rw [type_eq] at checked
  rw [typeOf_subst type_eq]
  simp only at checked ⊢
  obtain ⟨loc, typeId, kind⟩ := expression
  cases kind with
  | value literal source => exact constTyped_subst _ _ checked
  | constant reference =>
      simp only [subst_unit] at checked ⊢
      split at checked
      · rename_i targetNs declaration target_eq
        simp only [beq_iff_eq] at checked ⊢
        rw [resolveIn_closed (arguments := arguments) checked]
        exact checked
      · rfl
  | localVar localId =>
      simp only [beq_iff_eq] at checked ⊢
      exact localType_subst checked
  | operation operation instantiations operands surface => exact checkOperation_subst checked
  | block statements result =>
      cases result with
      | some value => exact flows_subst checked
      | none =>
          simp only [Bool.or_eq_true, Array.any_eq_true'] at checked ⊢
          rcases checked with packed | ⟨statement, member, stopped⟩
          · exact .inl (by simpa using packs_subst (results := []) packed)
          · exact .inr ⟨statement, member, stops_subst stopped⟩
  | letDecl pattern value body =>
      simp only [Bool.and_eq_true] at checked ⊢
      refine ⟨?_, flows_subst checked.2⟩
      have checked := checked.1
      cases value with
      | none => rfl
      | some value =>
          simp only [Bool.or_eq_true] at checked ⊢
          rcases checked with stopped | typed
          · exact .inl (stops_subst stopped)
          · right
            split at typed
            · rename_i valueType valueType_eq
              rw [exprType_subst valueType_eq]
              exact patternTyped_subst typed
            · cases typed
  | ifElse condition thenBranch elseBranch =>
      simp only [Bool.and_eq_true] at checked ⊢
      obtain ⟨⟨condition_, then_⟩, else_⟩ := checked
      refine ⟨⟨by simpa [SemTy.subst] using flows_subst (arguments := arguments) condition_,
        flows_subst then_⟩, ?_⟩
      cases elseBranch with
      | some elseBranch => exact flows_subst else_
      | none => simpa using packs_subst (results := []) else_
  | match_ scrutinee arms =>
      simp only [Bool.or_eq_true] at checked ⊢
      rcases checked with stopped | typed
      · exact .inl (stops_subst stopped)
      · right
        split at typed
        · rename_i scrutineeType scrutineeType_eq
          rw [exprType_subst scrutineeType_eq]
          simp only [Array.all_eq_true', Bool.and_eq_true, Option.all_eq_true] at typed ⊢
          intro arm member
          obtain ⟨⟨pattern_, guard_⟩, body_⟩ := typed arm member
          refine ⟨⟨patternTyped_subst pattern_, fun guard member' => ?_⟩, flows_subst body_⟩
          simpa [SemTy.subst] using flows_subst (arguments := arguments) (guard_ guard member')
        · cases typed
  | loop label body => rfl
  | break_ nest value =>
      simp only [subst_loops, List.getElem?_map] at checked ⊢
      split at checked
      · rename_i loopType loopType_eq
        rw [loopType_eq]
        simp only [Option.map_some]
        cases value with
        | some value => exact flows_subst checked
        | none => simpa using packs_subst (results := []) checked
      · cases checked
  | continue_ nest =>
      simpa using checked
  | return_ values =>
      simp only [subst_results] at checked ⊢
      split at checked
      · rename_i results results_eq
        rw [results_eq]
        simp only [Option.map_some, Bool.and_eq_true, beq_iff_eq, List.length_map] at checked ⊢
        exact ⟨checked.1, zip_all_subst (fun value result flowing => flows_subst flowing)
          checked.2⟩
      · cases checked
  | throw_ kind arguments' => rfl
  | assign place value =>
      simp only [Bool.and_eq_true] at checked ⊢
      refine ⟨?_, by simpa using packs_subst (results := []) checked.2⟩
      have checked := checked.1
      split at checked
      · rename_i placeType placeType_eq
        rw [placeTypeOf_subst placeType_eq]
        exact flows_subst checked
      · cases checked
  | assignPattern pattern value =>
      simp only [Bool.and_eq_true, Bool.or_eq_true] at checked ⊢
      refine ⟨?_, by simpa using packs_subst (results := []) checked.2⟩
      rcases checked.1 with stopped | typed
      · exact .inl (stops_subst stopped)
      · right
        split at typed
        · rename_i valueType valueType_eq
          rw [exprType_subst valueType_eq]
          exact patternTyped_subst typed
        · cases typed
  | quantifier kind binders triggers condition body => cases checked
  | spec block => simpa using packs_subst (results := []) checked

/-- The nodes a checked node runs, under semantic arguments: the same nodes,
each in its context under the arguments. -/
theorem children_subst {context : Context} {id : ExprId} (checked : context.checkNode id = true) :
    (context.subst arguments).children id =
      (context.children id).map fun (child, childId) => (child.subst arguments, childId) := by
  unfold children
  simp only [subst_ns]
  cases expression_eq : context.ns.expressions[id.index]? with
  | none => rfl
  | some expression =>
  simp only
  obtain ⟨loc, typeId, kind⟩ := expression
  cases kind with
  | loop label body =>
      unfold checkNode at checked
      rw [expression_eq] at checked
      simp only at checked
      cases type_eq : context.typeOf typeId with
      | none => rw [type_eq] at checked; cases checked
      | some type =>
          rw [typeOf_subst type_eq]
          simp [Context.subst]
  | spec block => rfl
  | quantifier kind binders triggers condition body => rfl
  | _ => simp [List.map_map, Function.comp_def]

theorem checkTree_subst :
    ∀ (fuel : Nat) {context : Context} {id : ExprId}, context.checkTree fuel id = true →
      (context.subst arguments).checkTree fuel id = true
  | 0, _, _, checked => by simp [checkTree] at checked
  | fuel + 1, context, id, checked => by
      simp only [checkTree, Bool.and_eq_true, List.all_eq_true] at checked ⊢
      refine ⟨checkNode_subst checked.1, ?_⟩
      rw [children_subst checked.1]
      simp only [List.mem_map, Prod.exists]
      rintro _ ⟨child, childId, member, rfl⟩
      exact checkTree_subst fuel (checked.2 _ member)
end Context

end Subst

end LeanerIR.Validation.StaticTyping
