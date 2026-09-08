-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

open Lean
open Lean.Elab.Command
open LeanerIR

leaner module 0x42::math where
  const ZERO : UInt<64> := 0
  const GREETING : string := "hello"
  const HEADER : Bytes := b[0, 127, 255]
  struct Pair {T : type has Copy, Drop} has Copy, Drop where
    first : T
    second : T
  struct Holder where
    pair : Pair::<UInt<64> >
  struct GenericKinds {N : const} {L : lifetime} {D : evidence} where
  enum Maybe has Copy, Drop where
    | none = 0
    | some (value : UInt<64>) = 1
  public fun increment (value : UInt<64>) -> UInt<64> :=
    value + 1
  fun guarded (value : UInt<64>) -> UInt<64> :=
    if core.prim.less(value, 1) then abort() else value
  spec increment where
    ensures core.prim.equal(result, core.prim.add(value, 1));

leaner namespace examples::rust_identity using rust where
  fun identity {T : type} (value : T) -> T := value

leaner namespace examples::rust_implicit_copy using rust where
  fun tuple_first(pair : (u32, Bool)) -> u32 := pair[0u32]
  fun vector_first(values : &Vector<u32>) -> u32 := values[0usize]

leaner namespace examples::comment_elaboration using rust where
  -- retained by ordinary command elaboration
  fun first() -> Unit := ()
  /- retained nested block
     /- inner -/
     comment -/
  fun second() -> Unit := ()

-- Documentation owned by the module declaration.
-- It must remain outside the module body.
leaner module 0x42::documented_module where
  /-- First item-documentation line.
   Second item-documentation line. -/
  fun value() -> u64 := 1

leaner module 0x42::text_literals where
  const GREETING : string := "hello"
  const HEADER : Bytes := b[0, 127, 255]

leaner module 0x42::constant_references where
  const ABORT_CODE : u64 := 7
  fun code() -> u64 := ABORT_CODE
  fun select(value : Bool, left : u64, right : u64) -> u64 :=
    match value with
      | true => left | right
      | false => ABORT_CODE

leaner module 0x42::specification_declarations where
  opaque spec fun choose {T : type} (value : T) : T
  spec fun positive (value : Int) : Bool := core.prim.greater(value, 0)
  spec fun nonnegative (value : Int) : Bool := positive(value)

leaner module 0x42::behavior_summaries where
  spec fun summarized_result(f : Fn(Int) -> Int, value : Int) : Int :=
    @1..@2 |~ result_of<f>(value)
  spec fun summarized_aborts(f : Fn(Int) -> Int, value : Int) : Bool :=
    @1 |~ aborts_of<f>(value)
  spec fun summarized_requires(f : Fn(Int) -> Int, value : Int) : Bool :=
    requires_of<f>(value)
  spec fun summarized_ensures(f : Fn(Int) -> Int, value : Int, result_value : Int) : Bool :=
    ensures_of<f>(value, result_value)
  spec fun summarized_unchanged(f : Fn(Int) -> Int, value : Int) : Bool :=
    unchanged_of<f>(value)
  spec fun summarized_folds(f : Fn(Int) -> Int, values : Vector<Int>, count : Int) : Bool :=
    folds_of<f>(values, count)

leaner module 0x42::behavior_generic_inference where
  public native fun push_back {Element}(
    self : &mut Vector<Element>, value : Element
  ) -> Unit
  fun range_with_step(start : u64, end_ : u64, step : u64) -> Vector<u64> := do
    let values := vector<u64>[]
    while start < end_ do
      values.push_back(start)
      start := start + step
    return values
  spec fun spec_fold {Element} {Acc}(
    f : Fn(Acc, &Element) -> Acc, values : Vector<Element>, init : Acc, end_ : Int
  ) : Acc :=
    if end_ == 0 then
      init
    else
      result_of<f>(spec_fold(f, values, init, end_ - 1), values[end_ - 1])
  spec fun spec_fold_idx {Acc}(
    f : Fn(Acc, u64) -> Acc, init : Acc, end_ : Int
  ) : Acc :=
    if end_ == 0 then
      init
    else
      result_of<f>(spec_fold_idx(f, init, end_ - 1), end_ - 1)
  spec fun spec_map_ref {Element} {NewElement}(
    f : Fn(&Element) -> NewElement, values : Vector<Element>, end_ : Int
  ) : Vector<NewElement> :=
    if end_ == 0 then
      vec::<NewElement>()
    else
      concat(
        spec_map_ref(f, values, end_ - 1),
        vec(result_of<f>(values[end_ - 1])))
  spec fun spec_map_ref_aborts {Element} {NewElement}(
    f : Fn(&Element) -> NewElement, values : Vector<Element>, end_ : Int
  ) : Bool :=
    end_ > 0 &&
      (spec_map_ref_aborts(f, values, end_ - 1) ||
        aborts_of<f>(values[end_ - 1]))

leaner module 0x42::contract_surface where
  opaque spec fun serialize {T : type} (value : T) : Vector<UInt<8> >
  public native fun maybe_size {T : type} () -> examples::option::Option::<UInt<64> >
  public native fun size {T : type} (value : &T) -> UInt<64>
  public native fun apply {T : type} (callback : Fn(T) -> T, value : T) -> T
  spec size where
    aborts_if [abstract] false;
    ensures core.prim.equal(result, core.prim.length(serialize(value)));
    pragma opaque;

