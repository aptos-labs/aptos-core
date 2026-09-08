-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Transpiler.Xast

/-!
# Comment attachment

Ordinary comments travel in the XAST `comments` table with their spans and
whether they stand on a line of their own.  The printer re-attaches them by
span, with the policy source formatters use: a comment on its own line is a
*leading* comment of the first node whose span starts after it; a comment
after code on the same line is a *trailing* comment of the node whose span
ends before it.

The pool hands comments out exactly once, in source order, so a comment
consumed inside a nested block is not emitted again by the enclosing sequence.
A comment whose anchor does not survive the export — the table is consumed
only for nodes the printer emits — is counted in the report as dropped.
-/

namespace Transpiler.Comments

open Transpiler.Xast

/-- The pool of not-yet-emitted comments of a module, in source order. -/
structure Pool where
  remaining : List Comment
  deriving Inhabited

def Pool.ofModule (m : Module) : Pool :=
  { remaining := m.comments.toArray.qsort (fun a b =>
      a.loc.file < b.loc.file || (a.loc.file == b.loc.file && a.loc.start < b.loc.start)) |>.toList }

/-- Splits the comments of `file` into leading and trailing comments for a
sequence of nodes with the given spans (in source order), bounded by the
enclosing range `[lo, hi)`.  Returns, for each span, its leading comments
(own-line comments between the previous span and this one) and its trailing
comments (not-own-line comments between this span and the next), and the
remaining pool.  Comments inside a span are left in the pool for the node's
own children. -/
def Pool.attach (pool : Pool) (file : Nat) (spans : List (Nat × Nat)) (lo hi : Nat) :
    List (List Comment × List Comment) × Pool :=
  let rec go (spans : List (Nat × Nat)) (prevEnd : Nat) (remaining : List Comment)
      (acc : List (List Comment × List Comment)) : List (List Comment × List Comment) × List Comment :=
    match spans with
    | [] => (acc.reverse, remaining)
    | (start, stop) :: rest =>
      let nextStart := match rest with
        | (s, _) :: _ => s
        | [] => hi
      let inFile (c : Comment) := c.loc.file == file
      -- Leading: own-line comments between prevEnd and start.
      let leading := remaining.filter fun c =>
        inFile c && c.ownLine && c.loc.start ≥ prevEnd && c.loc.start < start
      -- Trailing after the previous node: not-own-line comments before this
      -- start are attributed to the previous node, which was already emitted;
      -- they become leading here as well so they are never lost.
      let displaced := remaining.filter fun c =>
        inFile c && !c.ownLine && c.loc.start ≥ prevEnd && c.loc.start < start
      let trailing := remaining.filter fun c =>
        inFile c && !c.ownLine && c.loc.start ≥ stop && c.loc.start < nextStart &&
          (match rest with | [] => c.loc.start < hi | _ => true)
      let used := leading ++ displaced ++ trailing
      let remaining := remaining.filter fun c => !(used.any fun u => u.loc == c.loc)
      go rest stop remaining ((displaced ++ leading, trailing) :: acc)
  let (result, remaining) := go spans lo pool.remaining []
  (result, { remaining })

/-- Drops the comments of `file` inside `[lo, hi)` (the span of a node that
is not printed, or printed without children), returning them for the report. -/
def Pool.discard (pool : Pool) (file : Nat) (lo hi : Nat) : List Comment × Pool :=
  let (dropped, kept) := pool.remaining.partition fun c =>
    c.loc.file == file && c.loc.start ≥ lo && c.loc.start < hi
  (dropped, { remaining := kept })

/-- Lean spelling of a comment: `// x` → `-- x`, `/* x */` → `/- x -/`. -/
def leanComment (c : Comment) : String :=
  let t := c.text
  if t.startsWith "//" then "--" ++ t.drop 2
  else if t.startsWith "/*" && t.endsWith "*/" then
    "/-" ++ (t.drop 2).dropEnd 2 ++ "-/"
  else "-- " ++ t

end Transpiler.Comments
