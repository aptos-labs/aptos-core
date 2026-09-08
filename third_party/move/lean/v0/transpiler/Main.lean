-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Transpiler

/-!
`lake exe transpile`: transpiles Move source into Leaner Move.

```
transpile <package-dir> -o <out-dir>             a Move package (aptos move exchange --format ast)
transpile --include-deps <package-dir> -o <out>  … with its dependency modules
transpile --move-file <a.move> [<b.move> …] -o <out-dir>
                                                 self-contained module files
transpile --xast <json-dir> -o <out-dir>          an existing XAST export
```

`--verify` (before the input) appends `verify f` to every generated `spec f`,
so building the output attempts the automatic proofs.  `--lean-root <prefix>`
(before the input) is the Lean module prefix of the output directory, used by
the generated `import`s of transpiled dependencies.

The generated `.lean` files go under `<out-dir>` (one per module, under the
module's Lean path) together with `transpile-report.txt`.
-/

open Transpiler Transpiler.Driver Transpiler.Cli

def usage : String :=
  "usage: transpile [options] <package-dir> -o <out-dir>\n" ++
  "       transpile [options] --include-deps <package-dir> -o <out-dir>\n" ++
  "       transpile [options] --move-file <a.move> [<b.move> ...] -o <out-dir>\n" ++
  "       transpile [options] --xast <json-dir> -o <out-dir>\n" ++
  "options: --verify                 append `verify f` to every `spec f`\n" ++
  "         --lean-root <prefix>     Lean module prefix of <out-dir> for generated imports"

def splitOutput (args : List String) : Option (List String × String) :=
  match args.reverse with
  | out :: "-o" :: rest => some (rest.reverse, out)
  | _ => none

def main (args : List String) : IO UInt32 := do
  let some (inputs, output) := splitOutput args
    | IO.eprintln usage; pure 2
  -- Options precede the input.
  let rec options (inputs : List String) (emitVerify : Bool) (leanRoot : String) :
      Bool × String × List String :=
    match inputs with
    | "--verify" :: rest => options rest true leanRoot
    | "--lean-root" :: root :: rest => options rest emitVerify root
    | _ => (emitVerify, leanRoot, inputs)
  let (emitVerify, leanRoot, inputs) := options inputs false ""
  let pkg? : Option (IO Effects.Package) := match inputs with
    | ["--xast", dir] => some (readXastDir dir)
    | ["--include-deps", dir] => some (exportPackage dir (includeDeps := true))
    | "--move-file" :: files@(_ :: _) => some (exportMoveFiles (files.map System.FilePath.mk))
    | [dir] => some (exportPackage dir)
    | _ => none
  let some pkgIO := pkg?
    | IO.eprintln usage; pure 2
  let pkg ← pkgIO
  let results := transpilePackage pkg (emitVerify := emitVerify) (leanRoot := leanRoot)
  writeOutputs output results
  IO.print (renderReport (results.map (·.1)))
  let failed := results.any fun (o, _) => o.error.isSome
  pure (if failed then 1 else 0)