leaner module 0x42::nominal_contracts where
  pragma aborts_if_is_strict;
  struct Bounded where
    value : UInt<64>
  spec Bounded where
    invariant core.prim.less(value, 10);
    pragma intrinsic;
  enum State has Copy where
    | ready
    | done
  spec State where
    pragma intrinsic;
  fun bounded (value : UInt<64>) -> Bounded := core.construct Bounded(value)
  fun bounded_value (bounded : Bounded) -> UInt<64> :=
    core.data.select[Bounded, value](bounded)
  fun initial () -> State := core.construct State::ready()
  fun is_ready (state : State) -> Bool := core.data.testVariants[State, ready](state)
  fun is_ready_ref (state : &State) -> Bool :=
    core.data.testVariants[State, ready](core.ref.dereference(state))
  fun observe (value : UInt<64>) -> Unit := ()
  fun sequence (value : UInt<64>) -> UInt<64> := do
    observe(value);
    return value
  fun let_value (value : UInt<64>) -> UInt<64> := do
    let doubled : UInt<64> := core.prim.add(value, value);
    return doubled
  fun same {T : type} (value : T) -> T := value
  fun same_u64 (value : UInt<64>) -> UInt<64> := core.call same::<UInt<64> >(value)
  fun widen (value : UInt<64>) -> UInt<128> := value as UInt<128>
  fun borrow_bounded (value : Bounded) -> &Bounded := core.borrow(immutable, value)
  fun borrow_field (value : &Bounded) -> &UInt<64> :=
    core.borrow(immutable, (*value).value)
  fun borrow_value (value : UInt<64>) -> &Bounded :=
    core.ref.borrow(immutable, core.construct Bounded(value))
  fun freeze_bounded (value : &mut Bounded) -> &Bounded := core.ref.freeze(value)
  fun dereference (value : &UInt<64>) -> UInt<64> := core.ref.dereference(value)
  fun overwrite (target : &mut Bounded, value : Bounded) -> Unit :=
    core.ref.mutate(target, value)
  fun destructure (bounded : Bounded) -> UInt<64> := do
    let Bounded { value := field } : Bounded := bounded;
    return field
  fun element_at {T : type} (values : Vector<T>, index : UInt<64>) -> T :=
    core.prim.index(values, index)
  fun subvector {T : type} (values : Vector<T>, start : UInt<64>, stop : UInt<64>) -> Vector<T> :=
    core.prim.slice(values, start, stop)
  fun destructure_pair (pair : (UInt<64>, Bool)) -> UInt<64> := do
    let (first, _) : (UInt<64>, Bool) := pair;
    return first
  fun singleton_tuple (value : UInt<64>) -> (UInt<64>) := (value,)
  fun assume_true () -> Unit := spec do
    assume true;
  fun capture_state () -> Unit := spec do
    assert true;
    assume spec.saveStateAnchor[7]();
    assume spec.foldsCaptureAnchor[8]();
    invariant true;
  fun invoke_predicate (predicate : Fn(UInt<64>) -> Bool, value : UInt<64>) -> Bool :=
    core.invoke(predicate, value)
  fun count_to (limit : UInt<64>) -> UInt<64> := do
    let mut current : UInt<64> := 0;
    loop if core.prim.less(current, limit) then do
      current := current + 1
    else break;
    spec do
      let bound := limit
      invariant core.prim.lessEqual(current, bound)
    return current
  fun keyword_parameter («end» : UInt<64>) -> UInt<64> := «end»
  fun checked_equal (value : UInt<64>) -> Bool :=
    value + 1 == value
  fun assign_pair (left : UInt<64>, right : Bool, pair : (UInt<64>, Bool)) -> Unit :=
    core.assignPattern[(UInt<64>, Bool)]((left, right), pair)
  spec fun anchored (value : Bool) : Bool :=
    spec.withStateAnchor[9](spec.old(value))

leaner module 0x42::logical_range where
  spec fun interval (start : Int, stop : Int) : Range := core.prim.range(start, stop)
  spec fun contains {T : type} (values : Vector<T>, needle : T) : Bool :=
    ∃ (value in values), value == needle
  spec fun empty {T : type} () : Vector<T> := spec.emptyVector::<T>()
  spec fun singleton {T : type} (value : T) : Vector<T> :=
    spec.singletonVector::<T>(value)
  spec fun update {T : type} (values : Vector<T>, index : Int, value : T) : Vector<T> :=
    spec.updateVector::<T>(values, index, value)
  spec fun concat {T : type} (left : Vector<T>, right : Vector<T>) : Vector<T> :=
    spec.concatVector::<T>(left, right)
  spec fun index_of {T : type} (values : Vector<T>, value : T) : Int :=
    spec.indexOfVector::<T>(values, value)
  spec fun contains_value {T : type} (values : Vector<T>, value : T) : Bool :=
    spec.containsVector::<T>(values, value)
  spec fun size {T : type} (values : Vector<T>) : Int :=
    spec.lengthVector::<T>(values)
  spec fun get {T : type} (values : Vector<T>, index : Int) : T :=
    spec.indexVector::<T>(values, index)
  spec fun slice {T : type} (values : Vector<T>, bounds : Range) : Vector<T> :=
    spec.sliceVector::<T>(values, bounds)
  spec fun logical_int (value : UInt<64>) : Int := spec.bitVectorToInt(value)
  spec fun in_bounds {T : type} (values : Vector<T>, index : Int) : Bool :=
    spec.inVectorRange::<T>(values, index)
  spec fun indices {T : type} (values : Vector<T>) : Range :=
    spec.vectorRange::<T>(values)
  spec fun range_contains (values : Range, index : Int) : Bool := spec.inRange(values, index)

leaner module 0x42::vector_regressions where
  public native fun native_length {T : type} (values : &Vector<T>) -> UInt<64>
  fun call_length {T : type} (values : &Vector<T>) -> UInt<64> :=
    core.call native_length::<T>(values)
  fun pair_zero () -> (Bool, UInt<64>) := core.prim.tuple(false, 0)
  fun intrinsic_early_return (flag : Bool) -> Bool := do
    if flag then return(true);
    return false
  spec intrinsic_early_return where
    pragma intrinsic;
  fun external_identity {T : type} (value : T) -> T :=
    value
  fun generic_or_abort {T : type} (value : T, available : Bool) -> T :=
    if available then value else abort()

