-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace core where
  pragma verify = false

  mod iter where
    mod adapters where
      mod peekable where
        @[destructor]
        opaque struct Peekable {I} where

        impl Peekable where
          @[shape_only]
          native fun peek {I} {BorrowedItem}(
            _1 : &mut Peekable<I>
          ) -> option::Option<BorrowedItem>

      mod zip where
        @[destructor]
        opaque struct Zip {A} {B} where

        @[shape_only]
        native fun zip {A} {B} {IntoIter} {IntoIter2}(
          _1 : A, _2 : B
        ) -> Zip<IntoIter, IntoIter2>

  mod num where
    mod error where
      opaque struct TryFromIntError has Copy where

  mod ops where
    mod range where
      struct «Range» {Idx} where
        start : Idx
        end : Idx

  mod option where
    enum Option {T} has Copy where
      | None = 0
      | Some (0 : T) = 1

    impl Option where
      @[shape_only]
      native fun as_deref {T} {BorrowedTarget}(
        _1 : &Option<T>
      ) -> Option<BorrowedTarget>

  mod result where
    enum Result {T} {E} has Copy where
      | Ok (0 : T) = 0
      | Err (0 : E) = 1

  mod convert where
    mod num where
      trait impl «std::convert::num::<impl std::convert::TryFrom<u32> for u8>» where
        @[shape_only]
        native fun try_from {Error}(_1 : u32) -> result::Result<u8, Error>

verex namespace associated_aliases where
  use core::convert::num::«std::convert::num::<impl std::convert::TryFrom<u32> for u8>»::try_from
  use core::iter::adapters::peekable::Peekable
  use core::iter::adapters::peekable::Peekable::peek
  use core::iter::adapters::zip::Zip
  use core::iter::adapters::zip::zip
  use core::num::error::TryFromIntError
  use core::ops::range::«Range»
  use core::option::Option
  use core::option::Option::as_deref
  use core::result::Result

  -- Two associated types of one name, normalizing to the same type.
  fun zipped(n : u32) -> Zip<«Range»<u32>, «Range»<u32> > := do
    let _2 := new «Range»<u32> { start := 0u32, end := n }
    let _3 := new «Range»<u32> { start := 0u32, end := n }
    zip::<«Range»<u32>, «Range»<u32>, «Range»<u32>, «Range»<u32> >(_2, _3)

  -- An associated type on concrete types, the impl having no parameter.
  fun narrow(value : u32) -> Result<u8, TryFromIntError> :=
    try_from::<TryFromIntError>(value)

  -- Nested under `Option` and a reference.
  fun peeked(values : &mut Peekable<«Range»<u32> >) -> Option<&u32> :=
    peek::<«Range»<u32>, &u32>(values)

  -- An associated type at an unsized type.
  fun borrowed_contents(
    value : &Option<Vector<u32> >
  ) -> Option<&Slice<u32> > :=
    as_deref::<Vector<u32>, &Slice<u32> >(value)
