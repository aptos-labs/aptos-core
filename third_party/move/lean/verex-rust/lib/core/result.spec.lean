-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

mod result where
  impl Result where
    spec unwrap where
      panics_if _1 is Err
      ensures result == core.data.selectVariants[Result<T, E>, Ok.0](_1)

    spec unwrap_or where
      ensures _1 is Ok ==> result == core.data.selectVariants[Result<T, E>, Ok.0](_1)
      ensures _1 is Err ==> result == _2

    -- A panic formats the error, which a `Debug` impl may fail at: a panic either way.
    spec expect where
      panics_if _1 is Err
      ensures result == core.data.selectVariants[Result<T, E>, Ok.0](_1)

    spec ok where
      ensures (result is Some) == (_1 is Ok)
      ensures _1 is Ok ==> result.0 == core.data.selectVariants[Result<T, E>, Ok.0](_1)

    spec err where
      ensures (result is Some) == (_1 is Err)
      ensures _1 is Err ==> result.0 == core.data.selectVariants[Result<T, E>, Err.0](_1)

    spec is_ok where
      ensures result == (_1 is Ok)

    spec is_err where
      ensures result == (_1 is Err)

  trait impl «<std::result::Result<T, E> as std::ops::Try>» where
    spec branch at {T} {E} {Result<convert::Infallible, E>} {T} where
      ensures _1 is Ok ==> result is Continue
      ensures _1 is Ok ==> core.data.selectVariants[ops::control_flow::ControlFlow<Result<convert::Infallible, E>, T>, Continue.0](result) == core.data.selectVariants[Result<T, E>, Ok.0](_1)
      ensures _1 is Err ==> result is Break
      ensures _1 is Err ==> core.data.selectVariants[Result<convert::Infallible, E>, Err.0](core.data.selectVariants[ops::control_flow::ControlFlow<Result<convert::Infallible, E>, T>, Break.0](result)) == core.data.selectVariants[Result<T, E>, Err.0](_1)
      ensures _1 is Err ==> core.data.selectVariants[ops::control_flow::ControlFlow<Result<convert::Infallible, E>, T>, Break.0](result) is Err

  -- At an unchanged error type `From::from` is the identity of `impl From<T> for T`.
  -- A conversion to another type has no contract here.
  trait impl «<std::result::Result<T, F> as std::ops::FromResidual<std::result::Result<std::convert::Infallible, E>>>» where
    spec from_residual at {T} {E} {E} where
      ensures result is Err
      ensures core.data.selectVariants[Result<T, E>, Err.0](result) == core.data.selectVariants[Result<convert::Infallible, E>, Err.0](_1)