leaner module 0x42::statement_order where
  fun update (elem : &mut Bool, mut i : UInt<64>, len : UInt<64>) -> Unit := do
    while i < len do
      do
        let mut target := elem;
        *target := false
      i := i + 1;
  fun discard_result(value : UInt<64>) -> Unit := do
    value + 1
  fun choose(flag : Bool, mut value : UInt<64>) -> UInt<64> :=
    if flag then
      let next := value + 1
      return next
    else
      value := value + 2
      return value

leaner module 0x42::range_loop where
  fun observe (value : UInt<64>) -> Unit := ()
  fun observe_while (active : Bool) -> Unit :=
    while active do
      observe(0)
  fun visit (enable : Vector<UInt<64> >, disable : Vector<UInt<64> >) -> Unit := do
    for i in 0..enable.length do
      observe(enable[i])
    for i in 0..disable.length do
      observe(disable[i])
  fun skip_first(values : Vector<UInt<64> >) -> Unit :=
    for i in 0..values.length do
      if i == 0 then continue
      observe(values[i])

leaner module 0x42::spec_abort where
  fun assert_code (available : Bool) -> Unit := assert(available, 7)
  spec fun value_or_abort (available : Bool) : u64 :=
    if available then 1 else abort(7)

-- A friend module, a quantifier over a whole type's domain, and an inlined
-- call's derivation summary: the namespace-relation and specification surface
-- the Move exchange frontend produces.
leaner module 0x42::module_relations where
  friend 0x42::spec_abort;
  friend playground::companion;
  fun observed (value : u64) -> u64 := do
    spec assume spec.inlineCallSummary(value, false)
    return value
  spec fun total (value : u64) : Bool := ∀ (other : u64), other >= 0

leaner module 0x42::move2_index where
  struct Resource has Store, Key where
    value : u64
    values : Vector<u64>
  fun field (self : &Resource) -> u64 := self.value
  fun borrow_field (self : &Resource) -> &u64 := &self.value
  fun borrow_mut_field (self : &mut Resource) -> &mut u64 := &mut self.value
  fun vector_value (self : &Resource, index : u64) -> u64 := self.values[index]
  fun vector_borrow (self : &Resource, index : u64) -> &u64 := &self.values[index]
  fun vector_borrow_mut (self : &mut Resource, index : u64) -> &mut u64 :=
    &mut self.values[index]
  fun vector_write (self : &mut Resource, index : u64, value : u64) -> Unit :=
    self.values[index] := value
  fun storage_read (address : Address) -> Resource := Resource[address]
  fun storage_borrow (address : Address) -> &Resource := &Resource[address]
  fun storage_borrow_mut (address : Address) -> &mut Resource := &mut Resource[address]
  fun storage_field (address : Address) -> u64 := Resource[address].value
  fun storage_borrow_field (address : Address) -> &u64 := &Resource[address].value
  fun storage_borrow_mut_field (address : Address) -> &mut u64 :=
    &mut Resource[address].value
  fun storage_write_resource (address : Address, value : Resource) -> Unit :=
    Resource[address] := value
  fun storage_write (address : Address, value : u64) -> Unit :=
    Resource[address].value := value

leaner module 0x42::surface_regressions where
  use 0x1::std::mem
  use 0x1::std::vector
  struct Counter where
    value : u64
  struct ConstructorInner where
    first_field : u64
    second_field : u64
  struct ConstructorOuter where
    nested_field : ConstructorInner
  fun replace(value : u64) -> u64 := value
  fun current(self : &Counter) -> u64 := self.value
  fun advance(self : &Counter, amount : u64) -> u64 := self.value + amount
  fun contains(self : &Counter, expected : u64) -> Bool := self.value == expected
  fun calls_local_contains(self : &Counter, expected : u64) -> Bool :=
    self.contains(expected)
  fun read(value : Counter) -> u64 := value.advance(1)
  fun call_read(value : Counter) -> u64 := read(value)
  fun imported_replace(target : &mut u64, value : u64) -> u64 :=
    (mem::replace(target, value) : u64)
  fun has_value(values : &Vector<u64>, value : &u64) -> Bool :=
    (vector::contains(values, value) : Bool)
  fun append_value(values : &mut Vector<u64>, value : u64) -> Unit :=
    (vector::push_back(values, value) : Unit)
  fun take_last(values : &mut Vector<u64>) -> u64 := values.pop_back()
  fun nested_constructor() -> ConstructorOuter :=
    new ConstructorOuter {
      nested_field := new ConstructorInner { first_field := 1, second_field := 2 }
    }
  fun even(value : u64) -> Bool :=
    if value == 0 then true else odd(value - 1)
  fun odd(value : u64) -> Bool :=
    if value == 0 then false else even(value - 1)

