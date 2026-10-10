-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerRejectRecursiveGenericType where
  struct Left {T} has Copy, Drop, Store where
    right : Right<T>

  struct Right {T} has Copy, Drop, Store where
    left : Left<T>
