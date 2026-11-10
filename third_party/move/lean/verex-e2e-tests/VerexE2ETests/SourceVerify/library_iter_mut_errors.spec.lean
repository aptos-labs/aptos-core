-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- The same loop written with `for` instead of `while let` refuses. `for`
-- desugars through `<I as IntoIterator>::into_iter`, the identity on an
-- iterator, which the library declares as a native. `IterMut` holds a mutable
-- reference, so the type argument is not `Plain` and the native's generic
-- contract does not transfer to it (`doc/designs/denotation.md`, "Instances at
-- a reference"); generalizing skolems to references is on the backburner of
-- `roadmap.md`. `library_iter_mut` verifies the `while let` form, which calls
-- `next` directly.
spec set_all where
  ensures values.length == old(values).length -- error: no reading of `into_iter` there
