-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerBorrowRejectNestedPoison where
  struct Inner has Copy, Drop, Store where
    value : u64

  struct Outer has Copy, Drop, Store where
    inner : Inner

  fun run() -> u64 := do
    let mut owner := new Outer { inner := new Inner { value := 0 } }
    let outer := &mut owner
    let inner := &mut outer.inner
    let field := &mut inner.value
    let whole := &mut owner
    *whole := new Outer { inner := new Inner { value := 1 } }
    let result := *field
    result
