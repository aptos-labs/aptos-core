-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-! The `aggregator_v2` natives are read by the Move Prover's prelude: at `u64`
and `u128` exact arithmetic over the fields, at another concrete type an abort,
and at a type parameter an abort unless its `type_name` is one of those two,
the result and the referent unconstrained. -/

verex module 0x1::string where
  struct String has Copy, Drop, Store where
    bytes : Vector<u8>

verex module 0x1::type_info where
  use 0x1::string::String

  public native fun type_name {T}() -> String

verex module 0x1::aggregator_v2 where
  struct Aggregator {IntElement} has Store, Drop where
    value : IntElement
    max_value : IntElement

  public native fun create_aggregator {IntElement}(max_value : IntElement) ->
    Aggregator<IntElement>

  public native fun create_unbounded_aggregator {IntElement}() -> Aggregator<IntElement>

  public native fun try_add {IntElement}(
    self : &mut Aggregator<IntElement>, value : IntElement
  ) -> Bool

  public native fun try_sub {IntElement}(
    self : &mut Aggregator<IntElement>, value : IntElement
  ) -> Bool

  public native fun is_at_least_impl {IntElement}(
    self : &Aggregator<IntElement>, min_amount : IntElement
  ) -> Bool

  public native fun read {IntElement}(self : &Aggregator<IntElement>) -> IntElement

  opaque spec fun spec_get_value {IntElement}(aggregator : Aggregator<IntElement>) : IntElement

  opaque spec fun spec_get_max_value {IntElement}(aggregator : Aggregator<IntElement>) : IntElement

verex module 0x4a::uses where
  use 0x1::aggregator_v2::Aggregator
  use 0x1::aggregator_v2::create_aggregator
  use 0x1::aggregator_v2::create_unbounded_aggregator
  use 0x1::aggregator_v2::is_at_least_impl
  use 0x1::aggregator_v2::read
  use 0x1::aggregator_v2::spec_get_max_value
  use 0x1::aggregator_v2::spec_get_value
  use 0x1::aggregator_v2::try_add
  use 0x1::aggregator_v2::try_sub
  use 0x1::type_info::type_name

  struct Holder has Key where
    balance : Aggregator<u64>

  fun remove(addr : Address) -> Unit := do
    let Holder { balance := balance } := move_from<Holder>(addr)
    assert!(read(&balance) == 0, 1)

  spec remove where
    aborts_if !exists<Holder>(addr)
    aborts_if spec_get_value(global<Holder>(addr).balance) != 0
    ensures !exists<Holder>(addr)
    modifies *

  fun arithmetic() -> Aggregator<u64> := do
    let mut agg := create_aggregator::<u64>(10)
    spec assert spec_get_max_value(agg) == 10
    spec assert spec_get_value(agg) == 0
    let added := try_add(&mut agg, 5)
    spec assert added
    let refused := try_sub(&mut agg, 6)
    spec assert !refused
    spec assert spec_get_value(agg) == 5
    let taken := try_sub(&mut agg, 4)
    spec assert taken
    spec assert spec_get_value(agg) == 1
    spec assert spec_get_max_value(agg) == 10
    let overflown := try_add(&mut agg, 10)
    spec assert !overflown
    spec assert spec_get_value(agg) == 1
    agg

  spec arithmetic where
    aborts_if false
    ensures spec_get_value(result) == 1

  fun unbounded() -> Aggregator<u128> := create_unbounded_aggregator::<u128>()

  spec unbounded where
    aborts_if false
    ensures spec_get_value(result) == 0
    ensures spec_get_max_value(result) == MAX_U128

  fun read_value(aggregator : &Aggregator<u64>) -> u64 := read(aggregator)

  spec read_value where
    aborts_if false
    ensures result == spec_get_value(aggregator)

  fun at_least(aggregator : &Aggregator<u128>, amount : u128) -> Bool :=
    is_at_least_impl(aggregator, amount)

  spec at_least where
    aborts_if false
    ensures result == (spec_get_value(aggregator) >= amount)

  -- Called from a generic function, at a concrete element.
  fun concrete_in_generic {T}(aggregator : &mut Aggregator<u128>, amount : u128, unused : &T) ->
      Bool :=
    try_add(aggregator, amount)

  spec concrete_in_generic where
    aborts_if false
    ensures result <==> old(spec_get_value(aggregator)) + amount <= spec_get_max_value(aggregator)
    ensures result ==> spec_get_value(aggregator) == old(spec_get_value(aggregator)) + amount

  fun invalid_read(aggregator : &Aggregator<u8>) -> u8 := read(aggregator)

  spec invalid_read where
    aborts_if true

  fun invalid_create() -> Aggregator<u8> := create_unbounded_aggregator::<u8>()

  spec invalid_create where
    aborts_if true

  fun generic {T}() -> Aggregator<T> := create_unbounded_aggregator::<T>()

  spec generic where
    aborts_if type_name::<T>().bytes != b"u64" && type_name::<T>().bytes != b"u128"

  fun generic_try {T has Copy, Drop}(aggregator : &mut Aggregator<T>, value : T) -> Bool := do
    let _added := try_add(aggregator, value)
    is_at_least_impl(aggregator, value)

  spec generic_try where
    aborts_if type_name::<T>().bytes != b"u64" && type_name::<T>().bytes != b"u128"

  -- Expected to fail: each states what the prelude does not.
  fun wrong_sum() -> Aggregator<u64> := do
    let mut agg := create_aggregator::<u64>(10)
    let _added := try_add(&mut agg, 5)
    agg

  spec wrong_sum where
    ensures spec_get_value(result) == 6 -- error: the sum is 5

  fun wrong_abort() -> Aggregator<u64> := create_aggregator::<u64>(10)

  spec wrong_abort where
    aborts_if true -- error: a u64 aggregator is created without abort

  fun unconstrained {T}(aggregator : &mut Aggregator<T>, value : T) -> Bool :=
    try_add(aggregator, value)

  spec unconstrained where
    pragma aborts_if_is_partial
    ensures result -- error: at a type parameter the result is unconstrained

  -- `add`'s Move body at a type parameter: its assertion on the unconstrained
  -- result can abort, as it does at `u64` on overflow.
  fun generic_add {T has Copy, Drop}(aggregator : &mut Aggregator<T>, value : T) -> Unit :=
    assert!(try_add(aggregator, value), 1)

  spec generic_add where
    aborts_if type_name::<T>().bytes != b"u64" && type_name::<T>().bytes != b"u128" -- error: try_add can refuse
