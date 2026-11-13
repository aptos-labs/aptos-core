-- Copyright © Aptos Foundation

--# publish

import VerexMove

verex module 0x0::VerexRejectInvalidAbility where
  struct Resource has Key where
    value : u64

  struct InvalidCopy has Copy where
    resource : Resource
