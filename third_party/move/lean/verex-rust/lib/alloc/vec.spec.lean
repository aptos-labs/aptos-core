-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

mod vec where
  @[model(vector T)] opaque struct Vec {T} {A} where

  -- `Clone` is the identity at a type without heap state.
  spec from_elem at {Bool} where
    ensures result.length == _2
    ensures ∀ (k in 0 .. _2), result[k] == _1

  spec from_elem at {u8} where
    ensures result.length == _2
    ensures ∀ (k in 0 .. _2), result[k] == _1

  spec from_elem at {u16} where
    ensures result.length == _2
    ensures ∀ (k in 0 .. _2), result[k] == _1

  spec from_elem at {u32} where
    ensures result.length == _2
    ensures ∀ (k in 0 .. _2), result[k] == _1

  spec from_elem at {u64} where
    ensures result.length == _2
    ensures ∀ (k in 0 .. _2), result[k] == _1

  spec from_elem at {u128} where
    ensures result.length == _2
    ensures ∀ (k in 0 .. _2), result[k] == _1

  spec from_elem at {usize} where
    ensures result.length == _2
    ensures ∀ (k in 0 .. _2), result[k] == _1

  spec from_elem at {i8} where
    ensures result.length == _2
    ensures ∀ (k in 0 .. _2), result[k] == _1

  spec from_elem at {i16} where
    ensures result.length == _2
    ensures ∀ (k in 0 .. _2), result[k] == _1

  spec from_elem at {i32} where
    ensures result.length == _2
    ensures ∀ (k in 0 .. _2), result[k] == _1

  spec from_elem at {i64} where
    ensures result.length == _2
    ensures ∀ (k in 0 .. _2), result[k] == _1

  spec from_elem at {i128} where
    ensures result.length == _2
    ensures ∀ (k in 0 .. _2), result[k] == _1

  spec from_elem at {isize} where
    ensures result.length == _2
    ensures ∀ (k in 0 .. _2), result[k] == _1

  spec from_elem at {Char} where
    ensures result.length == _2
    ensures ∀ (k in 0 .. _2), result[k] == _1

  impl Vec where
    spec new where
      ensures result == vec::<T>()

    spec with_capacity where
      ensures result == vec::<T>()

    spec swap_remove where
      panics_if _2 >= old(_1).length
      ensures result == old(_1)[_2]
      ensures _1.length == old(_1).length - 1
      ensures ∀ (k in 0 .. _1.length), k != _2 ==> _1[k] == old(_1)[k]
      ensures _2 < _1.length ==> _1[_2] == old(_1)[old(_1).length - 1]

    -- Capacity and allocation are outside the model: a push that returns had room.
    spec push where
      ensures old(_1).length < MAX_U64
      ensures _1 == core.prim.pushVector(old(_1), _2)

    spec pop where
      ensures old(_1).length == 0 ==> result is None
      ensures old(_1).length > 0 ==> result is Some && result.0 == old(_1)[old(_1).length - 1]
      ensures old(_1).length > 0 ==> _1 == old(_1)[0 .. old(_1).length - 1]
      ensures old(_1).length == 0 ==> _1 == old(_1)

    -- Capacity and allocation are outside the model, as for `push`.
    spec insert where
      panics_if _2 > old(_1).length
      ensures old(_1).length < MAX_U64
      ensures _1.length == old(_1).length + 1
      ensures ∀ (k in 0 .. _2), _1[k] == old(_1)[k]
      ensures _1[_2] == _3
      ensures ∀ (k in _2 .. old(_1).length), _1[k + 1] == old(_1)[k]

    spec remove where
      panics_if _2 >= old(_1).length
      ensures result == old(_1)[_2]
      ensures _1.length == old(_1).length - 1
      ensures ∀ (k in 0 .. _2), _1[k] == old(_1)[k]
      ensures ∀ (k in _2 .. _1.length), _1[k] == old(_1)[k + 1]

    spec truncate where
      ensures _2 >= old(_1).length ==> _1 == old(_1)
      ensures _2 < old(_1).length ==> _1 == old(_1)[0 .. _2]

    spec clear where
      ensures _1 == vec::<T>()

    spec as_slice where
      ensures result == _1

    spec extend_from_slice at {u8} {A} where
      ensures old(_1).length + _2.length <= MAX_U64
      ensures _1.length == old(_1).length + _2.length
      ensures ∀ (k in 0 .. old(_1).length), _1[k] == old(_1)[k]
      ensures ∀ (k in 0 .. _2.length), _1[old(_1).length + k] == _2[k]

    spec extend_from_slice at {u16} {A} where
      ensures old(_1).length + _2.length <= MAX_U64
      ensures _1.length == old(_1).length + _2.length
      ensures ∀ (k in 0 .. old(_1).length), _1[k] == old(_1)[k]
      ensures ∀ (k in 0 .. _2.length), _1[old(_1).length + k] == _2[k]

    spec extend_from_slice at {u32} {A} where
      ensures old(_1).length + _2.length <= MAX_U64
      ensures _1.length == old(_1).length + _2.length
      ensures ∀ (k in 0 .. old(_1).length), _1[k] == old(_1)[k]
      ensures ∀ (k in 0 .. _2.length), _1[old(_1).length + k] == _2[k]

    spec extend_from_slice at {u64} {A} where
      ensures old(_1).length + _2.length <= MAX_U64
      ensures _1.length == old(_1).length + _2.length
      ensures ∀ (k in 0 .. old(_1).length), _1[k] == old(_1)[k]
      ensures ∀ (k in 0 .. _2.length), _1[old(_1).length + k] == _2[k]

    spec extend_from_slice at {u128} {A} where
      ensures old(_1).length + _2.length <= MAX_U64
      ensures _1.length == old(_1).length + _2.length
      ensures ∀ (k in 0 .. old(_1).length), _1[k] == old(_1)[k]
      ensures ∀ (k in 0 .. _2.length), _1[old(_1).length + k] == _2[k]

    spec extend_from_slice at {usize} {A} where
      ensures old(_1).length + _2.length <= MAX_U64
      ensures _1.length == old(_1).length + _2.length
      ensures ∀ (k in 0 .. old(_1).length), _1[k] == old(_1)[k]
      ensures ∀ (k in 0 .. _2.length), _1[old(_1).length + k] == _2[k]

    spec extend_from_slice at {i8} {A} where
      ensures old(_1).length + _2.length <= MAX_U64
      ensures _1.length == old(_1).length + _2.length
      ensures ∀ (k in 0 .. old(_1).length), _1[k] == old(_1)[k]
      ensures ∀ (k in 0 .. _2.length), _1[old(_1).length + k] == _2[k]

    spec extend_from_slice at {i16} {A} where
      ensures old(_1).length + _2.length <= MAX_U64
      ensures _1.length == old(_1).length + _2.length
      ensures ∀ (k in 0 .. old(_1).length), _1[k] == old(_1)[k]
      ensures ∀ (k in 0 .. _2.length), _1[old(_1).length + k] == _2[k]

    spec extend_from_slice at {i32} {A} where
      ensures old(_1).length + _2.length <= MAX_U64
      ensures _1.length == old(_1).length + _2.length
      ensures ∀ (k in 0 .. old(_1).length), _1[k] == old(_1)[k]
      ensures ∀ (k in 0 .. _2.length), _1[old(_1).length + k] == _2[k]

    spec extend_from_slice at {i64} {A} where
      ensures old(_1).length + _2.length <= MAX_U64
      ensures _1.length == old(_1).length + _2.length
      ensures ∀ (k in 0 .. old(_1).length), _1[k] == old(_1)[k]
      ensures ∀ (k in 0 .. _2.length), _1[old(_1).length + k] == _2[k]

    spec extend_from_slice at {i128} {A} where
      ensures old(_1).length + _2.length <= MAX_U64
      ensures _1.length == old(_1).length + _2.length
      ensures ∀ (k in 0 .. old(_1).length), _1[k] == old(_1)[k]
      ensures ∀ (k in 0 .. _2.length), _1[old(_1).length + k] == _2[k]

    spec extend_from_slice at {isize} {A} where
      ensures old(_1).length + _2.length <= MAX_U64
      ensures _1.length == old(_1).length + _2.length
      ensures ∀ (k in 0 .. old(_1).length), _1[k] == old(_1)[k]
      ensures ∀ (k in 0 .. _2.length), _1[old(_1).length + k] == _2[k]

    spec extend_from_slice at {Bool} {A} where
      ensures old(_1).length + _2.length <= MAX_U64
      ensures _1.length == old(_1).length + _2.length
      ensures ∀ (k in 0 .. old(_1).length), _1[k] == old(_1)[k]
      ensures ∀ (k in 0 .. _2.length), _1[old(_1).length + k] == _2[k]

    spec extend_from_slice at {Char} {A} where
      ensures old(_1).length + _2.length <= MAX_U64
      ensures _1.length == old(_1).length + _2.length
      ensures ∀ (k in 0 .. old(_1).length), _1[k] == old(_1)[k]
      ensures ∀ (k in 0 .. _2.length), _1[old(_1).length + k] == _2[k]

    spec len where
      ensures result == _1.length

    spec is_empty where
      ensures result == (_1.length == 0)

  -- `Clone` is the identity at a type without heap state.
  trait impl «<std::vec::Vec<T, A> as std::clone::Clone>» where
    spec clone at {u8} {A} where
      ensures result == _1

    spec clone at {u16} {A} where
      ensures result == _1

    spec clone at {u32} {A} where
      ensures result == _1

    spec clone at {u64} {A} where
      ensures result == _1

    spec clone at {u128} {A} where
      ensures result == _1

    spec clone at {usize} {A} where
      ensures result == _1

    spec clone at {i8} {A} where
      ensures result == _1

    spec clone at {i16} {A} where
      ensures result == _1

    spec clone at {i32} {A} where
      ensures result == _1

    spec clone at {i64} {A} where
      ensures result == _1

    spec clone at {i128} {A} where
      ensures result == _1

    spec clone at {isize} {A} where
      ensures result == _1

    spec clone at {Bool} {A} where
      ensures result == _1

    spec clone at {Char} {A} where
      ensures result == _1

  trait impl «<std::vec::Vec<T, A> as std::ops::Deref>» where
    spec deref where
      ensures result == _1

  trait impl «<std::vec::Vec<T, A> as std::ops::DerefMut>» where
    spec deref_mut where
      ensures result == old(_1)
      ensures _1 == final(result)

  trait impl «<std::vec::Vec<T, A> as std::ops::Index<I>>» where
    spec index at {T} {usize} {A} {&T} where
      panics_if _2 >= _1.length
      ensures result == _1[_2]

  trait impl «<std::vec::Vec<T, A> as std::ops::IndexMut<I>>» where
    spec index_mut at {T} {usize} {A} {T} where
      panics_if _2 >= old(_1).length
      ensures result == old(_1)[_2]
      ensures _1 == update(old(_1), _2, final(result))

  trait impl «<&'a std::vec::Vec<T, A> as std::iter::IntoIterator>» where
    spec into_iter at {T} {A} {core::slice::iter::Iter<lifetime _, T>} where
      ensures result.slice == _1 && result.pos == 0 && result.end == _1.length
