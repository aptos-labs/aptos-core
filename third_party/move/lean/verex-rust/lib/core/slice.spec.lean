-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

mod primitive where
  impl slice where
    spec get at {T} {usize} {&T} where
      ensures _2 < _1.length ==> result is Some && result.0 == _1[_2]
      ensures _2 >= _1.length ==> result is None

    -- A yielded mutable reference writes back into the slice at its position:
    -- the element takes the reference's final value.
    spec get_mut at {T} {usize} {T} where
      ensures _2 < old(_1).length ==> result is Some
      ensures result is Some ==> result.0 == old(_1)[_2]
      ensures _2 >= old(_1).length ==> result is None
      ensures _2 < old(_1).length ==> _1 == update(old(_1), _2, final(result.0))
      ensures _2 >= old(_1).length ==> _1 == old(_1)

    spec is_empty where
      ensures result == (_1.length == 0)

    spec iter where
      ensures result.slice == _1 && result.pos == 0 && result.end == _1.length

    -- The iterator holds the whole lender and the positions left; writes
    -- through the references it yields reach the lender's final value.
    spec iter_mut where
      ensures result.pos == 0 && result.end == old(_1).length
      ensures result.slice == old(_1)
      ensures _1 == final(result.slice)

    spec first where
      ensures _1.length > 0 ==> result is Some && result.0 == _1[0]
      ensures _1.length == 0 ==> result is None

    spec last where
      ensures _1.length > 0 ==> result is Some && result.0 == _1[_1.length - 1]
      ensures _1.length == 0 ==> result is None

    spec split_at where
      panics_if _2 > _1.length
      ensures spec.result[0] == _1[0 .. _2]
      ensures spec.result[1] == _1[_2 .. _1.length]

    spec reverse where
      ensures _1.length == old(_1).length
      ensures ∀ (k in 0 .. _1.length), _1[k] == old(_1)[old(_1).length - 1 - k]

    spec contains at {u8} where
      ensures result == (∃ (k in 0 .. _1.length), _1[k] == _2)

    spec contains at {u16} where
      ensures result == (∃ (k in 0 .. _1.length), _1[k] == _2)

    spec contains at {u32} where
      ensures result == (∃ (k in 0 .. _1.length), _1[k] == _2)

    spec contains at {u64} where
      ensures result == (∃ (k in 0 .. _1.length), _1[k] == _2)

    spec contains at {u128} where
      ensures result == (∃ (k in 0 .. _1.length), _1[k] == _2)

    spec contains at {usize} where
      ensures result == (∃ (k in 0 .. _1.length), _1[k] == _2)

    spec contains at {i8} where
      ensures result == (∃ (k in 0 .. _1.length), _1[k] == _2)

    spec contains at {i16} where
      ensures result == (∃ (k in 0 .. _1.length), _1[k] == _2)

    spec contains at {i32} where
      ensures result == (∃ (k in 0 .. _1.length), _1[k] == _2)

    spec contains at {i64} where
      ensures result == (∃ (k in 0 .. _1.length), _1[k] == _2)

    spec contains at {i128} where
      ensures result == (∃ (k in 0 .. _1.length), _1[k] == _2)

    spec contains at {isize} where
      ensures result == (∃ (k in 0 .. _1.length), _1[k] == _2)

    spec contains at {Bool} where
      ensures result == (∃ (k in 0 .. _1.length), _1[k] == _2)

    spec contains at {Char} where
      ensures result == (∃ (k in 0 .. _1.length), _1[k] == _2)

    spec swap where
      panics_if _2 >= old(_1).length || _3 >= old(_1).length
      ensures _1 == update(update(old(_1), _2, old(_1)[_3]), _3, old(_1)[_2])

    spec fill at {Bool} where
      ensures _1.length == old(_1).length
      ensures ∀ (k in 0 .. _1.length), _1[k] == _2

-- A model of the private pointers: `slice` the whole slice, the elements left at
-- positions `pos .. end`.
mod slice where
  mod iter where
    @[model] struct Iter {a : lifetime} {T} where
      slice : &[a] Slice<T>
      pos : usize
      end : usize

    trait impl «<std::slice::Iter<'a, T> as std::iter::Iterator>» where
      spec next where
        ensures (result is Some) == (old(_1).pos < old(_1).end)
        ensures result is Some ==> result.0 == old(_1).slice[old(_1).pos]
        ensures _1.slice == old(_1).slice && _1.end == old(_1).end
        ensures _1.pos == (if old(_1).pos < old(_1).end then old(_1).pos + 1 else old(_1).pos)

    trait impl «core::slice::iter::<impl std::iter::IntoIterator for &'a [T]>» where
      spec into_iter where
        ensures result.slice == _1 && result.pos == 0 && result.end == _1.length

    @[model] struct IterMut {a : lifetime} {T} where
      slice : &[a] mut Slice<T>
      pos : usize
      end : usize

    trait impl «<std::slice::IterMut<'a, T> as std::iter::Iterator>» where
      -- The element at `pos` takes the final value of the reference yielded
      -- for it; every other position of the lender is left alone.
      spec next where
        ensures (result is Some) == (old(_1).pos < old(_1).end)
        ensures result is Some ==> result.0 == old(_1).slice[old(_1).pos]
        ensures _1.end == old(_1).end
        ensures _1.pos == (if old(_1).pos < old(_1).end then old(_1).pos + 1 else old(_1).pos)
        ensures result is Some ==>
          _1.slice == update(old(_1).slice, old(_1).pos, final(result.0))
        ensures result is None ==> _1.slice == old(_1).slice

    trait impl «core::slice::iter::<impl std::iter::IntoIterator for &'a mut [T]>» where
      spec into_iter where
        ensures result.pos == 0 && result.end == old(_1).length
        ensures result.slice == old(_1)
        ensures _1 == final(result.slice)
