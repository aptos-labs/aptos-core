-- Copyright © Aptos Foundation

--# publish

import VerexMove

verex module 0x0::VerexRejectRecursiveEnum where
  enum Chain has Copy, Drop, Store where
    | End
    | Link (next : Chain)
