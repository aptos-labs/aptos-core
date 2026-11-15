-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace core where
  pragma verify = false

  mod option where
    enum Option {T} has Copy where
      | None = 0
      | Some (0 : T) = 1

  mod result where
    enum Result {T} {E} has Copy where
      | Ok (0 : T) = 0
      | Err (0 : E) = 1

    impl Result where
      native fun is_ok {T} {E}(_1 : &Result<T, E>) -> Bool

verex namespace alloc where
  use core::result::Result

  pragma verify = false

  mod alloc where
    opaque struct Global has Copy where

  mod «string» where
    struct FromUtf8Error where
      bytes : Vector<u8>

    impl FromUtf8Error where
      native fun as_bytes(_1 : &FromUtf8Error) -> &Slice<u8>

      native fun into_bytes(_1 : FromUtf8Error) -> Vector<u8>

    impl String where
      native fun from_utf8(_1 : Vector<u8>) -> Result<string, FromUtf8Error>

      native fun into_bytes(_1 : string) -> Vector<u8>

  mod vec where
    impl Vec where
      native fun len {T} {A}(_1 : &Vector<T>) -> usize

verex namespace library_utf8_error where
  use alloc::alloc::Global
  use alloc::vec::Vec::len
  use alloc::«string»::FromUtf8Error
  use alloc::«string»::FromUtf8Error::as_bytes
  use alloc::«string»::String::from_utf8
  use core::option::Option
  use core::result::Result
  use core::result::Result::is_ok

  fun recover(bytes : Vector<u8>) -> Vector<u8> := do
    let _2 := from_utf8(bytes)
    let mut _0 : Vector<u8>
    match core.readDiscriminant[isize](_2) with
      | 0 => do
        let text := move((downcast _2 as Result::Ok).0)
        _0 := alloc::«string»::String::into_bytes(text)
      | 1 => do
        let error := move((downcast _2 as Result::Err).0)
        _0 := alloc::«string»::FromUtf8Error::into_bytes(error)
    _0

  fun error_length(bytes : Vector<u8>) -> Bool := do
    let length := len::<u8, Global>(&bytes)
    let _4 := from_utf8(bytes)
    let mut _0 : Bool
    match core.readDiscriminant[isize](_4) with
      | 0 => _0 := true
      | 1 => do
        let error := move((downcast _4 as Result::Err).0)
        _0 := as_bytes(&error).length == length
        drop(error)
    match core.readDiscriminant[isize](_4) with
      | 1 => ()
      | 0 => drop(_4)
    _0

  fun roundtrip(text : string) -> Bool := do
    let _3 := from_utf8(alloc::«string»::String::into_bytes(text))
    let _0 := is_ok(&_3)
    drop(_3)
    _0

  fun discard_error(bytes : Vector<u8>) -> Option<Vector<u8> > := do
    let _2 := from_utf8(bytes)
    let mut _0 : Option<Vector<u8> >
    match core.readDiscriminant[isize](_2) with
      | 0 => do
        let text := move((downcast _2 as Result::Ok).0)
        _0 := new Option<Vector<u8> >::Some {
          0 := alloc::«string»::String::into_bytes(text)
        }
      | 1 => _0 := new Option<Vector<u8> >::None {}
    match core.readDiscriminant[isize](_2) with
      | 0 => ()
      | 1 => drop(_2)
    _0
