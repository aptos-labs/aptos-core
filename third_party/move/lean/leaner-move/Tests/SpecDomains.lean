-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove.Frontend.SpecDomains

open LeanerMove.Frontend
open SpecDomains

private def loc : Xast.Loc := ⟨0, 0, 0⟩

private def body : Xast.Exp := .mk .num loc (.«local» "x")

-- The bounds of each fixed-width integer type.
#guard range? .u8 == some (0, 255)
#guard range? .u64 == some (0, 2 ^ 64 - 1)
#guard range? .i8 == some (-128, 127)
#guard range? .i256 == some (-(2 ^ 255), 2 ^ 255 - 1)
#guard range? .num == none
#guard range? (.typeParam 0) == none

-- A fixed-width parameter guards the body; outside, the value is an aborting
-- branch of the result type.
#guard match partialBody [{ name := "x", ty := .u8 }, { name := "n", ty := .num }] body with
  | .mk .num _ (.ite
      (.mk .bool _ (.call .and []
        [.mk .bool _ (.call .le [] [.mk .num _ (.value (.number 0) none false),
            .mk .u8 _ (.«local» "x")] none),
         .mk .bool _ (.call .le [] [.mk .u8 _ (.«local» "x"),
            .mk .num _ (.value (.number 255) none false)] none)] none))
      (.mk .num _ (.«local» "x"))
      (.mk .num _ (.call (.abort .code) [] [_] none))) => true
  | _ => false

-- Two fixed-width parameters conjoin their ranges.
#guard match partialBody [{ name := "x", ty := .u8 }, { name := "y", ty := .i16 }] body with
  | .mk _ _ (.ite (.mk .bool _ (.call .and [] [_, .mk .bool _ (.call .and [] _ none)] none)) _ _) =>
      true
  | _ => false

-- Without a fixed-width parameter the function is total.
#guard match partialBody [{ name := "n", ty := .num }, { name := "v", ty := .vector .u8 }] body with
  | .mk .num _ (.«local» "x") => true
  | _ => false