set_option maxHeartbeats 4000000 in
elab "#guard_leaner_frontend" : command => do
  let env ← getEnv
  let some math := LeanerLang.registeredUnit? env `«0x42».math
    | throwError "the Move-profile Leaner fixture was not registered"
  let some mathNamespace := math.namespaces[0]?
    | throwError "the Move-profile Leaner fixture has no namespace"
  unless mathNamespace.constants.size == 3 && mathNamespace.functions.size == 2 &&
      mathNamespace.specFunctions.size == 3 && mathNamespace.structs.size == 4 do
    throwError "the Move-profile fixture lost a declaration or derived specification function"
  unless math.tables.types[mathNamespace.constants[1]!.type.typeId.index]? == some .string &&
      math.tables.types[mathNamespace.constants[2]!.type.typeId.index]? == some .bytes &&
      (mathNamespace.expressions[mathNamespace.constants[1]!.value.index]?.map (·.kind)) ==
        some (.value (.string "hello")) &&
      (mathNamespace.expressions[mathNamespace.constants[2]!.value.index]?.map (·.kind)) ==
        some (.value (.bytes #[0, 127, 255])) do
    throwError "String and Bytes constants did not preserve their core LIR types and values"
  let pair := mathNamespace.structs[0]!
  let holder := mathNamespace.structs[1]!
  let genericKinds := mathNamespace.structs[2]!
  let maybe := mathNamespace.structs[3]!
  unless pair.generics.size == 1 && pair.generics[0]!.abilities.size == 2 &&
      pair.fields.size == 2 && pair.variants.isEmpty && pair.abilities.size == 2 &&
      (math.tables.types[pair.fields[0]!.type.typeId.index]? == some (.typeParameter 0)) &&
      (math.tables.types[holder.fields[0]!.type.typeId.index]?).any (fun
        | .nominal _ arguments => arguments.size == 1
        | _ => false) &&
      genericKinds.generics.map (·.kind) == #[.const, .lifetime, .evidence] &&
      genericKinds.generics[0]!.type.bind
        (math.tables.types[·.typeId.index]?) == some (.integer .pointer false) &&
      maybe.fields.isEmpty && maybe.variants.size == 2 && maybe.abilities.size == 2 &&
      maybe.variants[0]!.discriminant == some 0 &&
      maybe.variants[1]!.discriminant == some 1 &&
      maybe.variants[1]!.fields.size == 1 do
    throwError "nominal declarations did not preserve fields, variants, abilities, or discriminants"
  let specFunction := mathNamespace.specFunctions[0]!
  let some parameter := specFunction.signature.parameters[0]?
    | throwError "the derived specification function lost its parameter"
  let some result := specFunction.signature.results[0]?
    | throwError "the derived specification function lost its result"
  unless math.tables.types[parameter.typeUse.typeId.index]? ==
      some (.integer .unbounded true) &&
      math.tables.types[result.typeId.index]? == some (.integer .unbounded true) do
    throwError "derived specification integers were not widened to Int"
  unless mathNamespace.specFunctions.any fun declaration =>
      (math.tables.names[declaration.name.index]?.map (·.name.startsWith "__leaner_arbitrary_")).getD false &&
        declaration.body.isNone do
    throwError "an aborting derived specification path did not become an opaque site value"
  let executableRoot ← match mathNamespace.functions[0]!.body with
    | .structured root => pure root
    | .absent => throwError "the executable arithmetic body was not retained"
  let some executableBody := mathNamespace.expressions[executableRoot.index]?
    | throwError "the executable arithmetic body does not index its expression arena"
  let some specificationBody := specFunction.body.bind
      (mathNamespace.expressions[·.index]?)
    | throwError "the derived specification arithmetic body was not retained"
  unless (match executableBody.kind with
      | .operation (.primitive (.checkedAdd .abort)) _ _ _ => true
      | _ => false) && (match specificationBody.kind with
      | .operation (.primitive .add) _ _ _ => true
      | _ => false) do
    throwError "checked executable arithmetic was not lifted to mathematical spec arithmetic"
  let some rust := LeanerLang.registeredUnit? env `examples.rust_identity
    | throwError "the Rust-profile Leaner fixture was not registered"
  let some rustFunction := rust.namespaces[0]?.bind (·.functions[0]?)
    | throwError "the Rust-profile Leaner fixture lost its function"
  unless rustFunction.signature.generics.size == 1 &&
      rustFunction.contract.conditions.any fun condition =>
      condition.kind == .abortsIf &&
        (rust.namespaces[0]?.bind (·.expressions[condition.expression.index]?)).any
          (fun expression => expression.kind == .value (.bool false)) do
    throwError "Rust functions must receive an implicit `aborts_if false`"
  let some textLiterals := LeanerLang.registeredUnit? env `«0x42».text_literals
    | throwError "the text-literal fixture was not registered"
  let expected := "-- Copyright © Aptos Foundation\n" ++
    "-- SPDX-License-Identifier: Apache-2.0\n\n" ++
    "import LeanerLang\n\n" ++
    "leaner module 0x42::text_literals where\n" ++
    "  const GREETING : string := \"hello\"\n\n" ++
    "  const HEADER : Bytes := b[0, 127, 255]\n"
  match LeanerLang.Print.render env textLiterals with
  | .ok actual => unless actual == expected do
      throwError "String and Bytes did not retain their canonical LeanerLang spelling\nexpected:\n{expected}\nactual:\n{actual}"
  | .error error => throwError "the text-literal fixture did not render: {error}"
  let some constantReferences :=
      LeanerLang.registeredUnit? env `«0x42».constant_references
    | throwError "the constant-reference fixture was not registered"
  match LeanerLang.Print.render env constantReferences with
  | .error error => throwError "constant references did not render: {error}"
  | .ok printed =>
      unless printed.contains "fun code() -> u64 := ABORT_CODE" do
        throwError "a named constant was printed as its folded value:\n{printed}"
      if printed.contains "(match" || printed.contains "=> (" then
        throwError "match expressions or arms gained redundant parentheses:\n{printed}"
      match LeanerLang.Print.formatSource env printed with
      | .error error => throwError "constant references did not re-import: {error}"
      | .ok formatted => unless formatted == printed do
          throwError "constant references are not a canonical fixed point:\n{formatted}"
  let some logicalRange := LeanerLang.registeredUnit? env `«0x42».logical_range
    | throwError "the logical-range fixture was not registered"
  let expectedLogicalRange := "-- Copyright © Aptos Foundation\n" ++
    "-- SPDX-License-Identifier: Apache-2.0\n\n" ++
    "import LeanerLang\n\n" ++
    "leaner module 0x42::logical_range where\n" ++
    "  spec fun interval(start : Int, stop : Int) : Range := start .. stop\n\n" ++
    "  spec fun contains {T}(values : Vector<T>, needle : T) : Bool :=\n" ++
    "    ∃ (value in values), value == needle\n\n" ++
    "  spec fun empty {T}() : Vector<T> := vec::<T>()\n\n" ++
    "  spec fun singleton {T}(value : T) : Vector<T> := vec(value)\n\n" ++
    "  spec fun update {T}(values : Vector<T>, index : Int, value : T) : Vector<T> :=\n" ++
    "    update(values, index, value)\n\n" ++
    "  spec fun concat {T}(left : Vector<T>, right : Vector<T>) : Vector<T> :=\n" ++
    "    concat(left, right)\n\n" ++
    "  spec fun index_of {T}(values : Vector<T>, value : T) : Int :=\n" ++
    "    index_of(values, value)\n\n" ++
    "  spec fun contains_value {T}(values : Vector<T>, value : T) : Bool :=\n" ++
    "    value ∈ values\n\n" ++
    "  spec fun size {T}(values : Vector<T>) : Int := values.length\n\n" ++
    "  spec fun get {T}(values : Vector<T>, index : Int) : T := values[index]\n\n" ++
    "  spec fun slice {T}(values : Vector<T>, bounds : Range) : Vector<T> :=\n" ++
    "    values[bounds]\n\n" ++
    "  spec fun logical_int(value : u64) : Int := value\n\n" ++
    "  spec fun in_bounds {T}(values : Vector<T>, index : Int) : Bool :=\n" ++
    "    in_range(values, index)\n\n" ++
    "  spec fun indices {T}(values : Vector<T>) : Range := range(values)\n\n" ++
    "  spec fun range_contains(values : Range, index : Int) : Bool :=\n" ++
    "    in_range(values, index)\n"
  match LeanerLang.Print.render env logicalRange with
  | .ok actual => unless actual == expectedLogicalRange do
      throwError "logical ranges changed their canonical spelling\nexpected:\n{expectedLogicalRange}\nactual:\n{actual}"
  | .error error => throwError "the logical-range fixture did not render: {error}"
  let some behaviorSummaries := LeanerLang.registeredUnit? env `«0x42».behavior_summaries
    | throwError "the behavior-summary fixture was not registered"
  match LeanerLang.Print.render env behaviorSummaries with
  | .error error => throwError "behavior summaries did not render: {error}"
  | .ok printed =>
      unless printed.contains "@1..@2 |~ result_of<f>(value)" &&
          printed.contains "@1 |~ aborts_of<f>(value)" &&
          printed.contains "requires_of<f>(value)" &&
          printed.contains "ensures_of<f>(value, result_value)" &&
          printed.contains "unchanged_of<f>(value)" &&
          printed.contains "folds_of<f>(values, count)" do
        throwError "behavior summaries lost their canonical source forms:\n{printed}"
      match LeanerLang.Print.formatSource env printed with
      | .error error => throwError "behavior summaries did not re-import: {error}"
      | .ok formatted => unless formatted == printed do
          throwError "behavior summaries are not a canonical fixed point:\n{formatted}"
  let some behaviorGenericInference :=
      LeanerLang.registeredUnit? env `«0x42».behavior_generic_inference
    | throwError "the generic behavior-inference fixture was not registered"
  match LeanerLang.Print.render env behaviorGenericInference with
  | .error error => throwError "generic behavior inference did not render: {error}"
  | .ok printed =>
      match LeanerLang.Print.formatSource env printed with
      | .error error => throwError "generic behavior inference did not re-import: {error}"
      | .ok formatted => unless formatted == printed do
          throwError "generic behavior inference is not a canonical fixed point:\n{formatted}"
  let some vectorRegressions := LeanerLang.registeredUnit? env `«0x42».vector_regressions
    | throwError "the vector-regression fixture was not registered"
  let some vectorRegressionNamespace := vectorRegressions.namespaces[0]?
    | throwError "the vector-regression fixture has no namespace"
  let intrinsicBodies := vectorRegressionNamespace.specFunctions.filter fun declaration =>
    (vectorRegressions.tables.names[declaration.name.index]?.map (·.name)).getD "" ==
      "intrinsic_early_return"
  let genericArbitrary := vectorRegressionNamespace.specFunctions.find? fun declaration =>
    (vectorRegressions.tables.names[declaration.name.index]?.map
      (·.name.startsWith "__leaner_arbitrary_")).getD false
  unless vectorRegressionNamespace.functions.size == 6 &&
      vectorRegressionNamespace.specFunctions.size == 6 &&
      intrinsicBodies.size == 1 && intrinsicBodies[0]!.body.isNone &&
      genericArbitrary.any (·.signature.generics.size == 1) do
    throwError "vector frontend regressions lost reference calls, tuple inference, intrinsic returns, or generic abort paths"
  let some specifications := LeanerLang.registeredUnit? env `«0x42».specification_declarations
    | throwError "the specification-declaration fixture was not registered"
  let some specificationNamespace := specifications.namespaces[0]?
    | throwError "the specification-declaration fixture has no namespace"
  unless specificationNamespace.specFunctions.size == 3 &&
      specificationNamespace.specFunctions[0]!.body.isNone &&
      specificationNamespace.specFunctions[1]!.body.isSome &&
      specificationNamespace.specFunctions[2]!.body.isSome do
    throwError "opaque and defined specification functions did not retain their body distinction"
  let expectedSpecifications := "-- Copyright © Aptos Foundation\n" ++
    "-- SPDX-License-Identifier: Apache-2.0\n\n" ++
    "import LeanerLang\n\n" ++
    "leaner module 0x42::specification_declarations where\n" ++
    "  opaque spec fun choose {T}(value : T) : T\n\n" ++
    "  spec fun positive(value : Int) : Bool := value > 0\n\n" ++
    "  spec fun nonnegative(value : Int) : Bool := positive(value)\n"
  match LeanerLang.Print.render env specifications with
  | .ok actual => unless actual == expectedSpecifications do
      throwError "specification declarations changed their canonical spelling\nexpected:\n{expectedSpecifications}\nactual:\n{actual}"
  | .error error => throwError "the specification-declaration fixture did not render: {error}"
  let some contractSurface := LeanerLang.registeredUnit? env `«0x42».contract_surface
    | throwError "the contract-surface fixture was not registered"
  let some contractNamespace := contractSurface.namespaces[0]?
    | throwError "the contract-surface fixture has no namespace"
  let some sizeParameter := contractNamespace.functions[1]?.bind (·.signature.parameters[0]?)
    | throwError "the contract-surface fixture lost its reference parameter"
  unless contractNamespace.functions.size == 3 && contractNamespace.specFunctions.size == 1 &&
      (contractSurface.tables.types[sizeParameter.typeUse.typeId.index]?).any (fun
        | .reference reference => reference.profile == .move
        | _ => false) &&
      contractSurface.tables.namespaces.any
        (·.segments == #["examples", "option"]) do
    throwError "references, external nominals, or specification declarations were not retained"
  let expectedContractSurface := "-- Copyright © Aptos Foundation\n" ++
    "-- SPDX-License-Identifier: Apache-2.0\n\n" ++
    "import LeanerLang\n\n" ++
    "leaner module 0x42::contract_surface where\n" ++
    "  use examples::option::Option\n\n" ++
    "  opaque spec fun serialize {T}(value : T) : Vector<u8>\n\n" ++
    "  public native fun maybe_size {T}() -> Option<u64>\n\n" ++
    "  public native fun size {T}(value : &T) -> u64\n\n" ++
    "  spec size where\n" ++
    "    pragma opaque\n" ++
    "    aborts_if [abstract] false\n" ++
    "    ensures result == serialize(value).length\n\n" ++
    "  public native fun apply {T}(callback : Fn(T) -> T, value : T) -> T\n"
  match LeanerLang.Print.render env contractSurface with
  | .ok actual => unless actual == expectedContractSurface do
      throwError "the contract surface changed its canonical spelling\nexpected:\n{expectedContractSurface}\nactual:\n{actual}"
  | .error error => throwError "the contract-surface fixture did not render: {error}"
  let some nominalContracts := LeanerLang.registeredUnit? env `«0x42».nominal_contracts
    | throwError "the nominal-contract fixture was not registered"
  let some nominalNamespace := nominalContracts.namespaces[0]?
    | throwError "the nominal-contract fixture has no namespace"
  let some bounded := nominalNamespace.structs[0]?
    | throwError "the nominal-contract fixture lost its structure"
  let some state := nominalNamespace.structs[1]?
    | throwError "the nominal-contract fixture lost its enum"
  unless nominalNamespace.pragmas.size == 1 && bounded.locals.size == 1 &&
      bounded.contract.conditions.size == 1 &&
      bounded.contract.conditions[0]!.kind == .structInvariant &&
      bounded.contract.pragmas.size == 1 && state.variants.size == 2 &&
      nominalNamespace.functions.size == 29 && nominalNamespace.specFunctions.size == 30 &&
      state.properties.any fun property =>
        property.profile == .move && property.tag == "struct.variants" &&
          property.payload.isEmpty do
    throwError "nominal contracts, field locals, or Move enum metadata were not retained"
  let expectedNominalContracts := "-- Copyright © Aptos Foundation\n" ++
    "-- SPDX-License-Identifier: Apache-2.0\n\n" ++
    "import LeanerLang\n\n" ++
    "leaner module 0x42::nominal_contracts where\n" ++
    "  pragma aborts_if_is_strict\n\n" ++
    "  struct Bounded where\n" ++
    "    value : u64\n\n" ++
    "  spec Bounded where\n" ++
    "    pragma intrinsic\n" ++
    "    invariant value < 10\n\n" ++
    "  enum State has Copy where\n" ++
    "    | ready\n" ++
    "    | done\n\n" ++
    "  spec State where\n" ++
    "    pragma intrinsic\n\n" ++
    "  fun bounded(value : u64) -> Bounded := new Bounded { value }\n\n" ++
    "  fun bounded_value(bounded : Bounded) -> u64 := bounded.value\n\n" ++
    "  fun initial() -> State := new State::ready {}\n\n" ++
    "  fun is_ready(state : State) -> Bool := state is ready\n\n" ++
    "  fun is_ready_ref(state : &State) -> Bool := state is ready\n\n" ++
    "  fun observe(value : u64) -> Unit := ()\n\n" ++
    "  fun sequence(value : u64) -> u64 := do\n" ++
    "    observe(value)\n" ++
    "    return value\n\n" ++
    "  fun let_value(value : u64) -> u64 := do\n" ++
    "    let doubled := value + value\n" ++
    "    return doubled\n\n" ++
    "  fun same {T}(value : T) -> T := value\n\n" ++
    "  fun same_u64(value : u64) -> u64 := same(value)\n\n" ++
    "  fun widen(value : u64) -> u128 := value as u128\n\n" ++
    "  fun borrow_bounded(value : Bounded) -> &Bounded := &value\n\n" ++
    "  fun borrow_field(value : &Bounded) -> &u64 := &value.value\n\n" ++
    "  fun borrow_value(value : u64) -> &Bounded := &new Bounded { value }\n\n" ++
    "  fun freeze_bounded(value : &mut Bounded) -> &Bounded := value\n\n" ++
    "  fun dereference(value : &u64) -> u64 := *value\n\n" ++
    "  fun overwrite(target : &mut Bounded, value : Bounded) -> Unit :=\n" ++
    "    *target := value\n\n" ++
    "  fun destructure(bounded : Bounded) -> u64 := do\n" ++
    "    let Bounded { value := field } := bounded\n" ++
    "    return field\n\n" ++
    "  fun element_at {T}(values : Vector<T>, index : u64) -> T := values[index]\n\n" ++
    "  fun subvector {T}(values : Vector<T>, start : u64, stop : u64) -> Vector<T> :=\n" ++
    "    slice(values, start, stop)\n\n" ++
    "  fun destructure_pair(pair : (u64, Bool)) -> u64 := do\n" ++
    "    let (first, _) := pair\n" ++
    "    return first\n\n" ++
    "  fun singleton_tuple(value : u64) -> (u64) := (value,)\n\n" ++
    "  fun assume_true() -> Unit := spec assume true\n\n" ++
    "  fun capture_state() -> Unit := spec do\n" ++
    "    assert true\n" ++
    "    assume save_state_anchor!(7)\n" ++
    "    assume folds_capture_anchor!(8)\n" ++
    "    invariant true\n\n" ++
    "  fun invoke_predicate(predicate : Fn(u64) -> Bool, value : u64) -> Bool :=\n" ++
    "    invoke(predicate, value)\n\n" ++
    "  fun count_to(limit : u64) -> u64 := do\n" ++
    "    let mut current := 0\n" ++
    "    while current < limit do\n" ++
    "      current := current + 1\n" ++
    "    where\n" ++
    "      let bound := limit\n" ++
    "      invariant current <= bound\n" ++
    "    return current\n\n" ++
    "  fun keyword_parameter(end : u64) -> u64 := end\n\n" ++
    "  fun checked_equal(value : u64) -> Bool := value + 1 == value\n\n" ++
    "  fun assign_pair(left : u64, right : Bool, pair : (u64, Bool)) -> Unit :=\n" ++
    "    assign_pattern[(u64, Bool)]((left, right), pair)\n\n" ++
    "  spec fun anchored(value : Bool) : Bool := with_state_anchor!(9, old(value))\n"
  match LeanerLang.Print.render env nominalContracts with
  | .ok actual => unless actual == expectedNominalContracts do
      throwError "the nominal-contract surface changed its canonical spelling\nexpected:\n{expectedNominalContracts}\nactual:\n{actual}"
  | .error error => throwError "the nominal-contract fixture did not render: {error}"
  let some statementOrder := LeanerLang.registeredUnit? env `«0x42».statement_order
    | throwError "the statement-order fixture was not registered"
  match LeanerLang.Print.render env statementOrder with
  | .error error => throwError "the statement-order fixture did not render: {error}"
  | .ok printed =>
      unless printed.contains
          "while i < len do\n      let mut target := elem\n      *target := false\n      i := i + 1" &&
          printed.contains
            "if flag then\n      let next := value + 1\n      return next\n    else\n      value := value + 2\n      return value" &&
          !printed.contains "then do" && !printed.contains "else do" &&
          !printed.contains "while i < len do\n      do" &&
          printed.contains "fun discard_result(value : u64) -> Unit := do\n    value + 1" &&
          !printed.contains "return ()" do
        throwError "statement-position loop blocks were not flattened:\n{printed}"
      match LeanerLang.Print.formatSource env printed with
      | .error error => throwError "the statement-order fixture did not re-import: {error}"
      | .ok formatted => unless formatted == printed do
          throwError "nested block sequencing is not a canonical fixed point\nprinted:\n{printed}\nformatted:\n{formatted}"
  let some rangeLoop := LeanerLang.registeredUnit? env `«0x42».range_loop
    | throwError "the range-loop fixture was not registered"
  match LeanerLang.Print.render env rangeLoop with
  | .error error => throwError "the range-loop fixture did not render: {error}"
  | .ok printed =>
      unless printed.contains "for i in 0..enable.length do" &&
          printed.contains "for i in 0..disable.length do" &&
          printed.contains "for i in 0..values.length do" &&
          printed.contains "while active do\n      observe(0)" &&
          !printed.contains "observe(0)\n      return ()" &&
          printed.contains "if i == 0 then continue" &&
          !printed.contains "$for" && !printed.contains "_for_" do
        throwError "range loops leaked their core lowering:\n{printed}"
      match LeanerLang.Print.formatSource env printed with
      | .error error => throwError "the range-loop fixture did not re-import: {error}"
      | .ok formatted => unless formatted == printed do
          throwError "range loops are not a canonical fixed point\nprinted:\n{printed}\nformatted:\n{formatted}"
  let some commentElaboration := LeanerLang.registeredUnit? env `examples.comment_elaboration
    | throwError "the ordinary comment-elaboration fixture was not registered"
  let sourceComments := commentElaboration.namespaces[0]!.comments
  unless sourceComments.size == 2 &&
      sourceComments[0]!.text == "-- retained by ordinary command elaboration" &&
      sourceComments[1]!.text.contains "/- inner -/" do
    throwError "ordinary Lean file elaboration did not retain comments: {repr sourceComments}"
  let some rustImplicitCopy := LeanerLang.registeredUnit? env `examples.rust_implicit_copy
    | throwError "the Rust implicit-copy fixture was not registered"
  match LeanerLang.Print.render env rustImplicitCopy with
  | .error error => throwError "the Rust implicit-copy fixture did not render: {error}"
  | .ok printed =>
      unless printed.contains "tuple_first(pair : (u32, Bool)) -> u32 := pair[0u32]" &&
          printed.contains "vector_first(values : &Vector<u32>) -> u32 := values[0usize]" &&
          !printed.contains "core.read(pair[0u32])" &&
          !printed.contains "*values[0usize]" do
        throwError "Rust scalar reads lost their implicit-copy spelling:\n{printed}"
      match LeanerLang.Print.formatSource env printed with
      | .error error => throwError "the Rust implicit-copy fixture did not re-import: {error}"
      | .ok formatted => unless formatted == printed do
          throwError "Rust implicit-copy source is not a canonical fixed point:\n{formatted}"
  let some specAbort := LeanerLang.registeredUnit? env `«0x42».spec_abort
    | throwError "the specification-abort fixture was not registered"
  match LeanerLang.Print.render env specAbort with
  | .error error => throwError "the specification-abort fixture did not render: {error}"
  | .ok printed =>
      unless printed.contains "fun assert_code(available : Bool) -> Unit :=\n    assert!(available, 7)" do
        throwError "runtime assertions lost their compact canonical spelling:\n{printed}"
      match LeanerLang.Print.formatSource env printed with
      | .error error => throwError "the specification-abort fixture did not re-import: {error}"
      | .ok formatted => unless formatted == printed do
          throwError "specification aborts are not a canonical fixed point\nprinted:\n{printed}\nformatted:\n{formatted}"
  let some moduleRelations := LeanerLang.registeredUnit? env `«0x42».module_relations
    | throwError "the module-relation fixture was not registered"
  match LeanerLang.Print.render env moduleRelations with
  | .error error => throwError "the module-relation fixture did not render: {error}"
  | .ok printed =>
      unless printed.contains "friend 0x42::spec_abort;" &&
          printed.contains "friend playground::companion;" &&
          printed.contains "spec.inlineCallSummary(value, false)" &&
          printed.contains "∀ (other : u64)" do
        throwError "module relations lost their canonical spelling:\n{printed}"
      match LeanerLang.Print.formatSource env printed with
      | .error error => throwError "the module-relation fixture did not re-import: {error}"
      | .ok formatted => unless formatted == printed do
          throwError "module relations are not a canonical fixed point\nprinted:\n{printed}\nformatted:\n{formatted}"
  let some move2Index := LeanerLang.registeredUnit? env `«0x42».move2_index
    | throwError "the Move 2 index-syntax fixture was not registered"
  match LeanerLang.Print.render env move2Index with
  | .error error => throwError "the Move 2 index-syntax fixture did not render: {error}"
  | .ok printed =>
      unless printed.contains "self.values[index]" &&
          printed.contains "&mut self.values[index]" &&
          printed.contains "self.values[index] := value" &&
          printed.contains "&Resource[address]" &&
          printed.contains "&mut Resource[address].value" &&
          printed.contains "Resource[address] := value" &&
          printed.contains "Resource[address].value" &&
          !printed.contains "(*self)" do
        throwError "Move 2 projections and indexing lost their canonical spelling:\n{printed}"
      match LeanerLang.Print.formatSource env printed with
      | .error error => throwError "the Move 2 index-syntax fixture did not re-import: {error}"
      | .ok formatted => unless formatted == printed do
          throwError "Move 2 index syntax is not a canonical fixed point\nprinted:\n{printed}\nformatted:\n{formatted}"
  let some surfaceRegressions := LeanerLang.registeredUnit? env `«0x42».surface_regressions
    | throwError "the surface-regression fixture was not registered"
  match LeanerLang.Print.render env surfaceRegressions with
  | .error error => throwError "the surface-regression fixture did not render: {error}"
  | .ok printed =>
      let declarationOrder := match printed.find? "fun even", printed.find? "fun odd" with
        | some evenPosition, some oddPosition => decide (evenPosition < oddPosition)
        | _, _ => false
      unless printed.contains "use 0x1::std::mem" &&
          printed.contains "(mem::replace(target, value) : u64)" &&
          printed.contains "value.advance(1)" &&
          printed.contains "fun read(value : Counter)" &&
          printed.contains "read(value)" &&
          !printed.contains "«read»" &&
          printed.contains "self.contains(expected)" &&
          printed.contains "values.contains(value)" &&
          printed.contains "values.push_back(value)" &&
          printed.contains "values.pop_back()" &&
          printed.contains "new ConstructorOuter {\n      nested_field := new ConstructorInner {\n        first_field := 1, second_field := 2\n      }\n    }" &&
          !printed.contains "(vector::contains" &&
          !printed.contains "(vector::push_back" &&
          declarationOrder &&
          !printed.contains "0x1::std::mem::replace" do
        throwError "imports, receivers, or mutually recursive source order regressed:\n{printed}"
      match LeanerLang.Print.formatSource env printed with
      | .error error => throwError "the surface-regression fixture did not re-import: {error}"
      | .ok formatted => unless formatted == printed do
          throwError "surface regressions are not a canonical fixed point\nprinted:\n{printed}\nformatted:\n{formatted}"
  let commentSource := "-- Copyright © Aptos Foundation\n" ++
    "-- SPDX-License-Identifier: Apache-2.0\n\n" ++
    "import LeanerLang\n\n" ++
    "leaner module 0x42::comments where\n" ++
    "  -- first declaration\n" ++
    "  fun first() -> Unit := () -- trailing declaration\n\n" ++
    "  /- preserved block\n" ++
    "     comment -/\n" ++
    "  -- second declaration\n" ++
    "  -- continued documentation\n" ++
    "  fun second() -> Unit := ()\n"
  match LeanerLang.Print.formatSource env commentSource with
  | .error error => throwError "comments did not survive the LIR round trip: {error}"
  | .ok formatted =>
      unless formatted.contains "-- first declaration\n  fun first" &&
          formatted.contains "-- trailing declaration" &&
          formatted.contains "-- preserved block" && formatted.contains "-- comment" &&
          formatted.contains "-- second declaration\n  -- continued documentation\n  fun second" do
        throwError "comments lost their declaration attachment or multiplicity:\n{formatted}"
      match LeanerLang.Print.formatSource env formatted with
      | .error error => throwError "formatted comments did not re-import: {error}"
      | .ok formattedAgain => unless formattedAgain == formatted do
          throwError "comment formatting is not a canonical fixed point:\n{formattedAgain}"
  let some documentedModule := LeanerLang.registeredUnit? env `«0x42».documented_module
    | throwError "the documented-module fixture was not registered"
  match LeanerLang.Print.render env documentedModule with
  | .error error => throwError "the documented-module fixture did not render: {error}"
  | .ok printed =>
      let documentation := "/-!\nDocumentation owned by the module declaration.\n" ++
        "It must remain outside the module body.\n-/\n"
      let header := "leaner module 0x42::documented_module where"
      unless printed.contains (documentation ++ header) &&
          printed.contains "/--\n  First item-documentation line.\n  Second item-documentation line.\n  -/\n  fun value" &&
          !printed.contains (header ++ "\n  " ++ documentation.trimAscii.toString) do
        throwError "module documentation was not placed before its header:\n{printed}"
      match LeanerLang.Print.formatSource env printed with
      | .error error => throwError "module documentation did not re-import: {error}"
      | .ok formatted => unless formatted == printed do
          throwError "module documentation is not a canonical fixed point:\n{formatted}"

#guard_leaner_frontend
#check_leaner 0x42::math
