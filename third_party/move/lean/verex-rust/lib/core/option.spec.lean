-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

mod option where
  impl Option where
    spec unwrap where
      panics_if _1 is None
      ensures result == _1.0

    spec unwrap_or where
      ensures _1 is Some ==> result == _1.0
      ensures _1 is None ==> result == _2

    spec expect where
      panics_if _1 is None
      ensures result == _1.0

    spec as_ref where
      ensures (result is Some) == (_1 is Some)
      ensures _1 is Some ==> result.0 == _1.0

    spec map where
      ensures (result is Some) == (_1 is Some)
      ensures _1 is Some ==> ensures_of<__trait_FnOnce_call_once>(_2, _1.0, result.0)

    spec ok_or where
      ensures (result is Ok) == (_1 is Some)
      ensures _1 is Some ==> core.data.selectVariants[result::Result<T, E>, Ok.0](result) == _1.0
      ensures _1 is None ==> core.data.selectVariants[result::Result<T, E>, Err.0](result) == _2

    spec take where
      ensures result == old(_1)
      ensures _1 is None

    spec replace where
      ensures result == old(_1)
      ensures _1 is Some && _1.0 == _2

    spec unwrap_or_default at {u8} where
      ensures _1 is Some ==> result == _1.0
      ensures _1 is None ==> result == 0

    spec unwrap_or_default at {u16} where
      ensures _1 is Some ==> result == _1.0
      ensures _1 is None ==> result == 0

    spec unwrap_or_default at {u32} where
      ensures _1 is Some ==> result == _1.0
      ensures _1 is None ==> result == 0

    spec unwrap_or_default at {u64} where
      ensures _1 is Some ==> result == _1.0
      ensures _1 is None ==> result == 0

    spec unwrap_or_default at {u128} where
      ensures _1 is Some ==> result == _1.0
      ensures _1 is None ==> result == 0

    spec unwrap_or_default at {usize} where
      ensures _1 is Some ==> result == _1.0
      ensures _1 is None ==> result == 0

    spec unwrap_or_default at {i8} where
      ensures _1 is Some ==> result == _1.0
      ensures _1 is None ==> result == 0

    spec unwrap_or_default at {i16} where
      ensures _1 is Some ==> result == _1.0
      ensures _1 is None ==> result == 0

    spec unwrap_or_default at {i32} where
      ensures _1 is Some ==> result == _1.0
      ensures _1 is None ==> result == 0

    spec unwrap_or_default at {i64} where
      ensures _1 is Some ==> result == _1.0
      ensures _1 is None ==> result == 0

    spec unwrap_or_default at {i128} where
      ensures _1 is Some ==> result == _1.0
      ensures _1 is None ==> result == 0

    spec unwrap_or_default at {isize} where
      ensures _1 is Some ==> result == _1.0
      ensures _1 is None ==> result == 0

    spec unwrap_or_default at {Bool} where
      ensures _1 is Some ==> result == _1.0
      ensures _1 is None ==> result == false

    spec copied where
      ensures (result is Some) == (_1 is Some)
      ensures _1 is Some ==> result.0 == _1.0

    spec cloned at {u8} where
      ensures (result is Some) == (_1 is Some)
      ensures _1 is Some ==> result.0 == _1.0

    spec cloned at {u16} where
      ensures (result is Some) == (_1 is Some)
      ensures _1 is Some ==> result.0 == _1.0

    spec cloned at {u32} where
      ensures (result is Some) == (_1 is Some)
      ensures _1 is Some ==> result.0 == _1.0

    spec cloned at {u64} where
      ensures (result is Some) == (_1 is Some)
      ensures _1 is Some ==> result.0 == _1.0

    spec cloned at {u128} where
      ensures (result is Some) == (_1 is Some)
      ensures _1 is Some ==> result.0 == _1.0

    spec cloned at {usize} where
      ensures (result is Some) == (_1 is Some)
      ensures _1 is Some ==> result.0 == _1.0

    spec cloned at {i8} where
      ensures (result is Some) == (_1 is Some)
      ensures _1 is Some ==> result.0 == _1.0

    spec cloned at {i16} where
      ensures (result is Some) == (_1 is Some)
      ensures _1 is Some ==> result.0 == _1.0

    spec cloned at {i32} where
      ensures (result is Some) == (_1 is Some)
      ensures _1 is Some ==> result.0 == _1.0

    spec cloned at {i64} where
      ensures (result is Some) == (_1 is Some)
      ensures _1 is Some ==> result.0 == _1.0

    spec cloned at {i128} where
      ensures (result is Some) == (_1 is Some)
      ensures _1 is Some ==> result.0 == _1.0

    spec cloned at {isize} where
      ensures (result is Some) == (_1 is Some)
      ensures _1 is Some ==> result.0 == _1.0

    spec cloned at {Bool} where
      ensures (result is Some) == (_1 is Some)
      ensures _1 is Some ==> result.0 == _1.0

    spec cloned at {Char} where
      ensures (result is Some) == (_1 is Some)
      ensures _1 is Some ==> result.0 == _1.0

    spec is_some where
      ensures result == (_1 is Some)

    spec is_none where
      ensures result == (_1 is None)

  -- `Clone` is the identity at a type without heap state.
  trait impl «<std::option::Option<T> as std::clone::Clone>» where
    spec clone at {u8} where
      ensures result == _1

    spec clone at {u16} where
      ensures result == _1

    spec clone at {u32} where
      ensures result == _1

    spec clone at {u64} where
      ensures result == _1

    spec clone at {u128} where
      ensures result == _1

    spec clone at {usize} where
      ensures result == _1

    spec clone at {i8} where
      ensures result == _1

    spec clone at {i16} where
      ensures result == _1

    spec clone at {i32} where
      ensures result == _1

    spec clone at {i64} where
      ensures result == _1

    spec clone at {i128} where
      ensures result == _1

    spec clone at {isize} where
      ensures result == _1

    spec clone at {Bool} where
      ensures result == _1

    spec clone at {Char} where
      ensures result == _1

  trait impl «<std::option::Option<T> as std::ops::Try>» where
    spec branch at {T} {Option<convert::Infallible>} {T} where
      ensures _1 is Some ==> result is Continue
      ensures _1 is Some ==> core.data.selectVariants[ops::control_flow::ControlFlow<Option<convert::Infallible>, T>, Continue.0](result) == _1.0
      ensures _1 is None ==> result is Break
      ensures _1 is None ==> core.data.selectVariants[ops::control_flow::ControlFlow<Option<convert::Infallible>, T>, Break.0](result) is None

  -- `Infallible` has no value, so the residual is `None`.
  trait impl «<std::option::Option<T> as std::ops::FromResidual<std::option::Option<std::convert::Infallible>>>» where
    spec from_residual where
      ensures result is None
