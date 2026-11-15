-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

mod cmp where
  trait Ord where
    spec max at {u8} where
      ensures result == (if _1 > _2 then _1 else _2)

    spec min at {u8} where
      ensures result == (if _1 < _2 then _1 else _2)

    spec max at {u16} where
      ensures result == (if _1 > _2 then _1 else _2)

    spec min at {u16} where
      ensures result == (if _1 < _2 then _1 else _2)

    spec max at {u32} where
      ensures result == (if _1 > _2 then _1 else _2)

    spec min at {u32} where
      ensures result == (if _1 < _2 then _1 else _2)

    spec max at {u64} where
      ensures result == (if _1 > _2 then _1 else _2)

    spec min at {u64} where
      ensures result == (if _1 < _2 then _1 else _2)

    spec max at {u128} where
      ensures result == (if _1 > _2 then _1 else _2)

    spec min at {u128} where
      ensures result == (if _1 < _2 then _1 else _2)

    spec max at {usize} where
      ensures result == (if _1 > _2 then _1 else _2)

    spec min at {usize} where
      ensures result == (if _1 < _2 then _1 else _2)

    spec max at {i8} where
      ensures result == (if _1 > _2 then _1 else _2)

    spec min at {i8} where
      ensures result == (if _1 < _2 then _1 else _2)

    spec max at {i16} where
      ensures result == (if _1 > _2 then _1 else _2)

    spec min at {i16} where
      ensures result == (if _1 < _2 then _1 else _2)

    spec max at {i32} where
      ensures result == (if _1 > _2 then _1 else _2)

    spec min at {i32} where
      ensures result == (if _1 < _2 then _1 else _2)

    spec max at {i64} where
      ensures result == (if _1 > _2 then _1 else _2)

    spec min at {i64} where
      ensures result == (if _1 < _2 then _1 else _2)

    spec max at {i128} where
      ensures result == (if _1 > _2 then _1 else _2)

    spec min at {i128} where
      ensures result == (if _1 < _2 then _1 else _2)

    spec max at {isize} where
      ensures result == (if _1 > _2 then _1 else _2)

    spec min at {isize} where
      ensures result == (if _1 < _2 then _1 else _2)

  mod impls where
    trait impl «std::cmp::impls::<impl std::cmp::Ord for u8>» where
      spec cmp where
        ensures (result is Less) == (_1 < _2)
        ensures (result is Equal) == (_1 == _2)
        ensures (result is Greater) == (_1 > _2)

    trait impl «std::cmp::impls::<impl std::cmp::Ord for u16>» where
      spec cmp where
        ensures (result is Less) == (_1 < _2)
        ensures (result is Equal) == (_1 == _2)
        ensures (result is Greater) == (_1 > _2)

    trait impl «std::cmp::impls::<impl std::cmp::Ord for u32>» where
      spec cmp where
        ensures (result is Less) == (_1 < _2)
        ensures (result is Equal) == (_1 == _2)
        ensures (result is Greater) == (_1 > _2)

    trait impl «std::cmp::impls::<impl std::cmp::Ord for u64>» where
      spec cmp where
        ensures (result is Less) == (_1 < _2)
        ensures (result is Equal) == (_1 == _2)
        ensures (result is Greater) == (_1 > _2)

    trait impl «std::cmp::impls::<impl std::cmp::Ord for u128>» where
      spec cmp where
        ensures (result is Less) == (_1 < _2)
        ensures (result is Equal) == (_1 == _2)
        ensures (result is Greater) == (_1 > _2)

    trait impl «std::cmp::impls::<impl std::cmp::Ord for usize>» where
      spec cmp where
        ensures (result is Less) == (_1 < _2)
        ensures (result is Equal) == (_1 == _2)
        ensures (result is Greater) == (_1 > _2)

    trait impl «std::cmp::impls::<impl std::cmp::Ord for i8>» where
      spec cmp where
        ensures (result is Less) == (_1 < _2)
        ensures (result is Equal) == (_1 == _2)
        ensures (result is Greater) == (_1 > _2)

    trait impl «std::cmp::impls::<impl std::cmp::Ord for i16>» where
      spec cmp where
        ensures (result is Less) == (_1 < _2)
        ensures (result is Equal) == (_1 == _2)
        ensures (result is Greater) == (_1 > _2)

    trait impl «std::cmp::impls::<impl std::cmp::Ord for i32>» where
      spec cmp where
        ensures (result is Less) == (_1 < _2)
        ensures (result is Equal) == (_1 == _2)
        ensures (result is Greater) == (_1 > _2)

    trait impl «std::cmp::impls::<impl std::cmp::Ord for i64>» where
      spec cmp where
        ensures (result is Less) == (_1 < _2)
        ensures (result is Equal) == (_1 == _2)
        ensures (result is Greater) == (_1 > _2)

    trait impl «std::cmp::impls::<impl std::cmp::Ord for i128>» where
      spec cmp where
        ensures (result is Less) == (_1 < _2)
        ensures (result is Equal) == (_1 == _2)
        ensures (result is Greater) == (_1 > _2)

    trait impl «std::cmp::impls::<impl std::cmp::Ord for isize>» where
      spec cmp where
        ensures (result is Less) == (_1 < _2)
        ensures (result is Equal) == (_1 == _2)
        ensures (result is Greater) == (_1 > _2)
