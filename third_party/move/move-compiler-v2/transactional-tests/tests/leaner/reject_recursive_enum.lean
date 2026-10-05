-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerRejectRecursiveEnum where
  enum Chain has Copy, Drop, Store where
    | End
    | Link (next : Chain)
