-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

mod iter where
  mod traits where
    mod collect where
      -- The blanket impl for an iterator.
      trait impl «<I as std::iter::IntoIterator>» where
        spec into_iter where
          ensures result == _1

  mod range where
    trait impl «std::iter::range::<impl std::iter::Iterator for std::ops::Range<A>>» where
      spec next at {u8} where
        ensures (result is Some) == (old(_1).start < old(_1).end)
        ensures result is Some ==> result.0 == old(_1).start
        ensures _1.start == (if old(_1).start < old(_1).end then old(_1).start + 1 else old(_1).start)
        ensures _1.end == old(_1).end

      spec next at {u16} where
        ensures (result is Some) == (old(_1).start < old(_1).end)
        ensures result is Some ==> result.0 == old(_1).start
        ensures _1.start == (if old(_1).start < old(_1).end then old(_1).start + 1 else old(_1).start)
        ensures _1.end == old(_1).end

      spec next at {u32} where
        ensures (result is Some) == (old(_1).start < old(_1).end)
        ensures result is Some ==> result.0 == old(_1).start
        ensures _1.start == (if old(_1).start < old(_1).end then old(_1).start + 1 else old(_1).start)
        ensures _1.end == old(_1).end

      spec next at {u64} where
        ensures (result is Some) == (old(_1).start < old(_1).end)
        ensures result is Some ==> result.0 == old(_1).start
        ensures _1.start == (if old(_1).start < old(_1).end then old(_1).start + 1 else old(_1).start)
        ensures _1.end == old(_1).end

      spec next at {u128} where
        ensures (result is Some) == (old(_1).start < old(_1).end)
        ensures result is Some ==> result.0 == old(_1).start
        ensures _1.start == (if old(_1).start < old(_1).end then old(_1).start + 1 else old(_1).start)
        ensures _1.end == old(_1).end

      spec next at {usize} where
        ensures (result is Some) == (old(_1).start < old(_1).end)
        ensures result is Some ==> result.0 == old(_1).start
        ensures _1.start == (if old(_1).start < old(_1).end then old(_1).start + 1 else old(_1).start)
        ensures _1.end == old(_1).end

      spec next at {i8} where
        ensures (result is Some) == (old(_1).start < old(_1).end)
        ensures result is Some ==> result.0 == old(_1).start
        ensures _1.start == (if old(_1).start < old(_1).end then old(_1).start + 1 else old(_1).start)
        ensures _1.end == old(_1).end

      spec next at {i16} where
        ensures (result is Some) == (old(_1).start < old(_1).end)
        ensures result is Some ==> result.0 == old(_1).start
        ensures _1.start == (if old(_1).start < old(_1).end then old(_1).start + 1 else old(_1).start)
        ensures _1.end == old(_1).end

      spec next at {i32} where
        ensures (result is Some) == (old(_1).start < old(_1).end)
        ensures result is Some ==> result.0 == old(_1).start
        ensures _1.start == (if old(_1).start < old(_1).end then old(_1).start + 1 else old(_1).start)
        ensures _1.end == old(_1).end

      spec next at {i64} where
        ensures (result is Some) == (old(_1).start < old(_1).end)
        ensures result is Some ==> result.0 == old(_1).start
        ensures _1.start == (if old(_1).start < old(_1).end then old(_1).start + 1 else old(_1).start)
        ensures _1.end == old(_1).end

      spec next at {i128} where
        ensures (result is Some) == (old(_1).start < old(_1).end)
        ensures result is Some ==> result.0 == old(_1).start
        ensures _1.start == (if old(_1).start < old(_1).end then old(_1).start + 1 else old(_1).start)
        ensures _1.end == old(_1).end

      spec next at {isize} where
        ensures (result is Some) == (old(_1).start < old(_1).end)
        ensures result is Some ==> result.0 == old(_1).start
        ensures _1.start == (if old(_1).start < old(_1).end then old(_1).start + 1 else old(_1).start)
        ensures _1.end == old(_1).end

    trait impl «std::iter::range::<impl std::iter::Iterator for std::ops::RangeInclusive<A>>» where
      spec next at {u8} where
        ensures (result is Some) == (!old(_1).exhausted && old(_1).start <= old(_1).end)
        ensures result is Some ==> result.0 == old(_1).start
        ensures _1.end == old(_1).end
        ensures _1.start == (if result is None then old(_1).start else if old(_1).start == MAX_U8 then 0 else old(_1).start + 1)
        ensures _1.exhausted == (if result is None then old(_1).exhausted else old(_1).start == MAX_U8)

      spec next at {u16} where
        ensures (result is Some) == (!old(_1).exhausted && old(_1).start <= old(_1).end)
        ensures result is Some ==> result.0 == old(_1).start
        ensures _1.end == old(_1).end
        ensures _1.start == (if result is None then old(_1).start else if old(_1).start == MAX_U16 then 0 else old(_1).start + 1)
        ensures _1.exhausted == (if result is None then old(_1).exhausted else old(_1).start == MAX_U16)

      spec next at {u32} where
        ensures (result is Some) == (!old(_1).exhausted && old(_1).start <= old(_1).end)
        ensures result is Some ==> result.0 == old(_1).start
        ensures _1.end == old(_1).end
        ensures _1.start == (if result is None then old(_1).start else if old(_1).start == MAX_U32 then 0 else old(_1).start + 1)
        ensures _1.exhausted == (if result is None then old(_1).exhausted else old(_1).start == MAX_U32)

      spec next at {u64} where
        ensures (result is Some) == (!old(_1).exhausted && old(_1).start <= old(_1).end)
        ensures result is Some ==> result.0 == old(_1).start
        ensures _1.end == old(_1).end
        ensures _1.start == (if result is None then old(_1).start else if old(_1).start == MAX_U64 then 0 else old(_1).start + 1)
        ensures _1.exhausted == (if result is None then old(_1).exhausted else old(_1).start == MAX_U64)

      spec next at {u128} where
        ensures (result is Some) == (!old(_1).exhausted && old(_1).start <= old(_1).end)
        ensures result is Some ==> result.0 == old(_1).start
        ensures _1.end == old(_1).end
        ensures _1.start == (if result is None then old(_1).start else if old(_1).start == MAX_U128 then 0 else old(_1).start + 1)
        ensures _1.exhausted == (if result is None then old(_1).exhausted else old(_1).start == MAX_U128)

      spec next at {usize} where
        ensures (result is Some) == (!old(_1).exhausted && old(_1).start <= old(_1).end)
        ensures result is Some ==> result.0 == old(_1).start
        ensures _1.end == old(_1).end
        ensures _1.start == (if result is None then old(_1).start else if old(_1).start == MAX_U64 then 0 else old(_1).start + 1)
        ensures _1.exhausted == (if result is None then old(_1).exhausted else old(_1).start == MAX_U64)

      spec next at {i8} where
        ensures (result is Some) == (!old(_1).exhausted && old(_1).start <= old(_1).end)
        ensures result is Some ==> result.0 == old(_1).start
        ensures _1.end == old(_1).end
        ensures _1.start == (if result is None then old(_1).start else if old(_1).start == MAX_I8 then MIN_I8 else old(_1).start + 1)
        ensures _1.exhausted == (if result is None then old(_1).exhausted else old(_1).start == MAX_I8)

      spec next at {i16} where
        ensures (result is Some) == (!old(_1).exhausted && old(_1).start <= old(_1).end)
        ensures result is Some ==> result.0 == old(_1).start
        ensures _1.end == old(_1).end
        ensures _1.start == (if result is None then old(_1).start else if old(_1).start == MAX_I16 then MIN_I16 else old(_1).start + 1)
        ensures _1.exhausted == (if result is None then old(_1).exhausted else old(_1).start == MAX_I16)

      spec next at {i32} where
        ensures (result is Some) == (!old(_1).exhausted && old(_1).start <= old(_1).end)
        ensures result is Some ==> result.0 == old(_1).start
        ensures _1.end == old(_1).end
        ensures _1.start == (if result is None then old(_1).start else if old(_1).start == MAX_I32 then MIN_I32 else old(_1).start + 1)
        ensures _1.exhausted == (if result is None then old(_1).exhausted else old(_1).start == MAX_I32)

      spec next at {i64} where
        ensures (result is Some) == (!old(_1).exhausted && old(_1).start <= old(_1).end)
        ensures result is Some ==> result.0 == old(_1).start
        ensures _1.end == old(_1).end
        ensures _1.start == (if result is None then old(_1).start else if old(_1).start == MAX_I64 then MIN_I64 else old(_1).start + 1)
        ensures _1.exhausted == (if result is None then old(_1).exhausted else old(_1).start == MAX_I64)

      spec next at {i128} where
        ensures (result is Some) == (!old(_1).exhausted && old(_1).start <= old(_1).end)
        ensures result is Some ==> result.0 == old(_1).start
        ensures _1.end == old(_1).end
        ensures _1.start == (if result is None then old(_1).start else if old(_1).start == MAX_I128 then MIN_I128 else old(_1).start + 1)
        ensures _1.exhausted == (if result is None then old(_1).exhausted else old(_1).start == MAX_I128)

      spec next at {isize} where
        ensures (result is Some) == (!old(_1).exhausted && old(_1).start <= old(_1).end)
        ensures result is Some ==> result.0 == old(_1).start
        ensures _1.end == old(_1).end
        ensures _1.start == (if result is None then old(_1).start else if old(_1).start == MAX_I64 then MIN_I64 else old(_1).start + 1)
        ensures _1.exhausted == (if result is None then old(_1).exhausted else old(_1).start == MAX_I64)

  mod adapters where
    mod rev where
      @[model] struct Rev {T} where
        iter : T

      -- `next` is the inner `next_back`.
      trait impl «<std::iter::Rev<I> as std::iter::Iterator>» where
        spec next at {ops::range::«Range»<u8>} {u8} where
          ensures (result is Some) == (old(_1).iter.start < old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.end - 1
          ensures _1.iter.start == old(_1).iter.start
          ensures _1.iter.end == (if old(_1).iter.start < old(_1).iter.end then old(_1).iter.end - 1 else old(_1).iter.end)

        spec next at {ops::range::«Range»<u16>} {u16} where
          ensures (result is Some) == (old(_1).iter.start < old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.end - 1
          ensures _1.iter.start == old(_1).iter.start
          ensures _1.iter.end == (if old(_1).iter.start < old(_1).iter.end then old(_1).iter.end - 1 else old(_1).iter.end)

        spec next at {ops::range::«Range»<u32>} {u32} where
          ensures (result is Some) == (old(_1).iter.start < old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.end - 1
          ensures _1.iter.start == old(_1).iter.start
          ensures _1.iter.end == (if old(_1).iter.start < old(_1).iter.end then old(_1).iter.end - 1 else old(_1).iter.end)

        spec next at {ops::range::«Range»<u64>} {u64} where
          ensures (result is Some) == (old(_1).iter.start < old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.end - 1
          ensures _1.iter.start == old(_1).iter.start
          ensures _1.iter.end == (if old(_1).iter.start < old(_1).iter.end then old(_1).iter.end - 1 else old(_1).iter.end)

        spec next at {ops::range::«Range»<u128>} {u128} where
          ensures (result is Some) == (old(_1).iter.start < old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.end - 1
          ensures _1.iter.start == old(_1).iter.start
          ensures _1.iter.end == (if old(_1).iter.start < old(_1).iter.end then old(_1).iter.end - 1 else old(_1).iter.end)

        spec next at {ops::range::«Range»<usize>} {usize} where
          ensures (result is Some) == (old(_1).iter.start < old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.end - 1
          ensures _1.iter.start == old(_1).iter.start
          ensures _1.iter.end == (if old(_1).iter.start < old(_1).iter.end then old(_1).iter.end - 1 else old(_1).iter.end)

        spec next at {ops::range::«Range»<i8>} {i8} where
          ensures (result is Some) == (old(_1).iter.start < old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.end - 1
          ensures _1.iter.start == old(_1).iter.start
          ensures _1.iter.end == (if old(_1).iter.start < old(_1).iter.end then old(_1).iter.end - 1 else old(_1).iter.end)

        spec next at {ops::range::«Range»<i16>} {i16} where
          ensures (result is Some) == (old(_1).iter.start < old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.end - 1
          ensures _1.iter.start == old(_1).iter.start
          ensures _1.iter.end == (if old(_1).iter.start < old(_1).iter.end then old(_1).iter.end - 1 else old(_1).iter.end)

        spec next at {ops::range::«Range»<i32>} {i32} where
          ensures (result is Some) == (old(_1).iter.start < old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.end - 1
          ensures _1.iter.start == old(_1).iter.start
          ensures _1.iter.end == (if old(_1).iter.start < old(_1).iter.end then old(_1).iter.end - 1 else old(_1).iter.end)

        spec next at {ops::range::«Range»<i64>} {i64} where
          ensures (result is Some) == (old(_1).iter.start < old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.end - 1
          ensures _1.iter.start == old(_1).iter.start
          ensures _1.iter.end == (if old(_1).iter.start < old(_1).iter.end then old(_1).iter.end - 1 else old(_1).iter.end)

        spec next at {ops::range::«Range»<i128>} {i128} where
          ensures (result is Some) == (old(_1).iter.start < old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.end - 1
          ensures _1.iter.start == old(_1).iter.start
          ensures _1.iter.end == (if old(_1).iter.start < old(_1).iter.end then old(_1).iter.end - 1 else old(_1).iter.end)

        spec next at {ops::range::«Range»<isize>} {isize} where
          ensures (result is Some) == (old(_1).iter.start < old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.end - 1
          ensures _1.iter.start == old(_1).iter.start
          ensures _1.iter.end == (if old(_1).iter.start < old(_1).iter.end then old(_1).iter.end - 1 else old(_1).iter.end)

        spec next at {ops::range::RangeInclusive<u8>} {u8} where
          ensures (result is Some) == (!old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.end
          ensures _1.iter.start == old(_1).iter.start
          ensures _1.iter.end == (if result is None then old(_1).iter.end else if old(_1).iter.end == 0 then MAX_U8 else old(_1).iter.end - 1)
          ensures _1.iter.exhausted == (if result is None then old(_1).iter.exhausted else old(_1).iter.end == 0)

        spec next at {ops::range::RangeInclusive<u16>} {u16} where
          ensures (result is Some) == (!old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.end
          ensures _1.iter.start == old(_1).iter.start
          ensures _1.iter.end == (if result is None then old(_1).iter.end else if old(_1).iter.end == 0 then MAX_U16 else old(_1).iter.end - 1)
          ensures _1.iter.exhausted == (if result is None then old(_1).iter.exhausted else old(_1).iter.end == 0)

        spec next at {ops::range::RangeInclusive<u32>} {u32} where
          ensures (result is Some) == (!old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.end
          ensures _1.iter.start == old(_1).iter.start
          ensures _1.iter.end == (if result is None then old(_1).iter.end else if old(_1).iter.end == 0 then MAX_U32 else old(_1).iter.end - 1)
          ensures _1.iter.exhausted == (if result is None then old(_1).iter.exhausted else old(_1).iter.end == 0)

        spec next at {ops::range::RangeInclusive<u64>} {u64} where
          ensures (result is Some) == (!old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.end
          ensures _1.iter.start == old(_1).iter.start
          ensures _1.iter.end == (if result is None then old(_1).iter.end else if old(_1).iter.end == 0 then MAX_U64 else old(_1).iter.end - 1)
          ensures _1.iter.exhausted == (if result is None then old(_1).iter.exhausted else old(_1).iter.end == 0)

        spec next at {ops::range::RangeInclusive<u128>} {u128} where
          ensures (result is Some) == (!old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.end
          ensures _1.iter.start == old(_1).iter.start
          ensures _1.iter.end == (if result is None then old(_1).iter.end else if old(_1).iter.end == 0 then MAX_U128 else old(_1).iter.end - 1)
          ensures _1.iter.exhausted == (if result is None then old(_1).iter.exhausted else old(_1).iter.end == 0)

        spec next at {ops::range::RangeInclusive<usize>} {usize} where
          ensures (result is Some) == (!old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.end
          ensures _1.iter.start == old(_1).iter.start
          ensures _1.iter.end == (if result is None then old(_1).iter.end else if old(_1).iter.end == 0 then MAX_U64 else old(_1).iter.end - 1)
          ensures _1.iter.exhausted == (if result is None then old(_1).iter.exhausted else old(_1).iter.end == 0)

        spec next at {ops::range::RangeInclusive<i8>} {i8} where
          ensures (result is Some) == (!old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.end
          ensures _1.iter.start == old(_1).iter.start
          ensures _1.iter.end == (if result is None then old(_1).iter.end else if old(_1).iter.end == MIN_I8 then MAX_I8 else old(_1).iter.end - 1)
          ensures _1.iter.exhausted == (if result is None then old(_1).iter.exhausted else old(_1).iter.end == MIN_I8)

        spec next at {ops::range::RangeInclusive<i16>} {i16} where
          ensures (result is Some) == (!old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.end
          ensures _1.iter.start == old(_1).iter.start
          ensures _1.iter.end == (if result is None then old(_1).iter.end else if old(_1).iter.end == MIN_I16 then MAX_I16 else old(_1).iter.end - 1)
          ensures _1.iter.exhausted == (if result is None then old(_1).iter.exhausted else old(_1).iter.end == MIN_I16)

        spec next at {ops::range::RangeInclusive<i32>} {i32} where
          ensures (result is Some) == (!old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.end
          ensures _1.iter.start == old(_1).iter.start
          ensures _1.iter.end == (if result is None then old(_1).iter.end else if old(_1).iter.end == MIN_I32 then MAX_I32 else old(_1).iter.end - 1)
          ensures _1.iter.exhausted == (if result is None then old(_1).iter.exhausted else old(_1).iter.end == MIN_I32)

        spec next at {ops::range::RangeInclusive<i64>} {i64} where
          ensures (result is Some) == (!old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.end
          ensures _1.iter.start == old(_1).iter.start
          ensures _1.iter.end == (if result is None then old(_1).iter.end else if old(_1).iter.end == MIN_I64 then MAX_I64 else old(_1).iter.end - 1)
          ensures _1.iter.exhausted == (if result is None then old(_1).iter.exhausted else old(_1).iter.end == MIN_I64)

        spec next at {ops::range::RangeInclusive<i128>} {i128} where
          ensures (result is Some) == (!old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.end
          ensures _1.iter.start == old(_1).iter.start
          ensures _1.iter.end == (if result is None then old(_1).iter.end else if old(_1).iter.end == MIN_I128 then MAX_I128 else old(_1).iter.end - 1)
          ensures _1.iter.exhausted == (if result is None then old(_1).iter.exhausted else old(_1).iter.end == MIN_I128)

        spec next at {ops::range::RangeInclusive<isize>} {isize} where
          ensures (result is Some) == (!old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.end
          ensures _1.iter.start == old(_1).iter.start
          ensures _1.iter.end == (if result is None then old(_1).iter.end else if old(_1).iter.end == MIN_I64 then MAX_I64 else old(_1).iter.end - 1)
          ensures _1.iter.exhausted == (if result is None then old(_1).iter.exhausted else old(_1).iter.end == MIN_I64)

        spec next at {slice::iter::Iter<lifetime _, Item>} {&Item} where
          ensures (result is Some) == (old(_1).iter.pos < old(_1).iter.end)
          ensures result is Some ==> result.0 == old(_1).iter.slice[old(_1).iter.end - 1]
          ensures _1.iter.slice == old(_1).iter.slice && _1.iter.pos == old(_1).iter.pos
          ensures _1.iter.end == (if old(_1).iter.pos < old(_1).iter.end then old(_1).iter.end - 1 else old(_1).iter.end)

        -- `Enumerate::next_back` pairs `count` with the elements left behind it.
        spec next at {iter::adapters::enumerate::Enumerate<slice::iter::Iter<lifetime _, Item> >} {(usize, &Item)} where
          ensures (result is Some) == (old(_1).iter.iter.pos < old(_1).iter.iter.end)
          ensures result is Some ==> result.0 == (old(_1).iter.count + old(_1).iter.iter.end - 1 - old(_1).iter.iter.pos, old(_1).iter.iter.slice[old(_1).iter.iter.end - 1])
          ensures _1.iter.count == old(_1).iter.count
          ensures _1.iter.iter.slice == old(_1).iter.iter.slice && _1.iter.iter.pos == old(_1).iter.iter.pos
          ensures _1.iter.iter.end == (if old(_1).iter.iter.pos < old(_1).iter.iter.end then old(_1).iter.iter.end - 1 else old(_1).iter.iter.end)

    mod enumerate where
      @[model] struct Enumerate {I} where
        iter : I
        count : usize

      -- `count` is checked, as `rustc_inherit_overflow_checks` asks of the crate's profile.
      trait impl «<std::iter::Enumerate<I> as std::iter::Iterator>» where
        spec next at {iter::adapters::rev::Rev<slice::iter::Iter<lifetime _, Item> >} {&Item} where
          panics_if old(_1).iter.iter.pos < old(_1).iter.iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (old(_1).iter.iter.pos < old(_1).iter.iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.iter.slice[old(_1).iter.iter.end - 1])
          ensures _1.iter.iter.slice == old(_1).iter.iter.slice && _1.iter.iter.pos == old(_1).iter.iter.pos
          ensures _1.iter.iter.end == (if old(_1).iter.iter.pos < old(_1).iter.iter.end then old(_1).iter.iter.end - 1 else old(_1).iter.iter.end)
          ensures _1.count == (if old(_1).iter.iter.pos < old(_1).iter.iter.end then old(_1).count + 1 else old(_1).count)

        spec next at {slice::iter::Iter<lifetime _, Item>} {&Item} where
          panics_if old(_1).iter.pos < old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (old(_1).iter.pos < old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.slice[old(_1).iter.pos])
          ensures _1.iter.slice == old(_1).iter.slice && _1.iter.end == old(_1).iter.end
          ensures _1.iter.pos == (if old(_1).iter.pos < old(_1).iter.end then old(_1).iter.pos + 1 else old(_1).iter.pos)
          ensures _1.count == (if old(_1).iter.pos < old(_1).iter.end then old(_1).count + 1 else old(_1).count)

        spec next at {ops::range::«Range»<u8>} {u8} where
          panics_if old(_1).iter.start < old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (old(_1).iter.start < old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.start)
          ensures _1.iter.end == old(_1).iter.end
          ensures _1.iter.start == (if old(_1).iter.start < old(_1).iter.end then old(_1).iter.start + 1 else old(_1).iter.start)
          ensures _1.count == (if old(_1).iter.start < old(_1).iter.end then old(_1).count + 1 else old(_1).count)

        spec next at {ops::range::«Range»<u16>} {u16} where
          panics_if old(_1).iter.start < old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (old(_1).iter.start < old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.start)
          ensures _1.iter.end == old(_1).iter.end
          ensures _1.iter.start == (if old(_1).iter.start < old(_1).iter.end then old(_1).iter.start + 1 else old(_1).iter.start)
          ensures _1.count == (if old(_1).iter.start < old(_1).iter.end then old(_1).count + 1 else old(_1).count)

        spec next at {ops::range::«Range»<u32>} {u32} where
          panics_if old(_1).iter.start < old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (old(_1).iter.start < old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.start)
          ensures _1.iter.end == old(_1).iter.end
          ensures _1.iter.start == (if old(_1).iter.start < old(_1).iter.end then old(_1).iter.start + 1 else old(_1).iter.start)
          ensures _1.count == (if old(_1).iter.start < old(_1).iter.end then old(_1).count + 1 else old(_1).count)

        spec next at {ops::range::«Range»<u64>} {u64} where
          panics_if old(_1).iter.start < old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (old(_1).iter.start < old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.start)
          ensures _1.iter.end == old(_1).iter.end
          ensures _1.iter.start == (if old(_1).iter.start < old(_1).iter.end then old(_1).iter.start + 1 else old(_1).iter.start)
          ensures _1.count == (if old(_1).iter.start < old(_1).iter.end then old(_1).count + 1 else old(_1).count)

        spec next at {ops::range::«Range»<u128>} {u128} where
          panics_if old(_1).iter.start < old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (old(_1).iter.start < old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.start)
          ensures _1.iter.end == old(_1).iter.end
          ensures _1.iter.start == (if old(_1).iter.start < old(_1).iter.end then old(_1).iter.start + 1 else old(_1).iter.start)
          ensures _1.count == (if old(_1).iter.start < old(_1).iter.end then old(_1).count + 1 else old(_1).count)

        spec next at {ops::range::«Range»<usize>} {usize} where
          panics_if old(_1).iter.start < old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (old(_1).iter.start < old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.start)
          ensures _1.iter.end == old(_1).iter.end
          ensures _1.iter.start == (if old(_1).iter.start < old(_1).iter.end then old(_1).iter.start + 1 else old(_1).iter.start)
          ensures _1.count == (if old(_1).iter.start < old(_1).iter.end then old(_1).count + 1 else old(_1).count)

        spec next at {ops::range::«Range»<i8>} {i8} where
          panics_if old(_1).iter.start < old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (old(_1).iter.start < old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.start)
          ensures _1.iter.end == old(_1).iter.end
          ensures _1.iter.start == (if old(_1).iter.start < old(_1).iter.end then old(_1).iter.start + 1 else old(_1).iter.start)
          ensures _1.count == (if old(_1).iter.start < old(_1).iter.end then old(_1).count + 1 else old(_1).count)

        spec next at {ops::range::«Range»<i16>} {i16} where
          panics_if old(_1).iter.start < old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (old(_1).iter.start < old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.start)
          ensures _1.iter.end == old(_1).iter.end
          ensures _1.iter.start == (if old(_1).iter.start < old(_1).iter.end then old(_1).iter.start + 1 else old(_1).iter.start)
          ensures _1.count == (if old(_1).iter.start < old(_1).iter.end then old(_1).count + 1 else old(_1).count)

        spec next at {ops::range::«Range»<i32>} {i32} where
          panics_if old(_1).iter.start < old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (old(_1).iter.start < old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.start)
          ensures _1.iter.end == old(_1).iter.end
          ensures _1.iter.start == (if old(_1).iter.start < old(_1).iter.end then old(_1).iter.start + 1 else old(_1).iter.start)
          ensures _1.count == (if old(_1).iter.start < old(_1).iter.end then old(_1).count + 1 else old(_1).count)

        spec next at {ops::range::«Range»<i64>} {i64} where
          panics_if old(_1).iter.start < old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (old(_1).iter.start < old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.start)
          ensures _1.iter.end == old(_1).iter.end
          ensures _1.iter.start == (if old(_1).iter.start < old(_1).iter.end then old(_1).iter.start + 1 else old(_1).iter.start)
          ensures _1.count == (if old(_1).iter.start < old(_1).iter.end then old(_1).count + 1 else old(_1).count)

        spec next at {ops::range::«Range»<i128>} {i128} where
          panics_if old(_1).iter.start < old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (old(_1).iter.start < old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.start)
          ensures _1.iter.end == old(_1).iter.end
          ensures _1.iter.start == (if old(_1).iter.start < old(_1).iter.end then old(_1).iter.start + 1 else old(_1).iter.start)
          ensures _1.count == (if old(_1).iter.start < old(_1).iter.end then old(_1).count + 1 else old(_1).count)

        spec next at {ops::range::«Range»<isize>} {isize} where
          panics_if old(_1).iter.start < old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (old(_1).iter.start < old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.start)
          ensures _1.iter.end == old(_1).iter.end
          ensures _1.iter.start == (if old(_1).iter.start < old(_1).iter.end then old(_1).iter.start + 1 else old(_1).iter.start)
          ensures _1.count == (if old(_1).iter.start < old(_1).iter.end then old(_1).count + 1 else old(_1).count)

        spec next at {ops::range::RangeInclusive<u8>} {u8} where
          panics_if !old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (!old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.start)
          ensures _1.iter.end == old(_1).iter.end
          ensures _1.iter.start == (if result is None then old(_1).iter.start else if old(_1).iter.start == MAX_U8 then 0 else old(_1).iter.start + 1)
          ensures _1.iter.exhausted == (if result is None then old(_1).iter.exhausted else old(_1).iter.start == MAX_U8)
          ensures _1.count == (if result is None then old(_1).count else old(_1).count + 1)

        spec next at {ops::range::RangeInclusive<u16>} {u16} where
          panics_if !old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (!old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.start)
          ensures _1.iter.end == old(_1).iter.end
          ensures _1.iter.start == (if result is None then old(_1).iter.start else if old(_1).iter.start == MAX_U16 then 0 else old(_1).iter.start + 1)
          ensures _1.iter.exhausted == (if result is None then old(_1).iter.exhausted else old(_1).iter.start == MAX_U16)
          ensures _1.count == (if result is None then old(_1).count else old(_1).count + 1)

        spec next at {ops::range::RangeInclusive<u32>} {u32} where
          panics_if !old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (!old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.start)
          ensures _1.iter.end == old(_1).iter.end
          ensures _1.iter.start == (if result is None then old(_1).iter.start else if old(_1).iter.start == MAX_U32 then 0 else old(_1).iter.start + 1)
          ensures _1.iter.exhausted == (if result is None then old(_1).iter.exhausted else old(_1).iter.start == MAX_U32)
          ensures _1.count == (if result is None then old(_1).count else old(_1).count + 1)

        spec next at {ops::range::RangeInclusive<u64>} {u64} where
          panics_if !old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (!old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.start)
          ensures _1.iter.end == old(_1).iter.end
          ensures _1.iter.start == (if result is None then old(_1).iter.start else if old(_1).iter.start == MAX_U64 then 0 else old(_1).iter.start + 1)
          ensures _1.iter.exhausted == (if result is None then old(_1).iter.exhausted else old(_1).iter.start == MAX_U64)
          ensures _1.count == (if result is None then old(_1).count else old(_1).count + 1)

        spec next at {ops::range::RangeInclusive<u128>} {u128} where
          panics_if !old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (!old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.start)
          ensures _1.iter.end == old(_1).iter.end
          ensures _1.iter.start == (if result is None then old(_1).iter.start else if old(_1).iter.start == MAX_U128 then 0 else old(_1).iter.start + 1)
          ensures _1.iter.exhausted == (if result is None then old(_1).iter.exhausted else old(_1).iter.start == MAX_U128)
          ensures _1.count == (if result is None then old(_1).count else old(_1).count + 1)

        spec next at {ops::range::RangeInclusive<usize>} {usize} where
          panics_if !old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (!old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.start)
          ensures _1.iter.end == old(_1).iter.end
          ensures _1.iter.start == (if result is None then old(_1).iter.start else if old(_1).iter.start == MAX_U64 then 0 else old(_1).iter.start + 1)
          ensures _1.iter.exhausted == (if result is None then old(_1).iter.exhausted else old(_1).iter.start == MAX_U64)
          ensures _1.count == (if result is None then old(_1).count else old(_1).count + 1)

        spec next at {ops::range::RangeInclusive<i8>} {i8} where
          panics_if !old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (!old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.start)
          ensures _1.iter.end == old(_1).iter.end
          ensures _1.iter.start == (if result is None then old(_1).iter.start else if old(_1).iter.start == MAX_I8 then MIN_I8 else old(_1).iter.start + 1)
          ensures _1.iter.exhausted == (if result is None then old(_1).iter.exhausted else old(_1).iter.start == MAX_I8)
          ensures _1.count == (if result is None then old(_1).count else old(_1).count + 1)

        spec next at {ops::range::RangeInclusive<i16>} {i16} where
          panics_if !old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (!old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.start)
          ensures _1.iter.end == old(_1).iter.end
          ensures _1.iter.start == (if result is None then old(_1).iter.start else if old(_1).iter.start == MAX_I16 then MIN_I16 else old(_1).iter.start + 1)
          ensures _1.iter.exhausted == (if result is None then old(_1).iter.exhausted else old(_1).iter.start == MAX_I16)
          ensures _1.count == (if result is None then old(_1).count else old(_1).count + 1)

        spec next at {ops::range::RangeInclusive<i32>} {i32} where
          panics_if !old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (!old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.start)
          ensures _1.iter.end == old(_1).iter.end
          ensures _1.iter.start == (if result is None then old(_1).iter.start else if old(_1).iter.start == MAX_I32 then MIN_I32 else old(_1).iter.start + 1)
          ensures _1.iter.exhausted == (if result is None then old(_1).iter.exhausted else old(_1).iter.start == MAX_I32)
          ensures _1.count == (if result is None then old(_1).count else old(_1).count + 1)

        spec next at {ops::range::RangeInclusive<i64>} {i64} where
          panics_if !old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (!old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.start)
          ensures _1.iter.end == old(_1).iter.end
          ensures _1.iter.start == (if result is None then old(_1).iter.start else if old(_1).iter.start == MAX_I64 then MIN_I64 else old(_1).iter.start + 1)
          ensures _1.iter.exhausted == (if result is None then old(_1).iter.exhausted else old(_1).iter.start == MAX_I64)
          ensures _1.count == (if result is None then old(_1).count else old(_1).count + 1)

        spec next at {ops::range::RangeInclusive<i128>} {i128} where
          panics_if !old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (!old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.start)
          ensures _1.iter.end == old(_1).iter.end
          ensures _1.iter.start == (if result is None then old(_1).iter.start else if old(_1).iter.start == MAX_I128 then MIN_I128 else old(_1).iter.start + 1)
          ensures _1.iter.exhausted == (if result is None then old(_1).iter.exhausted else old(_1).iter.start == MAX_I128)
          ensures _1.count == (if result is None then old(_1).count else old(_1).count + 1)

        spec next at {ops::range::RangeInclusive<isize>} {isize} where
          panics_if !old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end && old(_1).count == MAX_U64
          ensures (result is Some) == (!old(_1).iter.exhausted && old(_1).iter.start <= old(_1).iter.end)
          ensures result is Some ==> result.0 == (old(_1).count, old(_1).iter.start)
          ensures _1.iter.end == old(_1).iter.end
          ensures _1.iter.start == (if result is None then old(_1).iter.start else if old(_1).iter.start == MAX_I64 then MIN_I64 else old(_1).iter.start + 1)
          ensures _1.iter.exhausted == (if result is None then old(_1).iter.exhausted else old(_1).iter.start == MAX_I64)
          ensures _1.count == (if result is None then old(_1).count else old(_1).count + 1)

    mod step_by where
      @[model] struct StepBy {I} where
        iter : I
        step_minus_one : usize
        first_take : Bool

      -- Over an unsigned `Range` no wider than `usize`, `step_by` turns `end` into the
      -- count of elements left; a step past the type's range counts as its maximum.
      trait impl «<std::iter::StepBy<I> as std::iter::Iterator>» where
        spec next at {ops::range::«Range»<u8>} {u8} where
          ensures (result is Some) == (old(_1).iter.end > 0)
          ensures result is Some ==> result.0 == old(_1).iter.start
          ensures _1.step_minus_one == old(_1).step_minus_one && _1.first_take == old(_1).first_take
          ensures _1.iter.end == (if old(_1).iter.end > 0 then old(_1).iter.end - 1 else old(_1).iter.end)
          ensures _1.iter.start == (if old(_1).iter.end > 0 then (old(_1).iter.start + (if old(_1).step_minus_one >= MAX_U8 then MAX_U8 else old(_1).step_minus_one + 1)) % (MAX_U8 + 1) else old(_1).iter.start)

        spec next at {ops::range::«Range»<u16>} {u16} where
          ensures (result is Some) == (old(_1).iter.end > 0)
          ensures result is Some ==> result.0 == old(_1).iter.start
          ensures _1.step_minus_one == old(_1).step_minus_one && _1.first_take == old(_1).first_take
          ensures _1.iter.end == (if old(_1).iter.end > 0 then old(_1).iter.end - 1 else old(_1).iter.end)
          ensures _1.iter.start == (if old(_1).iter.end > 0 then (old(_1).iter.start + (if old(_1).step_minus_one >= MAX_U16 then MAX_U16 else old(_1).step_minus_one + 1)) % (MAX_U16 + 1) else old(_1).iter.start)

        spec next at {ops::range::«Range»<u32>} {u32} where
          ensures (result is Some) == (old(_1).iter.end > 0)
          ensures result is Some ==> result.0 == old(_1).iter.start
          ensures _1.step_minus_one == old(_1).step_minus_one && _1.first_take == old(_1).first_take
          ensures _1.iter.end == (if old(_1).iter.end > 0 then old(_1).iter.end - 1 else old(_1).iter.end)
          ensures _1.iter.start == (if old(_1).iter.end > 0 then (old(_1).iter.start + (if old(_1).step_minus_one >= MAX_U32 then MAX_U32 else old(_1).step_minus_one + 1)) % (MAX_U32 + 1) else old(_1).iter.start)

        spec next at {ops::range::«Range»<u64>} {u64} where
          ensures (result is Some) == (old(_1).iter.end > 0)
          ensures result is Some ==> result.0 == old(_1).iter.start
          ensures _1.step_minus_one == old(_1).step_minus_one && _1.first_take == old(_1).first_take
          ensures _1.iter.end == (if old(_1).iter.end > 0 then old(_1).iter.end - 1 else old(_1).iter.end)
          ensures _1.iter.start == (if old(_1).iter.end > 0 then (old(_1).iter.start + (if old(_1).step_minus_one >= MAX_U64 then MAX_U64 else old(_1).step_minus_one + 1)) % (MAX_U64 + 1) else old(_1).iter.start)

        spec next at {ops::range::«Range»<usize>} {usize} where
          ensures (result is Some) == (old(_1).iter.end > 0)
          ensures result is Some ==> result.0 == old(_1).iter.start
          ensures _1.step_minus_one == old(_1).step_minus_one && _1.first_take == old(_1).first_take
          ensures _1.iter.end == (if old(_1).iter.end > 0 then old(_1).iter.end - 1 else old(_1).iter.end)
          ensures _1.iter.start == (if old(_1).iter.end > 0 then (old(_1).iter.start + (if old(_1).step_minus_one >= MAX_U64 then MAX_U64 else old(_1).step_minus_one + 1)) % (MAX_U64 + 1) else old(_1).iter.start)

  mod traits where
    mod iterator where
      -- Provided methods: each wraps its receiver.
      trait Iterator where
        spec rev where
          ensures result.iter == _1

        spec enumerate where
          ensures result.iter == _1 && result.count == 0

        spec step_by at {ops::range::«Range»<u8>} where
          panics_if _2 == 0
          ensures result.step_minus_one == _2 - 1 && result.first_take
          ensures result.iter.start == _1.start
          ensures result.iter.end == (if _1.start < _1.end then (_1.end - _1.start + _2 - 1) / _2 else 0)

        spec step_by at {ops::range::«Range»<u16>} where
          panics_if _2 == 0
          ensures result.step_minus_one == _2 - 1 && result.first_take
          ensures result.iter.start == _1.start
          ensures result.iter.end == (if _1.start < _1.end then (_1.end - _1.start + _2 - 1) / _2 else 0)

        spec step_by at {ops::range::«Range»<u32>} where
          panics_if _2 == 0
          ensures result.step_minus_one == _2 - 1 && result.first_take
          ensures result.iter.start == _1.start
          ensures result.iter.end == (if _1.start < _1.end then (_1.end - _1.start + _2 - 1) / _2 else 0)

        spec step_by at {ops::range::«Range»<u64>} where
          panics_if _2 == 0
          ensures result.step_minus_one == _2 - 1 && result.first_take
          ensures result.iter.start == _1.start
          ensures result.iter.end == (if _1.start < _1.end then (_1.end - _1.start + _2 - 1) / _2 else 0)

        spec step_by at {ops::range::«Range»<usize>} where
          panics_if _2 == 0
          ensures result.step_minus_one == _2 - 1 && result.first_take
          ensures result.iter.start == _1.start
          ensures result.iter.end == (if _1.start < _1.end then (_1.end - _1.start + _2 - 1) / _2 else 0)

mod ops where
  mod range where
    @[model] struct RangeInclusive {Idx} where
      start : Idx
      end : Idx
      exhausted : Bool

    impl RangeInclusive where
      spec new where
        ensures result.start == _1 && result.end == _2 && !result.exhausted
