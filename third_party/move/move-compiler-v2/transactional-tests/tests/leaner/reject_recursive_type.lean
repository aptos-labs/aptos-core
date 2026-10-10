-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerRejectRecursiveType where
  struct RecursiveType has Copy, Drop, Store where
    next : RecursiveType
