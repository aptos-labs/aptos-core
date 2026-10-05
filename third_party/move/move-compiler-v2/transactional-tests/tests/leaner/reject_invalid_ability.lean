-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerRejectInvalidAbility where
  struct Resource has Key where
    value : u64

  struct InvalidCopy has Copy where
    resource : Resource
