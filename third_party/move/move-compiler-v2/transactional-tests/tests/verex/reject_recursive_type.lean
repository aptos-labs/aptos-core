-- Copyright © Aptos Foundation

--# publish

import VerexMove

verex module 0x0::VerexRejectRecursiveType where
  struct RecursiveType has Copy, Drop, Store where
    next : RecursiveType
