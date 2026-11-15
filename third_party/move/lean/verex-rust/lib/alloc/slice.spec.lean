-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- `Clone` is the identity at a type without heap state.
mod primitive where
  impl slice where
    -- How many elements of a vector are a value.
    spec fun count {T}(v : Vector<T>, x : T) : Int decreases v.length :=
      if v.length == 0 then 0
      else (if v[0] == x then 1 else 0) + count(v[1 .. v.length], x)

    spec sort at {u8} where
      ensures _1.length == old(_1).length
      ensures ∀ (x : u8), count(_1, x) == count(old(_1), x)
      ensures ∀ (i in 0 .. _1.length; j in 0 .. _1.length), i <= j ==> _1[i] <= _1[j]

    spec sort at {u16} where
      ensures _1.length == old(_1).length
      ensures ∀ (x : u16), count(_1, x) == count(old(_1), x)
      ensures ∀ (i in 0 .. _1.length; j in 0 .. _1.length), i <= j ==> _1[i] <= _1[j]

    spec sort at {u32} where
      ensures _1.length == old(_1).length
      ensures ∀ (x : u32), count(_1, x) == count(old(_1), x)
      ensures ∀ (i in 0 .. _1.length; j in 0 .. _1.length), i <= j ==> _1[i] <= _1[j]

    spec sort at {u64} where
      ensures _1.length == old(_1).length
      ensures ∀ (x : u64), count(_1, x) == count(old(_1), x)
      ensures ∀ (i in 0 .. _1.length; j in 0 .. _1.length), i <= j ==> _1[i] <= _1[j]

    spec sort at {u128} where
      ensures _1.length == old(_1).length
      ensures ∀ (x : u128), count(_1, x) == count(old(_1), x)
      ensures ∀ (i in 0 .. _1.length; j in 0 .. _1.length), i <= j ==> _1[i] <= _1[j]

    spec sort at {usize} where
      ensures _1.length == old(_1).length
      ensures ∀ (x : usize), count(_1, x) == count(old(_1), x)
      ensures ∀ (i in 0 .. _1.length; j in 0 .. _1.length), i <= j ==> _1[i] <= _1[j]

    spec sort at {i8} where
      ensures _1.length == old(_1).length
      ensures ∀ (x : i8), count(_1, x) == count(old(_1), x)
      ensures ∀ (i in 0 .. _1.length; j in 0 .. _1.length), i <= j ==> _1[i] <= _1[j]

    spec sort at {i16} where
      ensures _1.length == old(_1).length
      ensures ∀ (x : i16), count(_1, x) == count(old(_1), x)
      ensures ∀ (i in 0 .. _1.length; j in 0 .. _1.length), i <= j ==> _1[i] <= _1[j]

    spec sort at {i32} where
      ensures _1.length == old(_1).length
      ensures ∀ (x : i32), count(_1, x) == count(old(_1), x)
      ensures ∀ (i in 0 .. _1.length; j in 0 .. _1.length), i <= j ==> _1[i] <= _1[j]

    spec sort at {i64} where
      ensures _1.length == old(_1).length
      ensures ∀ (x : i64), count(_1, x) == count(old(_1), x)
      ensures ∀ (i in 0 .. _1.length; j in 0 .. _1.length), i <= j ==> _1[i] <= _1[j]

    spec sort at {i128} where
      ensures _1.length == old(_1).length
      ensures ∀ (x : i128), count(_1, x) == count(old(_1), x)
      ensures ∀ (i in 0 .. _1.length; j in 0 .. _1.length), i <= j ==> _1[i] <= _1[j]

    spec sort at {isize} where
      ensures _1.length == old(_1).length
      ensures ∀ (x : isize), count(_1, x) == count(old(_1), x)
      ensures ∀ (i in 0 .. _1.length; j in 0 .. _1.length), i <= j ==> _1[i] <= _1[j]

    spec to_vec at {u8} where
      ensures result == _1

    spec to_vec at {u16} where
      ensures result == _1

    spec to_vec at {u32} where
      ensures result == _1

    spec to_vec at {u64} where
      ensures result == _1

    spec to_vec at {u128} where
      ensures result == _1

    spec to_vec at {usize} where
      ensures result == _1

    spec to_vec at {i8} where
      ensures result == _1

    spec to_vec at {i16} where
      ensures result == _1

    spec to_vec at {i32} where
      ensures result == _1

    spec to_vec at {i64} where
      ensures result == _1

    spec to_vec at {i128} where
      ensures result == _1

    spec to_vec at {isize} where
      ensures result == _1

    spec to_vec at {Bool} where
      ensures result == _1

    spec to_vec at {Char} where
      ensures result == _1
