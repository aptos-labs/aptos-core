-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerAbilities where
  struct Plain where
    value : u64

  struct CopyDrop has Copy, Drop where
    value : u64

  struct Stored {T has Store} has Store where
    value : T

  struct Resource has Key where
    value : u64

  enum Droppable has Drop where
    | Empty
    | Value (inner : u64)
