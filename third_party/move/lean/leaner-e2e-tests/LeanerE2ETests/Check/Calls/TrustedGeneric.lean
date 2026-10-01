-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Trusted generic callees with enum results

A callee of a module that sets `pragma verify = false` is assumed through
its contract, at every family and instantiation. Its result reaches the
caller through the transport from the instantiated family to the caller's
own carriers, which the normalizer reduces constructor by constructor: the
variant a `none`/`some` built and the field read from it are what the
callee's contract states.
-/

namespace LeanerLang.Tests.Check.Calls.TrustedGeneric

leaner module 0x47::maybe where
  pragma verify = false

  enum Maybe {T} has Copy, Drop, Store where
    | None
    | Some (e : T)

  public fun none {T}() -> Maybe<T> := new Maybe<T>::None {}
  spec none where
    pragma opaque
    aborts_if false
    ensures result == spec_none::<T>()

  public fun some {T}(e : T) -> Maybe<T> := new Maybe<T>::Some { e }
  spec some where
    pragma opaque
    aborts_if false
    ensures result == spec_some(e)

  public fun is_some {T}(self : &Maybe<T>) -> Bool := self is Some
  spec is_some where
    pragma opaque
    aborts_if false
    ensures result == spec_is_some(self)

  public fun is_none {T}(self : &Maybe<T>) -> Bool := self is None
  spec is_none where
    pragma opaque
    aborts_if false
    ensures result == !spec_is_some(self)

  public fun borrow {T}(self : &Maybe<T>) -> &T :=
    if self is None then abort(1) else &self.e
  spec borrow where
    pragma opaque
    aborts_if !spec_is_some(self)
    ensures result == spec_borrow(self)

  spec fun spec_none {T}() : Maybe<T> := new Maybe<T>::None {}
  spec fun spec_some {T}(e : T) : Maybe<T> := new Maybe<T>::Some { e }
  spec fun spec_is_some {T}(self : Maybe<T>) : Bool := self is Some
  spec fun spec_borrow {T}(self : Maybe<T>) : T :=
    if self is Some then self.e else abort()

leaner module 0x47::pick where
  use 0x47::maybe::Maybe
  use 0x47::maybe::some
  use 0x47::maybe::none
  use 0x47::maybe::is_none
  use 0x47::maybe::is_some
  use 0x47::maybe::borrow

  struct Scalar has Copy, Drop, Store where
    data : Vector<u8>

  public fun pick(bytes : Vector<u8>) -> Maybe<Scalar> :=
    if bytes.length == 32 then some(new Scalar { data := bytes }) else none::<Scalar>()
  spec pick where
    aborts_if false
    ensures bytes.length == 32 ==> is_some(result) && borrow(result).data == bytes
    ensures bytes.length != 32 ==> is_none(result)

end LeanerLang.Tests.Check.Calls.TrustedGeneric
