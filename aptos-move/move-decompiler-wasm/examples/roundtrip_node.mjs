// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Roundtrip check: Move source -> compile -> v10 bytecode -> decompile -> source.
//
// Prereq: wasm-pack build --release --target nodejs --out-dir pkg-node
// Run:    node examples/roundtrip_node.mjs

import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const crate = join(here, "..");
const wasm = await import(join(crate, "pkg-node", "move_decompiler_wasm.js"));
const {
  compile_module,
  decompile_module,
  disassemble_module,
  get_module_metadata,
  verify_module,
} = wasm;

const NAMED = JSON.stringify({ example: "0x42" });

// A non-trivial module: a struct, a constant, arithmetic, a loop, branching,
// a public and a private function, and a function call.
const SOURCE = `
module example::math {
    const OVERFLOW: u64 = 1;

    struct Point has copy, drop, store {
        x: u64,
        y: u64,
    }

    public fun new_point(x: u64, y: u64): Point {
        Point { x, y }
    }

    public fun manhattan(p: &Point): u64 {
        p.x + p.y
    }

    public fun sum_to(n: u64): u64 {
        let i = 0;
        let acc = 0;
        while (i <= n) {
            acc = acc + i;
            i = i + 1;
        };
        acc
    }

    public fun classify(n: u64): u64 {
        if (n == 0) {
            0
        } else if (n < 10) {
            1
        } else {
            2
        }
    }

    fun helper(a: u64, b: u64): u64 {
        a * b + OVERFLOW
    }

    public fun compute(p: &Point): u64 {
        helper(manhattan(p), classify(p.x))
    }
}
`;

function banner(title) {
  console.log("\n" + "=".repeat(70) + "\n" + title + "\n" + "=".repeat(70));
}

banner("ORIGINAL SOURCE");
console.log(SOURCE.trim());

const bytecode = compile_module(SOURCE, NAMED);
if (!verify_module(bytecode)) throw new Error("compiled bytecode failed verification");
const meta = get_module_metadata(bytecode);
banner("COMPILED (bytecode v10)");
console.log(
  `address=${meta.address} name=${meta.name} version=${meta.version} ` +
    `functions=${meta.functionCount} structs=${meta.structCount} bytes=${bytecode.length}`,
);
if (meta.version !== 10) throw new Error(`expected v10, got ${meta.version}`);

banner("DECOMPILED SOURCE (from bytecode)");
const decompiled = decompile_module(bytecode);
console.log(decompiled);

banner("DISASSEMBLY (first 30 lines)");
console.log(disassemble_module(bytecode).split("\n").slice(0, 30).join("\n"));

// Idempotency-ish check: try to recompile the decompiled source and confirm it
// still produces a verifiable v10 module (the decompiler emits a v2-compatible
// dialect; this checks the decompiled output is at least well-formed enough to
// round back through the compiler).
banner("RE-COMPILE DECOMPILED SOURCE");
try {
  const bytecode2 = compile_module(decompiled, NAMED);
  const meta2 = get_module_metadata(bytecode2);
  console.log(
    `re-compiled OK: version=${meta2.version} functions=${meta2.functionCount} ` +
      `structs=${meta2.structCount} bytes=${bytecode2.length}`,
  );
  console.log(
    "byte-identical to first compile:",
    Buffer.compare(Buffer.from(bytecode), Buffer.from(bytecode2)) === 0,
  );
} catch (e) {
  console.log("re-compile of decompiled source FAILED:\n" + String(e));
}
