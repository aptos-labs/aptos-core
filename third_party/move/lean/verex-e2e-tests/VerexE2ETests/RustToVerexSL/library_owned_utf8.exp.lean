-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace core where
  pragma verify = false

  mod result where
    enum Result {T} {E} has Copy where
      | Ok (0 : T) = 0
      | Err (0 : E) = 1

verex namespace alloc where
  use core::result::Result

  pragma verify = false

  mod alloc where
    opaque struct Global has Copy where

  mod «string» where
    struct FromUtf8Error where
      bytes : Vector<u8>

    impl String where
      native fun from_utf8(_1 : Vector<u8>) -> Result<string, FromUtf8Error>

      native fun into_bytes(_1 : string) -> Vector<u8>

      native fun len(_1 : &string) -> usize

  mod vec where
    impl Vec where
      native fun len {T} {A}(_1 : &Vector<T>) -> usize

verex namespace library_owned_utf8 where
  use alloc::alloc::Global
  use alloc::«string»::FromUtf8Error
  use alloc::«string»::String::from_utf8
  use alloc::«string»::String::into_bytes
  use core::result::Result

  fun roundtrip(text : string) -> Result<string, FromUtf8Error> :=
    from_utf8(into_bytes(text))

  fun decode(bytes : Vector<u8>) -> Result<string, FromUtf8Error> :=
    from_utf8(bytes)

  fun success_bytes(
    bytes : Vector<u8>
  ) -> Result<Vector<u8>, FromUtf8Error> := do
    let _2 := from_utf8(bytes)
    let mut _0 : Result<Vector<u8>, FromUtf8Error>
    match core.readDiscriminant[isize](_2) with
      | 0 => do
        let text := move((downcast _2 as Result::Ok).0)
        _0 := new Result<Vector<u8>, FromUtf8Error>::Ok {
          0 := into_bytes(text)
        }
      | 1 => do
        let error := move((downcast _2 as Result::Err).0)
        _0 := new Result<Vector<u8>, FromUtf8Error>::Err { 0 := error }
    _0

  fun success_length(bytes : Vector<u8>) -> Result<Bool, FromUtf8Error> := do
    let length := alloc::vec::Vec::len::<u8, Global>(&bytes)
    let _4 := from_utf8(bytes)
    let mut _0 : Result<Bool, FromUtf8Error>
    match core.readDiscriminant[isize](_4) with
      | 0 => do
        let text := move((downcast _4 as Result::Ok).0)
        _0 := new Result<Bool, FromUtf8Error>::Ok {
          0 := alloc::«string»::String::len(&text) == length
        }
        drop(text)
      | 1 => do
        let error := move((downcast _4 as Result::Err).0)
        _0 := new Result<Bool, FromUtf8Error>::Err { 0 := error }
    _0
