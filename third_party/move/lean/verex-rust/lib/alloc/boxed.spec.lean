-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

mod boxed where
  @[model(T)] opaque struct Box {T} {A} where

  impl Box where
    spec new where
      ensures result == _1

  trait impl «<std::boxed::Box<T, A> as std::convert::AsRef<T>>» where
    spec as_ref where
      ensures result == _1

  trait impl «<std::boxed::Box<T, A> as std::convert::AsMut<T>>» where
    spec as_mut where
      ensures result == old(_1)
      ensures _1 == final(result)
