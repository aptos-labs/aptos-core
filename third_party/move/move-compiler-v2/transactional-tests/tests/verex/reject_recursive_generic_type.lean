-- Copyright © Aptos Foundation

--# publish

import VerexMove

verex module 0x0::VerexRejectRecursiveGenericType where
  struct Left {T} has Copy, Drop, Store where
    right : Right<T>

  struct Right {T} has Copy, Drop, Store where
    left : Left<T>
